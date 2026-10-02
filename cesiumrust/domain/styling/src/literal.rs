//! styling 语言的颜色与向量字面量求值。
//!
//! 核心入口：`to_byte` / `evaluate_literal_color` / `evaluate_literal_vector`，
//! 负责把颜色与向量字面量 AST 节点求值为运行时的颜色/向量值。
//!
//! CSS 颜色解析（`color("...")` / 命名颜色 / `#rgb` / `#rrggbb` /
//! `rgb()` / `hsl()`）覆盖：`from_css_color_string`、`from_hsl`、`hue2rgb`、
//! `parse_rgb_functional`、`parse_hsl_functional`、`parse_float_js`、
//! `is_css_whitespace`，以及 148 项的命名颜色表。
//!
//! # 偏离（依赖）
//!
//! blueprint 从 `cesium_core` 取 `Color` 和 `Cartesian2/3/4`。这个孤立的
//! domain crate 只依赖 `glam`，所以：
//! * 颜色直接用 `glam::DVec4` 表示（rgba，分量 0..1）；
//!   `evaluate_literal_color` 返回 `Value::Cartesian4`，与 blueprint 在
//!   `Color -> Cartesian4::from_elements` 之后的做法完全一致。
//! * 完整的 CSS 颜色解析器在此重新实现（与 `color.rs` 逐字节一致），
//!   而非导入 blueprint 的 `Color`——后者位于一个只读 crate，本 domain 不得依赖。
//!
//! # CSS 颜色解析覆盖
//!
//! `from_css_color_string` 按“命名色 → 十六进制 → 函数形式”三级回退，
//! 其中函数形式又分 `rgb()/rgba()` 与 `hsl()/hsla()` 两条手写扫描器；
//! 这些扫描器逐字节复现早期 CSS 正则的宽松空白/分隔规则，因此对尾随
//! 逗号、混合百分号、缺 alpha 等边界都保持一致的接受/拒绝。

use glam::{DVec2, DVec3, DVec4};

use crate::ast::Node;
use crate::runtime::ExpressionFeature;
use crate::value::{runtime_error, RuntimeError, Value};

// ---------------------------------------------------------------------------
// JS/CSS 数值辅助函数
// ---------------------------------------------------------------------------

/// 镜像 `Color.fromBytes` 的参数钳制（`CesiumMath.clamp` 到字节）：
/// 钳制到 `[0, 255]`，四舍五入，截断为 `u8`。
///
/// 注意：`evaluate_literal_color` 的 `rgb()`/`rgba()` 路径直接除以 255.0
/// （忠于 blueprint 的 `byteToFloat`，不做取整/钳制），所以本辅助函数
/// 是为需要 `Color.fromBytes` 字节量化的调用方/测试而暴露，
/// 并非在此内部使用。
pub fn to_byte(value: f64) -> u8 {
    // 先钳制到 [0,255]，再四舍五入，最后截断为 u8。
    value.clamp(0.0, 255.0).round() as u8
}

/// 模拟 ECMAScript `parseFloat`：解析最长的前导数字前缀，
/// 若不存在则返回 NaN（永不识别 "inf"/"NaN"）。
fn parse_float_js(s: &str) -> f64 {
    // 从最长前缀向短试探，返回第一个可解析为数字的前缀（都不可则 NaN）。
    for end in (1..=s.len()).rev() {
        let Some(candidate) = s.get(..end) else {
            continue;
        };
        // 只接受由数字与 ./+/-/e/E 组成的前缀，且必须能被 f64 解析。
        if candidate
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        {
            if let Ok(v) = candidate.parse::<f64>() {
                return v;
            }
        }
    }
    // 无任何可解析的数字前缀。
    f64::NAN
}

/// 原始 JS 颜色正则表达式的 `\s` 字符集
/// （ECMAScript WhiteSpace + LineTerminator）。
fn is_css_whitespace(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\x0B'
            | '\x0C'
            | '\u{A0}'
            | '\u{FEFF}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

/// 辅助函数：将色相（hue）转换为 rgb 分量。
fn hue2rgb(m1: f64, m2: f64, mut h: f64) -> f64 {
    // 把色相环回区间 [0,1)。
    if h < 0.0 {
        h += 1.0;
    }
    if h > 1.0 {
        h -= 1.0;
    }
    // 四个扇区分别对应升/平台/降/回落，与标准 HSL->RGB 分段一致。
    if h * 6.0 < 1.0 {
        return m1 + (m2 - m1) * 6.0 * h;
    }
    if h * 2.0 < 1.0 {
        return m2;
    }
    if h * 3.0 < 2.0 {
        return m1 + (m2 - m1) * (2.0 / 3.0 - h) * 6.0;
    }
    m1
}

/// `Color.fromHsl(hue, saturation, lightness, alpha)`（所有输入均为 0..1）。
fn from_hsl(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> DVec4 {
    // 色相取模到 [0,1)；无饱和度时三通道退化为亮度（灰度）。
    let hue = hue % 1.0;
    let mut red = lightness;
    let mut green = lightness;
    let mut blue = lightness;

    // 饱和度非零：由亮度导出 m1/m2，再逐通道经 hue2rgb 求值。
    if saturation != 0.0 {
        let m2 = if lightness < 0.5 {
            lightness * (1.0 + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };
        let m1 = 2.0 * lightness - m2;
        red = hue2rgb(m1, m2, hue + 1.0 / 3.0);
        green = hue2rgb(m1, m2, hue);
        blue = hue2rgb(m1, m2, hue - 1.0 / 3.0);
    }

    DVec4::new(red, green, blue, alpha)
}

/// 148 个 CSS 命名颜色，以 24 位 `(r, g, b)` 三元组表示（键为大写）。
/// 别名（`DARKGREY`、`GREY` 等）归并到其规范条目。
fn named_color_rgb(name: &str) -> Option<(u8, u8, u8)> {
    Some(match name {
        // ---- A 开头 ----
        "ALICEBLUE" => (0xF0, 0xF8, 0xFF),
        "ANTIQUEWHITE" => (0xFA, 0xEB, 0xD7),
        "AQUA" => (0x00, 0xFF, 0xFF),
        "AQUAMARINE" => (0x7F, 0xFF, 0xD4),
        "AZURE" => (0xF0, 0xFF, 0xFF),
        // ---- B 开头 ----
        "BEIGE" => (0xF5, 0xF5, 0xDC),
        "BISQUE" => (0xFF, 0xE4, 0xC4),
        "BLACK" => (0x00, 0x00, 0x00),
        "BLANCHEDALMOND" => (0xFF, 0xEB, 0xCD),
        "BLUE" => (0x00, 0x00, 0xFF),
        "BLUEVIOLET" => (0x8A, 0x2B, 0xE2),
        "BROWN" => (0xA5, 0x2A, 0x2A),
        "BURLYWOOD" => (0xDE, 0xB8, 0x87),
        // ---- C 开头 ----
        "CADETBLUE" => (0x5F, 0x9E, 0xA0),
        "CHARTREUSE" => (0x7F, 0xFF, 0x00),
        "CHOCOLATE" => (0xD2, 0x69, 0x1E),
        "CORAL" => (0xFF, 0x7F, 0x50),
        "CORNFLOWERBLUE" => (0x64, 0x95, 0xED),
        "CORNSILK" => (0xFF, 0xF8, 0xDC),
        "CRIMSON" => (0xDC, 0x14, 0x3C),
        "CYAN" => (0x00, 0xFF, 0xFF),
        // ---- D 开头（含多个 gray/grey 别名）----
        "DARKBLUE" => (0x00, 0x00, 0x8B),
        "DARKCYAN" => (0x00, 0x8B, 0x8B),
        "DARKGOLDENROD" => (0xB8, 0x86, 0x0B),
        "DARKGRAY" | "DARKGREY" => (0xA9, 0xA9, 0xA9),
        "DARKGREEN" => (0x00, 0x64, 0x00),
        "DARKKHAKI" => (0xBD, 0xB7, 0x6B),
        "DARKMAGENTA" => (0x8B, 0x00, 0x8B),
        "DARKOLIVEGREEN" => (0x55, 0x6B, 0x2F),
        "DARKORANGE" => (0xFF, 0x8C, 0x00),
        "DARKORCHID" => (0x99, 0x32, 0xCC),
        "DARKRED" => (0x8B, 0x00, 0x00),
        "DARKSALMON" => (0xE9, 0x96, 0x7A),
        "DARKSEAGREEN" => (0x8F, 0xBC, 0x8F),
        "DARKSLATEBLUE" => (0x48, 0x3D, 0x8B),
        "DARKSLATEGRAY" | "DARKSLATEGREY" => (0x2F, 0x4F, 0x4F),
        "DARKTURQUOISE" => (0x00, 0xCE, 0xD1),
        "DARKVIOLET" => (0x94, 0x00, 0xD3),
        "DEEPPINK" => (0xFF, 0x14, 0x93),
        "DEEPSKYBLUE" => (0x00, 0xBF, 0xFF),
        "DIMGRAY" | "DIMGREY" => (0x69, 0x69, 0x69),
        "DODGERBLUE" => (0x1E, 0x90, 0xFF),
        // ---- F 开头 ----
        "FIREBRICK" => (0xB2, 0x22, 0x22),
        "FLORALWHITE" => (0xFF, 0xFA, 0xF0),
        "FORESTGREEN" => (0x22, 0x8B, 0x22),
        "FUCHSIA" => (0xFF, 0x00, 0xFF),
        // ---- G 开头 ----
        "GAINSBORO" => (0xDC, 0xDC, 0xDC),
        "GHOSTWHITE" => (0xF8, 0xF8, 0xFF),
        "GOLD" => (0xFF, 0xD7, 0x00),
        "GOLDENROD" => (0xDA, 0xA5, 0x20),
        "GRAY" | "GREY" => (0x80, 0x80, 0x80),
        "GREEN" => (0x00, 0x80, 0x00),
        "GREENYELLOW" => (0xAD, 0xFF, 0x2F),
        // ---- H 开头 ----
        "HONEYDEW" => (0xF0, 0xFF, 0xF0),
        "HOTPINK" => (0xFF, 0x69, 0xB4),
        // ---- I 开头 ----
        "INDIANRED" => (0xCD, 0x5C, 0x5C),
        "INDIGO" => (0x4B, 0x00, 0x82),
        "IVORY" => (0xFF, 0xFF, 0xF0),
        // ---- K 开头 ----
        "KHAKI" => (0xF0, 0xE6, 0x8C),
        // ---- L 开头（大量 LIGHT* 变体）----
        "LAVENDER" => (0xE6, 0xE6, 0xFA),
        "LAWNGREEN" => (0x7C, 0xFC, 0x00),
        "LEMONCHIFFON" => (0xFF, 0xFA, 0xCD),
        "LIGHTBLUE" => (0xAD, 0xD8, 0xE6),
        "LIGHTCORAL" => (0xF0, 0x80, 0x80),
        "LIGHTCYAN" => (0xE0, 0xFF, 0xFF),
        "LIGHTGOLDENRODYELLOW" => (0xFA, 0xFA, 0xD2),
        "LIGHTGRAY" | "LIGHTGREY" => (0xD3, 0xD3, 0xD3),
        "LIGHTGREEN" => (0x90, 0xEE, 0x90),
        "LIGHTPINK" => (0xFF, 0xB6, 0xC1),
        "LIGHTSEAGREEN" => (0x20, 0xB2, 0xAA),
        "LIGHTSKYBLUE" => (0x87, 0xCE, 0xFA),
        "LIGHTSLATEGRAY" | "LIGHTSLATEGREY" => (0x77, 0x88, 0x99),
        "LIGHTSTEELBLUE" => (0xB0, 0xC4, 0xDE),
        "LIGHTYELLOW" => (0xFF, 0xFF, 0xE0),
        "LIME" => (0x00, 0xFF, 0x00),
        "LIMEGREEN" => (0x32, 0xCD, 0x32),
        "LINEN" => (0xFA, 0xF0, 0xE6),
        "MAGENTA" => (0xFF, 0x00, 0xFF),
        "MAROON" => (0x80, 0x00, 0x00),
        // ---- M 开头（大量 MEDIUM* 变体）----
        "MEDIUMAQUAMARINE" => (0x66, 0xCD, 0xAA),
        "MEDIUMBLUE" => (0x00, 0x00, 0xCD),
        "MEDIUMORCHID" => (0xBA, 0x55, 0xD3),
        "MEDIUMPURPLE" => (0x93, 0x70, 0xDB),
        "MEDIUMSEAGREEN" => (0x3C, 0xB3, 0x71),
        "MEDIUMSLATEBLUE" => (0x7B, 0x68, 0xEE),
        "MEDIUMSPRINGGREEN" => (0x00, 0xFA, 0x9A),
        "MEDIUMTURQUOISE" => (0x48, 0xD1, 0xCC),
        "MEDIUMVIOLETRED" => (0xC7, 0x15, 0x85),
        "MIDNIGHTBLUE" => (0x19, 0x19, 0x70),
        "MINTCREAM" => (0xF5, 0xFF, 0xFA),
        "MISTYROSE" => (0xFF, 0xE4, 0xE1),
        "MOCCASIN" => (0xFF, 0xE4, 0xB5),
        // ---- N 开头 ----
        "NAVAJOWHITE" => (0xFF, 0xDE, 0xAD),
        "NAVY" => (0x00, 0x00, 0x80),
        // ---- O 开头 ----
        "OLDLACE" => (0xFD, 0xF5, 0xE6),
        "OLIVE" => (0x80, 0x80, 0x00),
        "OLIVEDRAB" => (0x6B, 0x8E, 0x23),
        "ORANGE" => (0xFF, 0xA5, 0x00),
        "ORANGERED" => (0xFF, 0x45, 0x00),
        "ORCHID" => (0xDA, 0x70, 0xD6),
        // ---- P 开头 ----
        "PALEGOLDENROD" => (0xEE, 0xE8, 0xAA),
        "PALEGREEN" => (0x98, 0xFB, 0x98),
        "PALETURQUOISE" => (0xAF, 0xEE, 0xEE),
        "PALEVIOLETRED" => (0xDB, 0x70, 0x93),
        "PAPAYAWHIP" => (0xFF, 0xEF, 0xD5),
        "PEACHPUFF" => (0xFF, 0xDA, 0xB9),
        "PERU" => (0xCD, 0x85, 0x3F),
        "PINK" => (0xFF, 0xC0, 0xCB),
        "PLUM" => (0xDD, 0xA0, 0xDD),
        "POWDERBLUE" => (0xB0, 0xE0, 0xE6),
        "PURPLE" => (0x80, 0x00, 0x80),
        // ---- R 开头 ----
        "RED" => (0xFF, 0x00, 0x00),
        "ROSYBROWN" => (0xBC, 0x8F, 0x8F),
        "ROYALBLUE" => (0x41, 0x69, 0xE1),
        // ---- S 开头 ----
        "SADDLEBROWN" => (0x8B, 0x45, 0x13),
        "SALMON" => (0xFA, 0x80, 0x72),
        "SANDYBROWN" => (0xF4, 0xA4, 0x60),
        "SEAGREEN" => (0x2E, 0x8B, 0x57),
        "SEASHELL" => (0xFF, 0xF5, 0xEE),
        "SIENNA" => (0xA0, 0x52, 0x2D),
        "SILVER" => (0xC0, 0xC0, 0xC0),
        "SKYBLUE" => (0x87, 0xCE, 0xEB),
        "SLATEBLUE" => (0x6A, 0x5A, 0xCD),
        "SLATEGRAY" | "SLATEGREY" => (0x70, 0x80, 0x90),
        "SNOW" => (0xFF, 0xFA, 0xFA),
        "SPRINGGREEN" => (0x00, 0xFF, 0x7F),
        "STEELBLUE" => (0x46, 0x82, 0xB4),
        // ---- T 开头 ----
        "TAN" => (0xD2, 0xB4, 0x8C),
        "TEAL" => (0x00, 0x80, 0x80),
        "THISTLE" => (0xD8, 0xBF, 0xD8),
        "TOMATO" => (0xFF, 0x63, 0x47),
        "TURQUOISE" => (0x40, 0xE0, 0xD0),
        // ---- V 开头 ----
        "VIOLET" => (0xEE, 0x82, 0xEE),
        // ---- W 开头 ----
        "WHEAT" => (0xF5, 0xDE, 0xB3),
        "WHITE" => (0xFF, 0xFF, 0xFF),
        "WHITESMOKE" => (0xF5, 0xF5, 0xF5),
        // ---- Y 开头 ----
        "YELLOW" => (0xFF, 0xFF, 0x00),
        "YELLOWGREEN" => (0x9A, 0xCD, 0x32),
        // 未命中命名表。
        _ => return None,
    })
}

/// `Color.namedColor` + `TRANSPARENT`：把一个（大小写不敏感的）CSS 命名
/// 颜色解析为 0..1 的 rgba 浮点值。
fn named_color(name_upper: &str) -> Option<DVec4> {
    // TRANSPARENT 是唯一的零 alpha 特例（其余命名色 alpha=1）。
    if name_upper == "TRANSPARENT" {
        return Some(DVec4::new(0.0, 0.0, 0.0, 0.0));
    }
    let (r, g, b) = named_color_rgb(name_upper)?;
    // 命中的命名色统一以不透明 alpha=1 输出。
    Some(DVec4::new(
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        1.0,
    ))
}

/// 镜像 `rgbParenthesesMatcher`（见 `color.rs::parse_rgb_functional`）；
/// 返回已缩放到 0..1 的 `(r, g, b, a)`。
fn parse_rgb_functional(color: &str) -> Option<(f64, f64, f64, f64)> {
    let chars: Vec<char> = color.chars().collect();
    let n = chars.len();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();

    let mut i;
    // 前缀必须是 "rgb"（大小写不敏感）；后面可选一个 "a" 变成 "rgba"。
    if n < 3 || lower[0] != 'r' || lower[1] != 'g' || lower[2] != 'b' {
        return None;
    }
    i = 3;
    // 检测可选的 'a' 后缀（rgba / hsla）。
    if i < n && lower[i] == 'a' {
        i += 1;
    }
    // 跳过函数名与 '(' 之间的 CSS 空白。
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    if i >= n || chars[i] != '(' {
        return None;
    }
    i += 1;

    // 依次解析 r/g/b 三个分量：允许 % 百分比后缀，分量间以逗号/空白分隔。
    let mut components = [0.0f64; 3];
    let mut percentages = [false; 3];
    for k in 0..3usize {
        while i < n && is_css_whitespace(chars[i]) {
            i += 1;
        }
        // 跳过分量前的空白，扫描一个 [0-9.]+ 数值 token。
        let start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i == start {
            return None;
        }
        let token: String = chars[start..i].iter().collect();
        // 紧跟 '%' 则标记为百分比分量。
        if i < n && chars[i] == '%' {
            percentages[k] = true;
            i += 1;
        }
        components[k] = parse_float_js(&token);

        if k < 2 {
            // 前两个分量后必须至少有一个逗号或空白作为分隔符。
            let separator_start = i;
            while i < n && (chars[i] == ',' || is_css_whitespace(chars[i])) {
                i += 1;
            }
            if i == separator_start {
                return None;
            }
        }
    }

    // alpha 默认为 1；仅当出现分隔符（逗号或斜杠）时才尝试解析尾部数值，
    // 解析失败则回滚到括号前的游标。
    let mut alpha = 1.0f64;
    let before_group = i;
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    // 扫描分量组与可选 alpha 之间的分隔符（逗号/斜杠/空白）。
    let separator_start = i;
    while i < n && (chars[i] == ',' || chars[i] == '/' || is_css_whitespace(chars[i])) {
        i += 1;
    }
    // 若确实出现分隔符，则尝试读取尾部的 alpha 数值；否则回滚游标。
    if i > separator_start {
        while i < n && is_css_whitespace(chars[i]) {
            i += 1;
        }
        let number_start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i > number_start {
            let token: String = chars[number_start..i].iter().collect();
            alpha = parse_float_js(&token);
        } else {
            i = before_group;
        }
    } else {
        i = before_group;
    }

    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    // 收尾：跳过空白后必须恰好一个 ')'，且紧接着就到串尾。
    if i >= n || chars[i] != ')' {
        return None;
    }
    i += 1;
    if i != n {
        return None;
    }

    // 缩放回 0..1：百分比分量除以 100，否则除以 255。
    let scale = |k: usize| if percentages[k] { 100.0 } else { 255.0 };
    Some((
        components[0] / scale(0),
        components[1] / scale(1),
        components[2] / scale(2),
        alpha,
    ))
}

/// 镜像 `hslParenthesesMatcher`（见 `color.rs::parse_hsl_functional`）；
/// 返回原始的 `(hue, saturation, lightness, alpha)` 捕获值。
fn parse_hsl_functional(color: &str) -> Option<(f64, f64, f64, f64)> {
    let chars: Vec<char> = color.chars().collect();
    let n = chars.len();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();

    let mut i;
    // 前缀必须是 "hsl"（大小写不敏感）；后面可选一个 "a" 变成 "hsla"。
    if n < 3 || lower[0] != 'h' || lower[1] != 's' || lower[2] != 'l' {
        return None;
    }
    i = 3;
    // 检测可选的 'a' 后缀（rgba / hsla）。
    if i < n && lower[i] == 'a' {
        i += 1;
    }
    // 跳过函数名与 '(' 之间的 CSS 空白。
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    if i >= n || chars[i] != '(' {
        return None;
    }
    i += 1;

    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    let start = i;
    while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
    }
    if i == start {
        return None;
    }
    // hue 为不带 % 的度数（后续除以 360）。
    let hue: String = chars[start..i].iter().collect();
    let hue = parse_float_js(&hue);

    // saturation/lightness 必须带 % 后缀，依次捕获到两个槽位。
    let mut captured = [0.0f64; 2];
    for slot in captured.iter_mut() {
        // 分隔符（逗号/空白）后扫描数值，且必须紧跟 '%'。
        let separator_start = i;
        while i < n && (chars[i] == ',' || is_css_whitespace(chars[i])) {
            i += 1;
        }
        if i == separator_start {
            return None;
        }
        let start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i == start || i >= n || chars[i] != '%' {
            return None;
        }
        let token: String = chars[start..i].iter().collect();
        i += 1; // 消耗 '%'
        *slot = parse_float_js(&token);
    }

    // alpha 默认为 1；仅当出现分隔符（逗号或斜杠）时才尝试解析尾部数值，
    // 解析失败则回滚到括号前的游标。
    let mut alpha = 1.0f64;
    let before_group = i;
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    // 扫描分量组与可选 alpha 之间的分隔符（逗号/斜杠/空白）。
    let separator_start = i;
    while i < n && (chars[i] == ',' || chars[i] == '/' || is_css_whitespace(chars[i])) {
        i += 1;
    }
    // 若确实出现分隔符，则尝试读取尾部的 alpha 数值；否则回滚游标。
    if i > separator_start {
        while i < n && is_css_whitespace(chars[i]) {
            i += 1;
        }
        let number_start = i;
        while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        if i > number_start {
            let token: String = chars[number_start..i].iter().collect();
            alpha = parse_float_js(&token);
        } else {
            i = before_group;
        }
    } else {
        i = before_group;
    }

    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    // 收尾：跳过空白后必须恰好一个 ')'，且紧接着就到串尾。
    if i >= n || chars[i] != ')' {
        return None;
    }
    i += 1;
    if i != n {
        return None;
    }

    Some((hue, captured[0], captured[1], alpha))
}

/// 镜像 `Color.fromCssColorString`：命名色 -> `#rgb`/`#rgba` ->
/// `#rrggbb`/`#rrggbbaa` -> `rgb()`/`rgba()` -> `hsl()`/`hsla()`。
pub fn from_css_color_string(color: &str) -> Option<DVec4> {
    let color = color.trim();

    // 优先按命名颜色（含 TRANSPARENT）解析。
    if let Some(named) = named_color(&color.to_ascii_uppercase()) {
        return Some(named);
    }

    // 然后是十六进制形式：#rgb / #rgba / #rrggbb / #rrggbbaa。
    if let Some(stripped) = color.strip_prefix('#') {
        let hex: Vec<char> = stripped.chars().collect();
        // 3/4 位短形式：每位复制，除以 15 归一到 0..1。
        if (hex.len() == 3 || hex.len() == 4) && hex.iter().all(|c| c.is_ascii_hexdigit()) {
            let r = hex[0].to_digit(16).unwrap() as f64 / 15.0;
            let g = hex[1].to_digit(16).unwrap() as f64 / 15.0;
            let b = hex[2].to_digit(16).unwrap() as f64 / 15.0;
            // 第 4 位为可选 alpha，缺省时补 15（=1.0）。
            let a = hex
                .get(3)
                .map(|c| c.to_digit(16).unwrap() as f64)
                .unwrap_or(15.0)
                / 15.0;
            return Some(DVec4::new(r, g, b, a));
        }
        // 6/8 位长形式：每两位解析为 0..255 再除以 255。
        if (hex.len() == 6 || hex.len() == 8) && hex.iter().all(|c| c.is_ascii_hexdigit()) {
            // 每两位十六进制 -> 0..255 -> 0..1。
            let pair = |i: usize| {
                u32::from_str_radix(&stripped[i * 2..i * 2 + 2], 16).unwrap() as f64 / 255.0
            };
            let a = if hex.len() == 8 { pair(3) } else { 1.0 };
            return Some(DVec4::new(pair(0), pair(1), pair(2), a));
        }
    }

    // 最后尝试函数形式 rgb()/rgba() 然后 hsl()/hsla()。
    if let Some((r, g, b, a)) = parse_rgb_functional(color) {
        return Some(DVec4::new(r, g, b, a));
    }

    // hsl 参数（度/百分比）归一到 from_hsl 期望的 0..1 域。
    if let Some((h, s, l, a)) = parse_hsl_functional(color) {
        return Some(from_hsl(h / 360.0, s / 100.0, l / 100.0, a));
    }

    // 无任何形式匹配。
    None
}

// ---------------------------------------------------------------------------
// 字面量求值
// ---------------------------------------------------------------------------

/// 镜像 `_evaluateLiteralColor`。
pub fn evaluate_literal_color(
    name: &str,
    args: Option<&[Node]>,
    feature: Option<&dyn ExpressionFeature>,
) -> Result<Value, RuntimeError> {
    let color = match name {
        // color(...)：单个 CSS 串；可选第二个参数覆盖 alpha 透明度。
        "color" => match args {
            // 无参 color() 回退为不透明白。
            None => DVec4::new(1.0, 1.0, 1.0, 1.0),
            // 两参数：第二个覆盖 alpha 分量。
            Some(a) if a.len() > 1 => {
                let css = a[0].evaluate(feature)?.string_conversion();
                let mut color = from_css_color_string(&css).ok_or_else(|| {
                    runtime_error(&format!("{css} is not a valid color."))
                })?;
                color.w = a[1].evaluate(feature)?.number_conversion();
                color
            }
            // 单参数：纯 CSS 串。
            Some(a) => {
                let css = a[0].evaluate(feature)?.string_conversion();
                from_css_color_string(&css).ok_or_else(|| {
                    runtime_error(&format!("{css} is not a valid color."))
                })?
            }
        },
        "rgb" => {
            let a = args.expect("rgb requires arguments");
            // 镜像 Color.fromBytes(r, g, b, 255)：byteToFloat 只是简单地
            // 除以 255，不做取整/钳制。
            DVec4::new(
                a[0].evaluate(feature)?.number_conversion() / 255.0,
                a[1].evaluate(feature)?.number_conversion() / 255.0,
                a[2].evaluate(feature)?.number_conversion() / 255.0,
                1.0,
            )
        }
        "rgba" => {
            let a = args.expect("rgba requires arguments");
            // 在 css alpha（0 到 1）与 cesium alpha（0 到 255）之间转换；
            // byteToFloat 再除以 255，故 alpha 被精确保留。
            let alpha = a[3].evaluate(feature)?.number_conversion() * 255.0;
            DVec4::new(
                a[0].evaluate(feature)?.number_conversion() / 255.0,
                a[1].evaluate(feature)?.number_conversion() / 255.0,
                a[2].evaluate(feature)?.number_conversion() / 255.0,
                alpha / 255.0,
            )
        }
        // hsl(h, s, l)：三参数转 HSL 色（alpha 默认 1）。
        "hsl" => {
            let a = args.expect("hsl requires arguments");
            from_hsl(
                a[0].evaluate(feature)?.number_conversion(),
                a[1].evaluate(feature)?.number_conversion(),
                a[2].evaluate(feature)?.number_conversion(),
                1.0,
            )
        }
        // hsla(h, s, l, a)：四参数 HSL 色，最后一位为 alpha。
        "hsla" => {
            let a = args.expect("hsla requires arguments");
            from_hsl(
                a[0].evaluate(feature)?.number_conversion(),
                a[1].evaluate(feature)?.number_conversion(),
                a[2].evaluate(feature)?.number_conversion(),
                a[3].evaluate(feature)?.number_conversion(),
            )
        }
        // 构造器名在 ast 阶段已限定为五个之一。
        _ => unreachable!("literal color name is one of color/rgb/rgba/hsl/hsla"),
    };
    Ok(Value::Cartesian4(color))
}

/// 镜像 `_evaluateLiteralVector`。
pub fn evaluate_literal_vector(
    call: &str,
    args: &[Node],
    feature: Option<&dyn ExpressionFeature>,
) -> Result<Value, RuntimeError> {
    // 把每个参数展平为标量分量序列（向量按分量展开，数字直接推入）。
    let mut components: Vec<f64> = Vec::new();
    let args_length = args.len();
    for argument in args {
        let value = argument.evaluate(feature)?;
        // 单个参数展平后按类型推入：数字直接推入，向量按分量展开。
        match value {
            Value::Number(n) => components.push(n),
            Value::Cartesian2(v) => components.extend([v.x, v.y]),
            Value::Cartesian3(v) => components.extend([v.x, v.y, v.z]),
            Value::Cartesian4(v) => components.extend([v.x, v.y, v.z, v.w]),
            // 其余类型（字符串/布尔等）不是合法向量分量。
            _ => {
                return Err(runtime_error(&format!(
                    "{call} argument must be a vector or number. Argument is {value}."
                )))
            }
        }
    }

    let components_length = components.len();
    // 从 "vecN" 的第 4 个字符取目标维数 N。
    let vector_length = call
        .chars()
        .nth(3)
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as usize;

    // 校验分量数：不能为空、不能不足（>1 分量时）、不能过多。
    if components_length == 0 {
        return Err(runtime_error(&format!(
            "Invalid {call} constructor. No valid arguments."
        )));
    } else if components_length < vector_length && components_length > 1 {
        return Err(runtime_error(&format!(
            "Invalid {call} constructor. Not enough arguments."
        )));
    } else if components_length > vector_length && args_length > 1 {
        return Err(runtime_error(&format!(
            "Invalid {call} constructor. Too many arguments."
        )));
    }

    if components_length == 1 {
        // 标量入参广播：把同一个分量再添加 3 次。
        let component = components[0];
        components.extend([component, component, component]);
    }

    // 按 call 名分派到对应维度的向量构造。
    if call == "vec2" {
        Ok(Value::Cartesian2(DVec2::new(components[0], components[1])))
    } else if call == "vec3" {
        Ok(Value::Cartesian3(DVec3::new(
            components[0], components[1], components[2],
        )))
    } else {
        Ok(Value::Cartesian4(DVec4::new(
            components[0], components[1], components[2], components[3],
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// to_byte 钳制到 [0,255] 并四舍五入。
    #[test]
    fn to_byte_clamps_and_rounds() {
        // 低于 0 钳到 0。
        assert_eq!(to_byte(-10.0), 0);
        // 高于 255 钳到 255。
        assert_eq!(to_byte(300.0), 255);
        // 1.4 四舍五入为 1。
        assert_eq!(to_byte(1.4), 1);
        // 1.6 四舍五入为 2。
        assert_eq!(to_byte(1.6), 2);
        assert_eq!(to_byte(255.0), 255);
    }

    /// parseFloat 语义：取最长前导数字前缀，非法前缀返回 NaN。
    #[test]
    fn parse_float_js_longest_prefix() {
        // 尾部非数字被丢弃。
        assert_eq!(parse_float_js("12.5abc"), 12.5);
        // 科学计数法。
        assert_eq!(parse_float_js("1e3"), 1000.0);
        // 不识别 "inf"/"NaN"，与 parseFloat 一致。
        assert!(parse_float_js("abc").is_nan());
        assert!(parse_float_js("inf").is_nan());
    }

    /// 命名颜色与 TRANSPARENT 解析，含大小写不敏感与别名。
    #[test]
    fn named_colors_byte_exact() {
        // RED == #FF0000 == (1, 0, 0, 1)
        assert_eq!(from_css_color_string("red").unwrap(), DVec4::new(1.0, 0.0, 0.0, 1.0));
        // 大小写不敏感。
        assert_eq!(from_css_color_string("RED").unwrap(), DVec4::new(1.0, 0.0, 0.0, 1.0));
        // GREY 别名 == GRAY == #808080
        let grey = from_css_color_string("grey").unwrap();
        // x 通道等于 0x80/255。
        assert!((grey.x - 0x80 as f64 / 255.0).abs() < 1e-12);
        // TRANSPARENT == (0,0,0,0)
        assert_eq!(
            from_css_color_string("transparent").unwrap(),
            DVec4::new(0.0, 0.0, 0.0, 0.0)
        );
        // 未知名称 -> None。
        assert!(from_css_color_string("notacolor").is_none());
    }

    /// 十六进制形式 #rgb/#rgba/#rrggbb/#rrggbbaa 的解析与缩放。
    #[test]
    fn hex_forms() {
        // #fff == 白色
        assert_eq!(from_css_color_string("#fff").unwrap(), DVec4::new(1.0, 1.0, 1.0, 1.0));
        // #ff000080 -> alpha 0x80/255
        let c = from_css_color_string("#ff0000").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 1.0));
        let c8 = from_css_color_string("#ff000080").unwrap();
        // 8 位尾缀 alpha 归一到 0..1。
        assert!((c8.w - 0x80 as f64 / 255.0).abs() < 1e-12);
        // #rgba 4 位
        let c4 = from_css_color_string("#f00f").unwrap();
        assert_eq!(c4, DVec4::new(1.0, 0.0, 0.0, 1.0));
    }

    /// rgb()/rgba() 函数形式，含百分比分量。
    #[test]
    fn rgb_functional() {
        // rgb(255,0,0) -> 纯红。
        let c = from_css_color_string("rgb(255, 0, 0)").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 1.0));
        // rgba 第四位为 alpha 0.5。
        let c = from_css_color_string("rgba(255, 0, 0, 0.5)").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 0.5));
        // 百分比分量 100% 等同 255。
        let c = from_css_color_string("rgb(100%, 0%, 0%)").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 1.0));
    }

    /// hsl()/hsla() 函数形式：纯红色相验证。
    #[test]
    fn hsl_functional() {
        // hsl(0, 100%, 50%) == 红色
        let c = from_css_color_string("hsl(0, 100%, 50%)").unwrap();
        assert!((c.x - 1.0).abs() < 1e-12);
        // 绿通道为 0。
        assert!((c.y - 0.0).abs() < 1e-12);
        // 蓝通道为 0。
        assert!((c.z - 0.0).abs() < 1e-12);
    }

    /// saturation=0 时 HSL 退化为以亮度为值的灰度。
    #[test]
    fn from_hsl_greyscale_when_unsaturated() {
        // saturation 为 0 -> 所有通道 == lightness
        let c = from_hsl(0.0, 0.0, 0.5, 1.0);
        assert_eq!(c, DVec4::new(0.5, 0.5, 0.5, 1.0));
    }
}
