//! Color and vector literal evaluation for the styling language.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs` L1956-2102
//! (`to_byte` / `evaluate_literal_color` / `evaluate_literal_vector`), the Rust
//! port of upstream `packages/engine/Source/Scene/Expression.js`
//! (`_evaluateLiteralColor` / `_evaluateLiteralVector`).
//!
//! The CSS color parsing (`color("...")` / named colors / `#rgb` / `#rrggbb` /
//! `rgb()` / `hsl()`) is ported from the blueprint
//! `cesium-rs/crates/cesium-core/src/color.rs` (`from_css_color_string`,
//! `from_hsl`, `hue2rgb`, `parse_rgb_functional`, `parse_hsl_functional`,
//! `parse_float_js`, `is_css_whitespace`, the 148-entry named-color table).
//!
//! # DEVIATION (deps)
//!
//! The blueprint draws `Color` and `Cartesian2/3/4` from `cesium_core`. This
//! isolated domain crate only depends on `glam`, so:
//! * A color is represented directly as `glam::DVec4` (rgba, components 0..1);
//!   `evaluate_literal_color` returns `Value::Cartesian4` exactly as the
//!   blueprint does after `Color -> Cartesian4::from_elements`.
//! * The full CSS color parser is re-implemented here (byte-exact against
//!   `color.rs`) rather than importing the blueprint `Color`, which lives in a
//!   read-only crate this domain must not depend on.

use glam::{DVec2, DVec3, DVec4};

use crate::ast::Node;
use crate::runtime::ExpressionFeature;
use crate::value::{runtime_error, RuntimeError, Value};

// ---------------------------------------------------------------------------
// JS/CSS numeric helpers (ported from color.rs)
// ---------------------------------------------------------------------------

/// Mirrors `Color.fromBytes` argument clamping (`CesiumMath.clamp` to byte):
/// clamp to `[0, 255]`, round, truncate to `u8`.
///
/// NOTE: `evaluate_literal_color`'s `rgb()`/`rgba()` paths divide by 255.0
/// directly (blueprint-faithful `byteToFloat`, no rounding/clamping), so this
/// helper is exposed for callers/tests that need the `Color.fromBytes` byte
/// quantization rather than being used internally here.
pub fn to_byte(value: f64) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

/// Emulates ECMAScript `parseFloat`: parses the longest leading numeric prefix
/// and returns NaN if none exists (never recognises "inf"/"NaN").
fn parse_float_js(s: &str) -> f64 {
    for end in (1..=s.len()).rev() {
        let Some(candidate) = s.get(..end) else {
            continue;
        };
        if candidate
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        {
            if let Ok(v) = candidate.parse::<f64>() {
                return v;
            }
        }
    }
    f64::NAN
}

/// The `\s` character set of the original JS color regular expressions
/// (ECMAScript WhiteSpace + LineTerminator).
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

/// Helper: convert hue to an rgb component (ported from `color.rs::hue2rgb`).
fn hue2rgb(m1: f64, m2: f64, mut h: f64) -> f64 {
    if h < 0.0 {
        h += 1.0;
    }
    if h > 1.0 {
        h -= 1.0;
    }
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

/// `Color.fromHsl(hue, saturation, lightness, alpha)` (all inputs 0..1).
fn from_hsl(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> DVec4 {
    let hue = hue % 1.0;
    let mut red = lightness;
    let mut green = lightness;
    let mut blue = lightness;

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

/// The 148 CSS named colors as 24-bit `(r, g, b)` triples (uppercase keys).
/// Aliases (`DARKGREY`, `GREY`, ...) are folded into their canonical entry.
fn named_color_rgb(name: &str) -> Option<(u8, u8, u8)> {
    Some(match name {
        "ALICEBLUE" => (0xF0, 0xF8, 0xFF),
        "ANTIQUEWHITE" => (0xFA, 0xEB, 0xD7),
        "AQUA" => (0x00, 0xFF, 0xFF),
        "AQUAMARINE" => (0x7F, 0xFF, 0xD4),
        "AZURE" => (0xF0, 0xFF, 0xFF),
        "BEIGE" => (0xF5, 0xF5, 0xDC),
        "BISQUE" => (0xFF, 0xE4, 0xC4),
        "BLACK" => (0x00, 0x00, 0x00),
        "BLANCHEDALMOND" => (0xFF, 0xEB, 0xCD),
        "BLUE" => (0x00, 0x00, 0xFF),
        "BLUEVIOLET" => (0x8A, 0x2B, 0xE2),
        "BROWN" => (0xA5, 0x2A, 0x2A),
        "BURLYWOOD" => (0xDE, 0xB8, 0x87),
        "CADETBLUE" => (0x5F, 0x9E, 0xA0),
        "CHARTREUSE" => (0x7F, 0xFF, 0x00),
        "CHOCOLATE" => (0xD2, 0x69, 0x1E),
        "CORAL" => (0xFF, 0x7F, 0x50),
        "CORNFLOWERBLUE" => (0x64, 0x95, 0xED),
        "CORNSILK" => (0xFF, 0xF8, 0xDC),
        "CRIMSON" => (0xDC, 0x14, 0x3C),
        "CYAN" => (0x00, 0xFF, 0xFF),
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
        "FIREBRICK" => (0xB2, 0x22, 0x22),
        "FLORALWHITE" => (0xFF, 0xFA, 0xF0),
        "FORESTGREEN" => (0x22, 0x8B, 0x22),
        "FUCHSIA" => (0xFF, 0x00, 0xFF),
        "GAINSBORO" => (0xDC, 0xDC, 0xDC),
        "GHOSTWHITE" => (0xF8, 0xF8, 0xFF),
        "GOLD" => (0xFF, 0xD7, 0x00),
        "GOLDENROD" => (0xDA, 0xA5, 0x20),
        "GRAY" | "GREY" => (0x80, 0x80, 0x80),
        "GREEN" => (0x00, 0x80, 0x00),
        "GREENYELLOW" => (0xAD, 0xFF, 0x2F),
        "HONEYDEW" => (0xF0, 0xFF, 0xF0),
        "HOTPINK" => (0xFF, 0x69, 0xB4),
        "INDIANRED" => (0xCD, 0x5C, 0x5C),
        "INDIGO" => (0x4B, 0x00, 0x82),
        "IVORY" => (0xFF, 0xFF, 0xF0),
        "KHAKI" => (0xF0, 0xE6, 0x8C),
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
        "NAVAJOWHITE" => (0xFF, 0xDE, 0xAD),
        "NAVY" => (0x00, 0x00, 0x80),
        "OLDLACE" => (0xFD, 0xF5, 0xE6),
        "OLIVE" => (0x80, 0x80, 0x00),
        "OLIVEDRAB" => (0x6B, 0x8E, 0x23),
        "ORANGE" => (0xFF, 0xA5, 0x00),
        "ORANGERED" => (0xFF, 0x45, 0x00),
        "ORCHID" => (0xDA, 0x70, 0xD6),
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
        "RED" => (0xFF, 0x00, 0x00),
        "ROSYBROWN" => (0xBC, 0x8F, 0x8F),
        "ROYALBLUE" => (0x41, 0x69, 0xE1),
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
        "TAN" => (0xD2, 0xB4, 0x8C),
        "TEAL" => (0x00, 0x80, 0x80),
        "THISTLE" => (0xD8, 0xBF, 0xD8),
        "TOMATO" => (0xFF, 0x63, 0x47),
        "TURQUOISE" => (0x40, 0xE0, 0xD0),
        "VIOLET" => (0xEE, 0x82, 0xEE),
        "WHEAT" => (0xF5, 0xDE, 0xB3),
        "WHITE" => (0xFF, 0xFF, 0xFF),
        "WHITESMOKE" => (0xF5, 0xF5, 0xF5),
        "YELLOW" => (0xFF, 0xFF, 0x00),
        "YELLOWGREEN" => (0x9A, 0xCD, 0x32),
        _ => return None,
    })
}

/// `Color.namedColor` + `TRANSPARENT`: resolves a (case-insensitive) CSS named
/// color to rgba floats in 0..1.
fn named_color(name_upper: &str) -> Option<DVec4> {
    if name_upper == "TRANSPARENT" {
        return Some(DVec4::new(0.0, 0.0, 0.0, 0.0));
    }
    let (r, g, b) = named_color_rgb(name_upper)?;
    Some(DVec4::new(
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        1.0,
    ))
}

/// Mirrors `rgbParenthesesMatcher` (see `color.rs::parse_rgb_functional`);
/// returns `(r, g, b, a)` already scaled to 0..1.
fn parse_rgb_functional(color: &str) -> Option<(f64, f64, f64, f64)> {
    let chars: Vec<char> = color.chars().collect();
    let n = chars.len();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();

    let mut i;
    if n < 3 || lower[0] != 'r' || lower[1] != 'g' || lower[2] != 'b' {
        return None;
    }
    i = 3;
    if i < n && lower[i] == 'a' {
        i += 1;
    }
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    if i >= n || chars[i] != '(' {
        return None;
    }
    i += 1;

    let mut components = [0.0f64; 3];
    let mut percentages = [false; 3];
    for k in 0..3usize {
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
        let token: String = chars[start..i].iter().collect();
        if i < n && chars[i] == '%' {
            percentages[k] = true;
            i += 1;
        }
        components[k] = parse_float_js(&token);

        if k < 2 {
            let separator_start = i;
            while i < n && (chars[i] == ',' || is_css_whitespace(chars[i])) {
                i += 1;
            }
            if i == separator_start {
                return None;
            }
        }
    }

    let mut alpha = 1.0f64;
    let before_group = i;
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    let separator_start = i;
    while i < n && (chars[i] == ',' || chars[i] == '/' || is_css_whitespace(chars[i])) {
        i += 1;
    }
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
    if i >= n || chars[i] != ')' {
        return None;
    }
    i += 1;
    if i != n {
        return None;
    }

    let scale = |k: usize| if percentages[k] { 100.0 } else { 255.0 };
    Some((
        components[0] / scale(0),
        components[1] / scale(1),
        components[2] / scale(2),
        alpha,
    ))
}

/// Mirrors `hslParenthesesMatcher` (see `color.rs::parse_hsl_functional`);
/// returns the raw `(hue, saturation, lightness, alpha)` captures.
fn parse_hsl_functional(color: &str) -> Option<(f64, f64, f64, f64)> {
    let chars: Vec<char> = color.chars().collect();
    let n = chars.len();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();

    let mut i;
    if n < 3 || lower[0] != 'h' || lower[1] != 's' || lower[2] != 'l' {
        return None;
    }
    i = 3;
    if i < n && lower[i] == 'a' {
        i += 1;
    }
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
    let hue: String = chars[start..i].iter().collect();
    let hue = parse_float_js(&hue);

    let mut captured = [0.0f64; 2];
    for slot in captured.iter_mut() {
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
        i += 1; // consume '%'
        *slot = parse_float_js(&token);
    }

    let mut alpha = 1.0f64;
    let before_group = i;
    while i < n && is_css_whitespace(chars[i]) {
        i += 1;
    }
    let separator_start = i;
    while i < n && (chars[i] == ',' || chars[i] == '/' || is_css_whitespace(chars[i])) {
        i += 1;
    }
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
    if i >= n || chars[i] != ')' {
        return None;
    }
    i += 1;
    if i != n {
        return None;
    }

    Some((hue, captured[0], captured[1], alpha))
}

/// Mirrors `Color.fromCssColorString`: named -> `#rgb`/`#rgba` ->
/// `#rrggbb`/`#rrggbbaa` -> `rgb()`/`rgba()` -> `hsl()`/`hsla()`.
pub fn from_css_color_string(color: &str) -> Option<DVec4> {
    let color = color.trim();

    if let Some(named) = named_color(&color.to_ascii_uppercase()) {
        return Some(named);
    }

    if let Some(stripped) = color.strip_prefix('#') {
        let hex: Vec<char> = stripped.chars().collect();
        if (hex.len() == 3 || hex.len() == 4) && hex.iter().all(|c| c.is_ascii_hexdigit()) {
            let r = hex[0].to_digit(16).unwrap() as f64 / 15.0;
            let g = hex[1].to_digit(16).unwrap() as f64 / 15.0;
            let b = hex[2].to_digit(16).unwrap() as f64 / 15.0;
            let a = hex
                .get(3)
                .map(|c| c.to_digit(16).unwrap() as f64)
                .unwrap_or(15.0)
                / 15.0;
            return Some(DVec4::new(r, g, b, a));
        }
        if (hex.len() == 6 || hex.len() == 8) && hex.iter().all(|c| c.is_ascii_hexdigit()) {
            let pair = |i: usize| {
                u32::from_str_radix(&stripped[i * 2..i * 2 + 2], 16).unwrap() as f64 / 255.0
            };
            let a = if hex.len() == 8 { pair(3) } else { 1.0 };
            return Some(DVec4::new(pair(0), pair(1), pair(2), a));
        }
    }

    if let Some((r, g, b, a)) = parse_rgb_functional(color) {
        return Some(DVec4::new(r, g, b, a));
    }

    if let Some((h, s, l, a)) = parse_hsl_functional(color) {
        return Some(from_hsl(h / 360.0, s / 100.0, l / 100.0, a));
    }

    None
}

// ---------------------------------------------------------------------------
// Literal evaluation
// ---------------------------------------------------------------------------

/// Mirrors `_evaluateLiteralColor`.
pub fn evaluate_literal_color(
    name: &str,
    args: Option<&[Node]>,
    feature: Option<&dyn ExpressionFeature>,
) -> Result<Value, RuntimeError> {
    let color = match name {
        "color" => match args {
            None => DVec4::new(1.0, 1.0, 1.0, 1.0),
            Some(a) if a.len() > 1 => {
                let css = a[0].evaluate(feature)?.string_conversion();
                let mut color = from_css_color_string(&css).ok_or_else(|| {
                    runtime_error(&format!("{css} is not a valid color."))
                })?;
                color.w = a[1].evaluate(feature)?.number_conversion();
                color
            }
            Some(a) => {
                let css = a[0].evaluate(feature)?.string_conversion();
                from_css_color_string(&css).ok_or_else(|| {
                    runtime_error(&format!("{css} is not a valid color."))
                })?
            }
        },
        "rgb" => {
            let a = args.expect("rgb requires arguments");
            // Mirrors Color.fromBytes(r, g, b, 255): byteToFloat is a plain
            // divide by 255 with no rounding/clamping.
            DVec4::new(
                a[0].evaluate(feature)?.number_conversion() / 255.0,
                a[1].evaluate(feature)?.number_conversion() / 255.0,
                a[2].evaluate(feature)?.number_conversion() / 255.0,
                1.0,
            )
        }
        "rgba" => {
            let a = args.expect("rgba requires arguments");
            // convert between css alpha (0 to 1) and cesium alpha (0 to 255);
            // byteToFloat divides back by 255 so alpha is preserved exactly.
            let alpha = a[3].evaluate(feature)?.number_conversion() * 255.0;
            DVec4::new(
                a[0].evaluate(feature)?.number_conversion() / 255.0,
                a[1].evaluate(feature)?.number_conversion() / 255.0,
                a[2].evaluate(feature)?.number_conversion() / 255.0,
                alpha / 255.0,
            )
        }
        "hsl" => {
            let a = args.expect("hsl requires arguments");
            from_hsl(
                a[0].evaluate(feature)?.number_conversion(),
                a[1].evaluate(feature)?.number_conversion(),
                a[2].evaluate(feature)?.number_conversion(),
                1.0,
            )
        }
        "hsla" => {
            let a = args.expect("hsla requires arguments");
            from_hsl(
                a[0].evaluate(feature)?.number_conversion(),
                a[1].evaluate(feature)?.number_conversion(),
                a[2].evaluate(feature)?.number_conversion(),
                a[3].evaluate(feature)?.number_conversion(),
            )
        }
        _ => unreachable!("literal color name is one of color/rgb/rgba/hsl/hsla"),
    };
    Ok(Value::Cartesian4(color))
}

/// Mirrors `_evaluateLiteralVector`.
pub fn evaluate_literal_vector(
    call: &str,
    args: &[Node],
    feature: Option<&dyn ExpressionFeature>,
) -> Result<Value, RuntimeError> {
    let mut components: Vec<f64> = Vec::new();
    let args_length = args.len();
    for argument in args {
        let value = argument.evaluate(feature)?;
        match value {
            Value::Number(n) => components.push(n),
            Value::Cartesian2(v) => components.extend([v.x, v.y]),
            Value::Cartesian3(v) => components.extend([v.x, v.y, v.z]),
            Value::Cartesian4(v) => components.extend([v.x, v.y, v.z, v.w]),
            _ => {
                return Err(runtime_error(&format!(
                    "{call} argument must be a vector or number. Argument is {value}."
                )))
            }
        }
    }

    let components_length = components.len();
    let vector_length = call
        .chars()
        .nth(3)
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as usize;

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
        // Add the same component 3 more times
        let component = components[0];
        components.extend([component, component, component]);
    }

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

    #[test]
    fn to_byte_clamps_and_rounds() {
        assert_eq!(to_byte(-10.0), 0);
        assert_eq!(to_byte(300.0), 255);
        assert_eq!(to_byte(1.4), 1);
        assert_eq!(to_byte(1.6), 2);
        assert_eq!(to_byte(255.0), 255);
    }

    #[test]
    fn parse_float_js_longest_prefix() {
        assert_eq!(parse_float_js("12.5abc"), 12.5);
        assert_eq!(parse_float_js("1e3"), 1000.0);
        assert!(parse_float_js("abc").is_nan());
        assert!(parse_float_js("inf").is_nan());
    }

    #[test]
    fn named_colors_byte_exact() {
        // RED == #FF0000 == (1, 0, 0, 1)
        assert_eq!(from_css_color_string("red").unwrap(), DVec4::new(1.0, 0.0, 0.0, 1.0));
        // Case-insensitive.
        assert_eq!(from_css_color_string("RED").unwrap(), DVec4::new(1.0, 0.0, 0.0, 1.0));
        // GREY alias == GRAY == #808080
        let grey = from_css_color_string("grey").unwrap();
        assert!((grey.x - 0x80 as f64 / 255.0).abs() < 1e-12);
        // TRANSPARENT == (0,0,0,0)
        assert_eq!(
            from_css_color_string("transparent").unwrap(),
            DVec4::new(0.0, 0.0, 0.0, 0.0)
        );
        // Unknown name -> None.
        assert!(from_css_color_string("notacolor").is_none());
    }

    #[test]
    fn hex_forms() {
        // #fff == white
        assert_eq!(from_css_color_string("#fff").unwrap(), DVec4::new(1.0, 1.0, 1.0, 1.0));
        // #ff000080 -> alpha 0x80/255
        let c = from_css_color_string("#ff0000").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 1.0));
        let c8 = from_css_color_string("#ff000080").unwrap();
        assert!((c8.w - 0x80 as f64 / 255.0).abs() < 1e-12);
        // #rgba 4-digit
        let c4 = from_css_color_string("#f00f").unwrap();
        assert_eq!(c4, DVec4::new(1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn rgb_functional() {
        let c = from_css_color_string("rgb(255, 0, 0)").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 1.0));
        let c = from_css_color_string("rgba(255, 0, 0, 0.5)").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 0.5));
        let c = from_css_color_string("rgb(100%, 0%, 0%)").unwrap();
        assert_eq!(c, DVec4::new(1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn hsl_functional() {
        // hsl(0, 100%, 50%) == red
        let c = from_css_color_string("hsl(0, 100%, 50%)").unwrap();
        assert!((c.x - 1.0).abs() < 1e-12);
        assert!((c.y - 0.0).abs() < 1e-12);
        assert!((c.z - 0.0).abs() < 1e-12);
    }

    #[test]
    fn from_hsl_greyscale_when_unsaturated() {
        // saturation 0 -> all channels == lightness
        let c = from_hsl(0.0, 0.0, 0.5, 1.0);
        assert_eq!(c, DVec4::new(0.5, 0.5, 0.5, 1.0));
    }
}
