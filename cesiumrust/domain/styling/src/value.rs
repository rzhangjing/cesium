//! 3D Tiles Styling 表达式引擎的动态值、JS 强制转换/相等语义与运行时错误类型。
//!
//! 本模块定义 styling 表达式求值用到的动态值类型：
//! - `RegExpValue`：编译后的正则值（实际实现在 `regex.rs`，此处仅复用）
//! - `Value` 枚举 + impl：标量/向量/字符串/数组/正则的统一运行时表示
//! - `number_to_js_string`：以 JS 字符串插值语义格式化数字
//! - `js_parse_number`：JS `Number()` / `parseFloat` 风格的宽松数字解析
//!
//! 这些类型共同构成求值器运行时的值域，需要精确复现 JS 的强制转换与
//! 相等语义（严格 `===` 与宽松 `==` 两条路径）。
//!
//! # M7-A 作用域说明（基础层）
//!
//! * 偏离（依赖）：blueprint 从 `cesium_core` 取 `RuntimeError`、从
//!   `cesium_core` 取 `Cartesian2/3/4`。`cesium-styling` 是一个孤立的 domain
//!   crate（只有 `glam`），所以此处定义一个最小的本地 [`RuntimeError`]，
//!   向量变体由 `glam::DVec2/DVec3/DVec4`（f64，domain 精度）承载。逐分量
//!   比较 / 字符串形式完全一致。
//! * 偏离（regex → M7-B）：[`RegExpValue`] 现在位于 `regex.rs`，带一个
//!   **真正编译的** `::regex::Regex`（`regex` crate 在离线 registry 中可用）。
//!   本模块只是为 [`Value::RegExp`] 变体、它的 `String()` 形式与相等而复用它；
//!   `compile` / `test` / `exec_first_capture` 见 `regex.rs`。
//! * 新增（相对 blueprint）：[`Value::equals_loose`] 实现 JS 抽象相等（`==`）。
//!   styling 语言本身只定义 `===` / `!==`（对应求值器的严格相等分支；blueprint
//!   `equals_strict`），所以 `PartialEq` 和 `equals_strict` 保持**严格**
//!   （忠于 blueprint）；提供 `equals_loose` 是为了让完整的 JS 相等怪癖面
//!   （宽松强制转换）能在此表达并单元测试，并被 M7-B / 横切 specs 复用。

use glam::{DVec2, DVec3, DVec4};

use crate::regex::RegExpValue;

// ---------------------------------------------------------------------------
// 运行时错误（镜像 `cesium_core::RuntimeError` / JS `RuntimeError`）
// ---------------------------------------------------------------------------

/// 词法分析、解析或（M7-B 中）求值一个表达式期间抛出的运行时错误。
/// 镜像 `new RuntimeError(message)`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    /// 人类可读的错误消息文本。
    message: String,
}

impl RuntimeError {
    /// 镜像 `new RuntimeError(message)`；`None` 产生一个空消息。
    pub fn new(message: Option<&str>) -> RuntimeError {
        RuntimeError {
            message: message.unwrap_or("").to_string(),
        }
    }

    /// 错误消息文本。
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for RuntimeError {
    /// 直接把错误消息文本作为渲染结果输出。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// 便捷构造器，镜像 `new RuntimeError(message)`。
pub fn runtime_error(message: &str) -> RuntimeError {
    RuntimeError::new(Some(message))
}

// ---------------------------------------------------------------------------
// 动态值（镜像表达式语言产生的 JS 值）
// ---------------------------------------------------------------------------

/// 表达式求值产生的动态值类型，镜像 styling 语言返回的 JS 值并集。
///
/// 注：blueprint 的 `Value` **没有** `Color` 变体 —— 颜色字面量求值为
/// [`Value::Cartesian4`]（rgba）。任务简报的枚举列表提到了 `Color`；
/// 这里遵循 blueprint（事实来源）。
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Cartesian2(DVec2),
    Cartesian3(DVec3),
    Cartesian4(DVec4),
    RegExp(RegExpValue),
    Array(Vec<Value>),
}

impl Value {
    /// 除非值是 `undefined` 或 `null`，否则为 `true`。
    pub fn is_defined(&self) -> bool {
        !matches!(self, Value::Undefined | Value::Null)
    }

    /// JS 真值性。
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    /// 镜像 `Boolean(value)`。
    pub fn boolean_conversion(&self) -> bool {
        self.is_truthy()
    }

    /// 对本语言产生的那些值类型镜像 `Number(value)`。
    pub fn number_conversion(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Null => 0.0,
            Value::String(s) => js_parse_number(s),
            // undefined、向量、regex 与数组经 Number() 强制转换时都变成 NaN。
            _ => f64::NAN,
        }
    }

    /// 镜像 `String(value)`。
    pub fn string_conversion(&self) -> String {
        match self {
            Value::Undefined => "undefined".to_string(),
            Value::Null => "null".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => number_to_js_string(*n),
            Value::String(s) => s.clone(),
            Value::Cartesian2(v) => format!("({}, {})", v.x, v.y),
            // 注：blueprint（expression.rs L171）把 `"({}, {})"` 写成了
            // 三个参数（一个无法编译的笔误）；按上游 `Cartesian3.toString()`
            // == "(x, y, z)" 修正为三个占位符。
            Value::Cartesian3(v) => format!("({}, {}, {})", v.x, v.y, v.z),
            Value::Cartesian4(v) => format!("({}, {}, {}, {})", v.x, v.y, v.z, v.w),
            Value::RegExp(r) => r.to_js_string(),
            Value::Array(items) => items
                .iter()
                .map(|item| match item {
                    Value::Undefined | Value::Null => String::new(),
                    other => other.string_conversion(),
                })
                .collect::<Vec<_>>()
                .join(","),
        }
    }

    /// 镜像 `left === right`（严格相等；Cartesian 值逐分量比较，如
    /// `_evaluateEqualsStrict` 中那样）。`NaN === NaN` 为 `false`
    /// 且 `null === undefined` 为 `false`，与 JS 完全一致。
    pub fn equals_strict(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Cartesian2(a), Value::Cartesian2(b)) => a.x == b.x && a.y == b.y,
            (Value::Cartesian3(a), Value::Cartesian3(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z
            }
            (Value::Cartesian4(a), Value::Cartesian4(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z && a.w == b.w
            }
            _ => false,
        }
    }

    /// JS 抽象相等（`left == right`）。
    ///
    /// 新增（见模块文档）：styling 语言没有 `==` 运算符，所以
    /// blueprint 只定义了 [`Value::equals_strict`]。本方法为原始类型捕获
    /// ECMAScript 抽象相等比较，使宽松相等的怪癖（`"1" == 1`、`null == undefined`、
    /// `NaN != NaN`）能被表达和测试。复合值（向量 / regex / 数组）
    /// 像 [`PartialEq`] 中那样按结构比较；原始对复合为 `false`。
    pub fn equals_loose(&self, other: &Value) -> bool {
        match (self, other) {
            // null/undefined 彼此宽松相等，且与其他任何东西都不相等。
            (Value::Null, Value::Null)
            | (Value::Undefined, Value::Undefined)
            | (Value::Null, Value::Undefined)
            | (Value::Undefined, Value::Null) => true,
            (Value::Null, _) | (_, Value::Null) => false,
            (Value::Undefined, _) | (_, Value::Undefined) => false,
            // Boolean 强制转换为 Number(1/0)，然后重新比较。
            (Value::Boolean(a), _) => {
                Value::Number(if *a { 1.0 } else { 0.0 }).equals_loose(other)
            }
            (_, Value::Boolean(b)) => {
                self.equals_loose(&Value::Number(if *b { 1.0 } else { 0.0 }))
            }
            // Number 对 Number（NaN != NaN 由 f64 `==` 自然得出）。
            (Value::Number(a), Value::Number(b)) => a == b,
            // String 对 Number（任意顺序）：Number(string) == number。
            (Value::String(s), Value::Number(n)) => js_parse_number(s) == *n,
            (Value::Number(n), Value::String(s)) => *n == js_parse_number(s),
            // String 对 String。
            (Value::String(a), Value::String(b)) => a == b,
            // 复合：同类结构比较；跨类为 false。
            (Value::Cartesian2(a), Value::Cartesian2(b)) => a.x == b.x && a.y == b.y,
            (Value::Cartesian3(a), Value::Cartesian3(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z
            }
            (Value::Cartesian4(a), Value::Cartesian4(b)) => {
                a.x == b.x && a.y == b.y && a.z == b.z && a.w == b.w
            }
            (Value::RegExp(a), Value::RegExp(b)) => a.source == b.source && a.flags == b.flags,
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|(left, right)| left.equals_loose(right))
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for Value {
    /// 以 [`Value::string_conversion`] 的 JS `String()` 结果渲染。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string_conversion())
    }
}

/// 供测试和调用方使用的深度相等：标量/向量取**严格**相等语义
/// （镜像 `_evaluateEqualsStrict`），外加 regex 值的 source/flags 比较
/// 和数组的逐元素比较。
///
/// 这是忠于 blueprint 的（严格）。JS `==` 请用 [`Value::equals_loose`]。
impl PartialEq for Value {
    /// 深度严格相等：regex 比 source/flags，数组逐元素比较，其余委托 `equals_strict`。
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // RegExpValue 持有一个编译的 Regex（无 PartialEq）；按
            // source + flags 比较，与 blueprint `PartialEq for Value` 完全一致。
            (Value::RegExp(a), Value::RegExp(b)) => a.source == b.source && a.flags == b.flags,
            (Value::Array(a), Value::Array(b)) => a == b,
            _ => self.equals_strict(other),
        }
    }
}

/// 以 JS 字符串插值的方式格式化一个数字（镜像运行时错误消息里
/// 所有 `${}` 插值所采用的转换规则）。
pub fn number_to_js_string(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_string();
    }
    if number.is_infinite() {
        return if number > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    format!("{number}")
}

/// 镜像 JS `Number("...")` 字符串强制转换（去空白、十进制、指数、
/// 十六进制；空串为 `0`；其他任何都是 `NaN`）。
pub fn js_parse_number(text: &str) -> f64 {
    let trimmed = text.trim();
    // 空串（或仅空白）按 JS 语义强制转为 0。
    if trimmed.is_empty() {
        return 0.0;
    }
    // 0x/0X 前缀走十六进制解析；非法十六进制回 NaN。
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        if let Ok(value) = i64::from_str_radix(hex, 16) {
            return value as f64;
        }
        return f64::NAN;
    }
    trimmed.parse::<f64>().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nan() -> Value {
        Value::Number(f64::NAN)
    }

    // --- JS 相等怪癖（任务要求的四个） ---

    #[test]
    fn nan_is_not_equal_to_nan() {
        // 严格、宽松与 PartialEq 一致：NaN != NaN。
        assert!(!nan().equals_strict(&nan()));
        assert!(!nan().equals_loose(&nan()));
        assert!(nan() != nan());
    }

    #[test]
    fn string_number_loose_equal() {
        // "1" == 1（宽松）为真 …
        assert!(Value::String("1".to_string()).equals_loose(&Value::Number(1.0)));
        // … 但严格（===）和忠于 blueprint 的 PartialEq 为假。
        assert!(!Value::String("1".to_string()).equals_strict(&Value::Number(1.0)));
        assert!(Value::String("1".to_string()) != Value::Number(1.0));
    }

    #[test]
    fn null_loose_equals_undefined() {
        assert!(Value::Null.equals_loose(&Value::Undefined));
        assert!(Value::Undefined.equals_loose(&Value::Null));
        // 严格：null === undefined 为假。
        assert!(!Value::Null.equals_strict(&Value::Undefined));
        assert!(Value::Null != Value::Undefined);
    }

    #[test]
    fn string_number_strict_not_equal() {
        // "5" === 5 为假（严格） …
        assert!(!Value::String("5".to_string()).equals_strict(&Value::Number(5.0)));
        // … 而 "5" == 5 为真（宽松），演示 == 与 === 之别。
        assert!(Value::String("5".to_string()).equals_loose(&Value::Number(5.0)));
    }

    #[test]
    fn boolean_loose_coercion() {
        assert!(Value::Boolean(true).equals_loose(&Value::Number(1.0)));
        assert!(Value::Boolean(false).equals_loose(&Value::Number(0.0)));
        assert!(Value::Boolean(true).equals_loose(&Value::String("1".to_string())));
        assert!(!Value::Boolean(true).equals_loose(&Value::Number(2.0)));
    }

    #[test]
    fn strict_same_type_equality() {
        assert!(Value::Number(3.0).equals_strict(&Value::Number(3.0)));
        assert!(Value::String("a".to_string()).equals_strict(&Value::String("a".to_string())));
        assert!(Value::Null.equals_strict(&Value::Null));
        assert!(Value::Undefined.equals_strict(&Value::Undefined));
        assert!(!Value::Number(3.0).equals_strict(&Value::Number(4.0)));
    }

    #[test]
    fn vector_strict_componentwise() {
        let a = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let b = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let c = Value::Cartesian3(DVec3::new(1.0, 2.0, 4.0));
        assert!(a.equals_strict(&b));
        assert!(!a.equals_strict(&c));
        // NaN 分量 => 不相等。
        let n = Value::Cartesian2(DVec2::new(f64::NAN, 0.0));
        assert!(!n.equals_strict(&n));
    }

    #[test]
    fn array_and_regex_partial_eq() {
        let a1 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let a2 = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let a3 = Value::Array(vec![Value::Number(1.0)]);
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);

        let r1 = Value::RegExp(RegExpValue::compile("ab", "gi").unwrap());
        let r2 = Value::RegExp(RegExpValue::compile("ab", "gi").unwrap());
        let r3 = Value::RegExp(RegExpValue::compile("ab", "g").unwrap());
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    // --- js_parse_number（JS Number() 语义） ---

    #[test]
    fn js_parse_number_cases() {
        assert_eq!(js_parse_number(""), 0.0);
        assert_eq!(js_parse_number("   "), 0.0);
        assert_eq!(js_parse_number("  42  "), 42.0);
        assert_eq!(js_parse_number("0x10"), 16.0);
        assert_eq!(js_parse_number("0XFF"), 255.0);
        assert_eq!(js_parse_number("1.25"), 1.25);
        assert_eq!(js_parse_number("1e3"), 1000.0);
        assert!(js_parse_number("abc").is_nan());
        assert!(js_parse_number("0xZZ").is_nan());
    }

    // --- number_to_js_string ---

    #[test]
    fn number_to_js_string_cases() {
        assert_eq!(number_to_js_string(f64::NAN), "NaN");
        assert_eq!(number_to_js_string(f64::INFINITY), "Infinity");
        assert_eq!(number_to_js_string(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(number_to_js_string(5.0), "5");
        assert_eq!(number_to_js_string(-2.5), "-2.5");
    }

    // --- 转换 / 真值性 ---

    #[test]
    fn number_conversion_cases() {
        assert_eq!(Value::Null.number_conversion(), 0.0);
        assert_eq!(Value::Boolean(true).number_conversion(), 1.0);
        assert_eq!(Value::Boolean(false).number_conversion(), 0.0);
        assert_eq!(Value::String("7".to_string()).number_conversion(), 7.0);
        assert!(Value::Undefined.number_conversion().is_nan());
        assert!(Value::Cartesian2(DVec2::new(1.0, 2.0)).number_conversion().is_nan());
    }

    #[test]
    fn string_conversion_cases() {
        assert_eq!(Value::Undefined.string_conversion(), "undefined");
        assert_eq!(Value::Null.string_conversion(), "null");
        assert_eq!(Value::Boolean(true).string_conversion(), "true");
        assert_eq!(Value::Number(5.0).string_conversion(), "5");
        assert_eq!(Value::String("hi".to_string()).string_conversion(), "hi");
        assert_eq!(Value::Cartesian2(DVec2::new(1.0, 2.0)).string_conversion(), "(1, 2)");
        assert_eq!(
            Value::Cartesian4(DVec4::new(1.0, 2.0, 3.0, 4.0)).string_conversion(),
            "(1, 2, 3, 4)"
        );
        // 数组：null/undefined 渲染为空，以 "," 连接。
        let arr = Value::Array(vec![Value::Number(1.0), Value::Null, Value::Number(3.0)]);
        assert_eq!(arr.string_conversion(), "1,,3");
    }

    #[test]
    fn truthiness_cases() {
        assert!(!Value::Undefined.is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Boolean(false).is_truthy());
        assert!(Value::Boolean(true).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(!Value::Number(f64::NAN).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::String("a".to_string()).is_truthy());
        assert!(Value::Array(vec![]).is_truthy());
    }

    #[test]
    fn is_defined_cases() {
        assert!(!Value::Undefined.is_defined());
        assert!(!Value::Null.is_defined());
        assert!(Value::Number(0.0).is_defined());
        assert!(Value::String("".to_string()).is_defined());
    }

    #[test]
    fn regexp_to_js_string_sorts_flags() {
        assert_eq!(
            RegExpValue::compile("ab", "gi").unwrap().to_js_string(),
            "/ab/gi"
        );
        // flags 会被排成 `dgimsuy` 顺序。
        assert_eq!(
            RegExpValue::compile("ab", "ig").unwrap().to_js_string(),
            "/ab/gi"
        );
        assert_eq!(RegExpValue::compile("x", "").unwrap().to_js_string(), "/x/");
    }

    #[test]
    fn runtime_error_message() {
        let e = runtime_error("boom");
        assert_eq!(e.message(), "boom");
        assert_eq!(format!("{e}"), "boom");
        assert_eq!(RuntimeError::new(None).message(), "");
    }
}
