//! 3D Tiles Styling 语言实现。
//!
//! 镜像 CesiumJS：
//! - `Scene/Cesium3DTileStyle.js`
//! - `Scene/Expression.js`
//! - `Scene/ConditionsExpression.js`
//!
//! 3D Tiles Styling 语言允许基于 feature 属性定义样式：
//! ```json
//! {
//!   "color": {
//!     "conditions": [
//!       ["${Height} >= 100", "color('red')"],
//!       ["true", "color('blue')"]
//!     ]
//!   },
//!   "show": "${Height} > 0",
//!   "pointSize": 2.0
//! }
//! ```

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ── M7-D：JSEP styling 引擎 feature 门控 ────────────────────────────────────

/// 切换新 `cesium-styling` JSEP 表达式引擎的环境变量。
///
/// 镜像 M0.3 feature-flag 注册表常量 `ENV_ENABLE_STYLING_JSEP`
///（环境字符串 `CESIUM_ENABLE_STYLING_JSEP`）。本 crate 是一个 **domain** crate，
/// 不得依赖应用层的 `feature_flags` 模块（DDD 依赖方向），因此
/// truthy 读取在本地重复实现，与 M0.3 语义逐字节一致。默认 OFF → 遗留的朴素解析器。
const ENV_ENABLE_STYLING_JSEP: &str = "CESIUM_ENABLE_STYLING_JSEP";

/// M0.3 truthy token：大小写不敏感、去空白后匹配的 `1|true|yes|on`。
/// 与 `cesium-app::feature_flags::truthy` 逐字节一致。
fn truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// 当 JSEP styling 引擎门控启用时返回 `true`（默认 OFF）。
///
/// 启用时，[`Expression::parse`] 通过 `cesium_styling` 编译，且
/// 求值委托给新引擎（对引擎拒绝的形式保留遗留回退）。当禁用时，
/// 则原样使用遗留的朴素解析器。
pub fn styling_jsep_enabled() -> bool {
    match std::env::var(ENV_ENABLE_STYLING_JSEP) {
        Ok(v) => truthy(&v),
        Err(_) => false,
    }
}

/// 一个可针对一组 feature 属性求值的已解析表达式。
///
/// 映射到 CesiumJS `Scene/Expression.js`
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// 常量布尔值。
    BoolConstant(bool),
    /// 常量数值。
    NumberConstant(f64),
    /// 常量字符串值。
    StringConstant(String),
    /// 属性引用：`${propertyName}`
    PropertyRef(String),
    /// 二元运算（例如 `${Height} >= 100`）
    BinaryOp {
        /// 左操作数。
        left: Box<Expression>,
        /// 运算符。
        op: BinaryOperator,
        /// 右操作数。
        right: Box<Expression>,
    },
    /// 一元运算（例如 `!${visible}`）
    UnaryOp {
        /// 运算符。
        op: UnaryOperator,
        /// 操作数。
        operand: Box<Expression>,
    },
    /// 函数调用（例如 `color('red', 0.5)`）
    FunctionCall {
        /// 函数名。
        name: String,
        /// 参数。
        args: Vec<Expression>,
    },
    /// 颜色字面量（从 `color('name')` 或 `color(r, g, b, a)` 解析）
    ColorLiteral([f64; 4]),
    /// M7-D：由 `cesium-styling` JSEP 引擎编译的表达式。
    ///
    /// 仅当 [`styling_jsep_enabled()`] 为 true 时产生。求值委托给新引擎，
    /// 并在引擎拒绝它所建模的表达式形式时（例如带数值分量的
    /// `color(r,g,b,a)`）回退到遗留解析器。新增变体：现有消费者不会
    /// 对 [`Expression`] 做穷尽匹配。
    Jsep(JsepExpression),
}

/// 一个 JSEP 编译的表达式（M7-D 双轨包装器）。
///
/// `cesium_styling::Expression` 不携带 `Clone`/`Debug`/`PartialEq` derive，
/// 因此它被持有在一个 [`Arc`] 后面，且这三个 trait 手动实现：`Clone`
/// 共享已编译的 AST，`Debug` 仅打印源代码文本，`PartialEq` 比较源文本。这保持了
/// [`Expression`] 的 derive（以及 `TileStyle: Clone/Debug`）完整。
pub struct JsepExpression {
    /// 原始表达式源码（预 define/变量展开前），也用于在引擎于求值时
    /// 出错时通过遗留回退重新解析。
    pub source: String,
    /// 已编译的引擎表达式，克隆时廉价共享。
    compiled: Arc<cesium_styling::Expression>,
}

impl JsepExpression {
    /// 用 JSEP 引擎编译 `source`，解析失败时返回 `None`。
    ///
    /// `defines` 不会穿过 [`Expression::parse`] 的签名传递；遗留解析器同样
    /// 只存储而从不展开 define，因此传 `None` 可保持位精确的一致性。
    /// Define 展开需要 API 变更，已推迟（M7.7）。
    fn compile(source: &str) -> Option<Self> {
        cesium_styling::Expression::try_new(source, None)
            .ok()
            .map(|compiled| Self {
                source: source.to_string(),
                compiled: Arc::new(compiled),
            })
    }
}

impl Clone for JsepExpression {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            compiled: Arc::clone(&self.compiled),
        }
    }
}

impl std::fmt::Debug for JsepExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Jsep").field(&self.source).finish()
    }
}

impl PartialEq for JsepExpression {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

/// 表达式的二元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
}

/// 表达式的一元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// `!`（逻辑非）
    Not,
    /// `-`（取负）
    Negate,
}

/// 表达式求值的结果。
#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    /// 布尔结果。
    Bool(bool),
    /// 数值结果。
    Number(f64),
    /// 字符串结果。
    String(String),
    /// 颜色结果 [r, g, b, a]，范围为 0-1。
    Color([f64; 4]),
}

impl EvalResult {
    /// 转换为布尔（用于 show 表达式）。
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            // JS `Boolean(NaN)` === false；守卫 NaN 以使源自 `undefined`
            // 的 NaN（gate=1 严格强转）保持为 falsy，与上游 `show` 一致。
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::String(s) => !s.is_empty() && s != "false",
            Self::Color(_) => true,
        }
    }

    /// 转换为数值（用于 pointSize 表达式）。
    pub fn as_number(&self) -> f64 {
        match self {
            Self::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Number(n) => *n,
            Self::String(s) => s.parse().unwrap_or(0.0),
            Self::Color(c) => c[0],
        }
    }

    /// 转换为颜色（用于 color 表达式）。
    pub fn as_color(&self) -> [f64; 4] {
        match self {
            Self::Color(c) => *c,
            Self::Number(n) => [*n, *n, *n, 1.0],
            Self::Bool(b) => {
                if *b {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                }
            }
            Self::String(_) => [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl Expression {
    /// 针对一组 feature 属性求值表达式。
    pub fn evaluate(&self, properties: &HashMap<String, Value>) -> EvalResult {
        match self {
            Self::BoolConstant(b) => EvalResult::Bool(*b),
            Self::NumberConstant(n) => EvalResult::Number(*n),
            Self::StringConstant(s) => EvalResult::String(s.clone()),
            Self::PropertyRef(name) => {
                if let Some(value) = properties.get(name) {
                    json_to_eval_result(value)
                } else {
                    EvalResult::Number(0.0)
                }
            }
            Self::BinaryOp { left, op, right } => {
                let l = left.evaluate(properties);
                let r = right.evaluate(properties);
                eval_binary_op(&l, *op, &r)
            }
            Self::UnaryOp { op, operand } => {
                let v = operand.evaluate(properties);
                match op {
                    UnaryOperator::Not => EvalResult::Bool(!v.as_bool()),
                    UnaryOperator::Negate => EvalResult::Number(-v.as_number()),
                }
            }
            Self::FunctionCall { name, args } => {
                eval_function(name, args, properties)
            }
            Self::ColorLiteral(c) => EvalResult::Color(*c),
            Self::Jsep(jsep) => {
                let feature = JsonFeature(properties);
                match jsep.compiled.evaluate(Some(&feature)) {
                    Ok(value) => cesium_value_to_eval_result(&value),
                    // 引擎拒绝某些遗留接受的格式（例如带数值分量的
                    // `color(r,g,b,a)`）。在原始源码上回退到遗留解析器，以使 `evaluate`
                    // 保持全函数并与 M7-D 之前的行为位精确一致。
                    Err(_) => {
                        #[allow(deprecated)]
                        legacy_parse(&jsep.source).evaluate(properties)
                    }
                }
            }
        }
    }

    /// 从字符串解析一个表达式。
    ///
    /// M7-D 双轨分发：当 [`styling_jsep_enabled()`] 为 true 时，输入由
    /// `cesium-styling` JSEP 引擎编译并包装为 [`Expression::Jsep`]；否则
    ///（门控默认 OFF）委托给遗留的朴素解析器 [`legacy_parse`]。`parse` 保持不会失败：
    /// JSEP 编译失败会回退到遗留解析器。
    ///
    /// 支持：
    /// - 属性引用：`${Height}`
    /// - 比较：`${Height} >= 100`
    /// - 布尔字面量：`true`、`false`
    /// - 数值字面量：`2.0`、`100`
    /// - 函数调用：`color('red')`、`color(1.0, 0.0, 0.0, 1.0)`
    pub fn parse(input: &str) -> Self {
        if styling_jsep_enabled() {
            if let Some(jsep) = JsepExpression::compile(input) {
                return Self::Jsep(jsep);
            }
        }
        #[allow(deprecated)]
        legacy_parse(input)
    }
}

/// 遗留的朴素表达式解析器（M7-D 之前）。
///
/// 原样保留在 [`styling_jsep_enabled()`] 门控之后，并作为 JSEP 引擎拒绝的
/// 表达式形式（例如带数值分量的 `color(r,g,b,a)`）的回退。已知缺陷：首次
/// 匹配的运算符优先级，以及 `defines` 只存储不展开。JSEP 引擎是修正后的
/// 替代物；计定于 2026-10-15 之后移除（M7.7）。
#[deprecated(note = "use cesium_styling::Expression; remove after 2026-10-15")]
fn legacy_parse(input: &str) -> Expression {
    let input = input.trim();

    // 布尔字面量
    if input == "true" {
        return Expression::BoolConstant(true);
    }
    if input == "false" {
        return Expression::BoolConstant(false);
    }

    // 数值字面量
    if let Ok(n) = input.parse::<f64>() {
        return Expression::NumberConstant(n);
    }

    // 字符串字面量
    if (input.starts_with('\'') && input.ends_with('\''))
        || (input.starts_with('"') && input.ends_with('"'))
    {
        return Expression::StringConstant(input[1..input.len() - 1].to_string());
    }

    // 属性引用（仅当整个字符串是一个单一属性引用时）
    if input.starts_with("${") && input.ends_with('}') {
        // 检查这是否是一个单一属性引用（闭合的 } 之后没有其他内容）
        let inner = &input[2..input.len() - 1];
        // 确保内部没有嵌套的 ${ 或 }
        if !inner.contains("${") && !inner.contains('}') {
            return Expression::PropertyRef(inner.to_string());
        }
    }

    // 函数调用：color(...)、rgb(...) 等。
    if let Some(paren_start) = input.find('(') {
        if input.ends_with(')') {
            let func_name = input[..paren_start].trim();
            let args_str = &input[paren_start + 1..input.len() - 1];
            #[allow(deprecated)]
            let args = parse_function_args(args_str);
            return Expression::FunctionCall {
                name: func_name.to_string(),
                args,
            };
        }
    }

    // 二元运算（对常见情况的简单解析）
    // 先尝试比较运算符
    for op_str in [">=", "<=", "!=", "==", ">", "<"] {
        if let Some(pos) = find_operator(input, op_str) {
            let left_str = input[..pos].trim();
            let right_str = input[pos + op_str.len()..].trim();
            let left = legacy_parse(left_str);
            let right = legacy_parse(right_str);
            let op = match op_str {
                ">=" => BinaryOperator::Ge,
                "<=" => BinaryOperator::Le,
                "!=" => BinaryOperator::Ne,
                "==" => BinaryOperator::Eq,
                ">" => BinaryOperator::Gt,
                "<" => BinaryOperator::Lt,
                _ => unreachable!(),
            };
            return Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
    }

    // 算术运算符
    for op_str in ["+", "-", "*", "/"] {
        if let Some(pos) = find_operator(input, op_str) {
            let left_str = input[..pos].trim();
            let right_str = input[pos + op_str.len()..].trim();
            if !left_str.is_empty() && !right_str.is_empty() {
                let left = legacy_parse(left_str);
                let right = legacy_parse(right_str);
                let op = match op_str {
                    "+" => BinaryOperator::Add,
                    "-" => BinaryOperator::Sub,
                    "*" => BinaryOperator::Mul,
                    "/" => BinaryOperator::Div,
                    _ => unreachable!(),
                };
                return Expression::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            }
        }
    }

    // 逻辑运算符
    for op_str in ["&&", "||"] {
        if let Some(pos) = input.find(op_str) {
            let left_str = input[..pos].trim();
            let right_str = input[pos + op_str.len()..].trim();
            let left = legacy_parse(left_str);
            let right = legacy_parse(right_str);
            let op = if op_str == "&&" {
                BinaryOperator::And
            } else {
                BinaryOperator::Or
            };
            return Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
    }

    // 一元非
    if let Some(stripped) = input.strip_prefix('!') {
        let operand = legacy_parse(stripped);
        return Expression::UnaryOp {
            op: UnaryOperator::Not,
            operand: Box::new(operand),
        };
    }

    // 回退：视为字符串
    Expression::StringConstant(input.to_string())
}

/// 查找一个运算符位置，避免匹配 `${...}` 或引号内的内容。
fn find_operator(input: &str, op: &str) -> Option<usize> {
    let mut in_property = false;
    let mut in_quote = false;
    let mut quote_char = ' ';
    let chars: Vec<char> = input.chars().collect();
    let op_chars: Vec<char> = op.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        if in_quote {
            if c == quote_char {
                in_quote = false;
            }
            i += 1;
            continue;
        }

        if c == '\'' || c == '"' {
            in_quote = true;
            quote_char = c;
            i += 1;
            continue;
        }

        if c == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            in_property = true;
            i += 2;
            continue;
        }

        if in_property {
            if c == '}' {
                in_property = false;
            }
            i += 1;
            continue;
        }

        // 检查运算符匹配
        if i + op_chars.len() <= chars.len() {
            let matches = op_chars
                .iter()
                .enumerate()
                .all(|(j, &oc)| chars[i + j] == oc);
            if matches {
                return Some(i);
            }
        }

        i += 1;
    }

    None
}

/// 从逗号分隔的字符串解析函数参数。
///
/// 仅遗留的辅助函数：递归始终落在 [`legacy_parse`] 上，因此回退路径
/// 永不会混入 JSEP 编译的子表达式。
#[deprecated(note = "use cesium_styling::Expression; remove after 2026-10-15")]
#[allow(deprecated)]
fn parse_function_args(args_str: &str) -> Vec<Expression> {
    let args_str = args_str.trim();
    if args_str.is_empty() {
        return vec![];
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quote = false;
    let mut quote_char = ' ';

    for c in args_str.chars() {
        if in_quote {
            current.push(c);
            if c == quote_char {
                in_quote = false;
            }
            continue;
        }

        match c {
            '\'' | '"' => {
                in_quote = true;
                quote_char = c;
                current.push(c);
            }
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                args.push(legacy_parse(current.trim()));
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        args.push(legacy_parse(current.trim()));
    }

    args
}

/// 求值一个二元运算。
fn eval_binary_op(left: &EvalResult, op: BinaryOperator, right: &EvalResult) -> EvalResult {
    match op {
        BinaryOperator::Add => EvalResult::Number(left.as_number() + right.as_number()),
        BinaryOperator::Sub => EvalResult::Number(left.as_number() - right.as_number()),
        BinaryOperator::Mul => EvalResult::Number(left.as_number() * right.as_number()),
        BinaryOperator::Div => {
            let r = right.as_number();
            if r == 0.0 {
                EvalResult::Number(0.0)
            } else {
                EvalResult::Number(left.as_number() / r)
            }
        }
        BinaryOperator::Mod => {
            let r = right.as_number();
            if r == 0.0 {
                EvalResult::Number(0.0)
            } else {
                EvalResult::Number(left.as_number() % r)
            }
        }
        BinaryOperator::Eq => {
            EvalResult::Bool((left.as_number() - right.as_number()).abs() < f64::EPSILON)
        }
        BinaryOperator::Ne => {
            EvalResult::Bool((left.as_number() - right.as_number()).abs() >= f64::EPSILON)
        }
        BinaryOperator::Lt => EvalResult::Bool(left.as_number() < right.as_number()),
        BinaryOperator::Le => EvalResult::Bool(left.as_number() <= right.as_number()),
        BinaryOperator::Gt => EvalResult::Bool(left.as_number() > right.as_number()),
        BinaryOperator::Ge => EvalResult::Bool(left.as_number() >= right.as_number()),
        BinaryOperator::And => EvalResult::Bool(left.as_bool() && right.as_bool()),
        BinaryOperator::Or => EvalResult::Bool(left.as_bool() || right.as_bool()),
    }
}

/// 求值一个函数调用。
fn eval_function(
    name: &str,
    args: &[Expression],
    properties: &HashMap<String, Value>,
) -> EvalResult {
    match name {
        "color" => {
            if args.is_empty() {
                return EvalResult::Color([1.0, 1.0, 1.0, 1.0]);
            }

            // color('name') 或 color('name', alpha)
            if let Expression::StringConstant(color_name) = &args[0] {
                let base_color = parse_color_name(color_name);
                let alpha = if args.len() > 1 {
                    args[1].evaluate(properties).as_number()
                } else {
                    1.0
                };
                return EvalResult::Color([base_color[0], base_color[1], base_color[2], alpha]);
            }

            // color(r, g, b) 或 color(r, g, b, a)
            if args.len() >= 3 {
                let r = args[0].evaluate(properties).as_number();
                let g = args[1].evaluate(properties).as_number();
                let b = args[2].evaluate(properties).as_number();
                let a = if args.len() > 3 {
                    args[3].evaluate(properties).as_number()
                } else {
                    1.0
                };
                return EvalResult::Color([r, g, b, a]);
            }

            EvalResult::Color([1.0, 1.0, 1.0, 1.0])
        }
        "rgb" => {
            if args.len() >= 3 {
                let r = args[0].evaluate(properties).as_number() / 255.0;
                let g = args[1].evaluate(properties).as_number() / 255.0;
                let b = args[2].evaluate(properties).as_number() / 255.0;
                return EvalResult::Color([r, g, b, 1.0]);
            }
            EvalResult::Color([1.0, 1.0, 1.0, 1.0])
        }
        "rgba" => {
            if args.len() >= 4 {
                let r = args[0].evaluate(properties).as_number() / 255.0;
                let g = args[1].evaluate(properties).as_number() / 255.0;
                let b = args[2].evaluate(properties).as_number() / 255.0;
                let a = args[3].evaluate(properties).as_number();
                return EvalResult::Color([r, g, b, a]);
            }
            EvalResult::Color([1.0, 1.0, 1.0, 1.0])
        }
        "vec4" => {
            // vec4(value) -> 灰度颜色
            if !args.is_empty() {
                let v = args[0].evaluate(properties).as_number();
                return EvalResult::Color([v, v, v, 1.0]);
            }
            EvalResult::Color([0.0, 0.0, 0.0, 1.0])
        }
        "abs" => {
            if !args.is_empty() {
                let v = args[0].evaluate(properties).as_number();
                return EvalResult::Number(v.abs());
            }
            EvalResult::Number(0.0)
        }
        "sqrt" => {
            if !args.is_empty() {
                let v = args[0].evaluate(properties).as_number();
                return EvalResult::Number(v.sqrt());
            }
            EvalResult::Number(0.0)
        }
        "min" => {
            if args.len() >= 2 {
                let a = args[0].evaluate(properties).as_number();
                let b = args[1].evaluate(properties).as_number();
                return EvalResult::Number(a.min(b));
            }
            EvalResult::Number(0.0)
        }
        "max" => {
            if args.len() >= 2 {
                let a = args[0].evaluate(properties).as_number();
                let b = args[1].evaluate(properties).as_number();
                return EvalResult::Number(a.max(b));
            }
            EvalResult::Number(0.0)
        }
        "clamp" => {
            if args.len() >= 3 {
                let v = args[0].evaluate(properties).as_number();
                let min = args[1].evaluate(properties).as_number();
                let max = args[2].evaluate(properties).as_number();
                // 当 `min > max` 或遇到 NaN 时 `f64::clamp` 会 panic（对攻击者
                // 提供的 styling 而言是一个 DoS 向量）。使用不会 panic 的条件
                // 形式；NaN 会直接落到 `v`。
                let clamped = if v < min { min } else if v > max { max } else { v };
                return EvalResult::Number(clamped);
            }
            EvalResult::Number(0.0)
        }
        // 三角函数
        "cos" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.cos())
        }
        "sin" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.sin())
        }
        "tan" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.tan())
        }
        "acos" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.acos())
        }
        "asin" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.asin())
        }
        "atan" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.atan())
        }
        "atan2" => {
            if args.len() >= 2 {
                let y = args[0].evaluate(properties).as_number();
                let x = args[1].evaluate(properties).as_number();
                return EvalResult::Number(y.atan2(x));
            }
            EvalResult::Number(0.0)
        }
        // 角度转换
        "radians" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.to_radians())
        }
        "degrees" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.to_degrees())
        }
        // 取整 / 符号
        "sign" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            // `else { v }`（而非 `0.0`）：对 NaN 和 ±0 返回原始值，
            // 与 CesiumMath.sign 一致。
            let s = if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { v };
            EvalResult::Number(s)
        }
        "floor" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.floor())
        }
        "ceil" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.ceil())
        }
        "round" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            // JS Math.round 是向 +∞ 方向取半（Math.round(-0.5) === 0），而
            // `f64::round` 是远离零取半。使用 `(v + 0.5).floor()`。
            EvalResult::Number((v + 0.5).floor())
        }
        "fract" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v - v.floor())
        }
        // 指数 / 对数
        "exp" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.exp())
        }
        "exp2" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.exp2())
        }
        "log" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.ln())
        }
        "log2" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.log2())
        }
        "pow" => {
            if args.len() >= 2 {
                let base = args[0].evaluate(properties).as_number();
                let exp = args[1].evaluate(properties).as_number();
                return EvalResult::Number(base.powf(exp));
            }
            EvalResult::Number(0.0)
        }
        "mod" => {
            if args.len() >= 2 {
                let a = args[0].evaluate(properties).as_number();
                let b = args[1].evaluate(properties).as_number();
                return EvalResult::Number(if b != 0.0 { a % b } else { 0.0 });
            }
            EvalResult::Number(0.0)
        }
        // 插值
        "mix" => {
            if args.len() >= 3 {
                let a = args[0].evaluate(properties).as_number();
                let b = args[1].evaluate(properties).as_number();
                let t = args[2].evaluate(properties).as_number();
                return EvalResult::Number(a * (1.0 - t) + b * t);
            }
            EvalResult::Number(0.0)
        }
        // HSL 颜色
        "hsl" => {
            if args.len() >= 3 {
                let h = args[0].evaluate(properties).as_number();
                let s = args[1].evaluate(properties).as_number();
                let l = args[2].evaluate(properties).as_number();
                let rgb = hsl_to_rgb(h, s, l);
                return EvalResult::Color([rgb[0], rgb[1], rgb[2], 1.0]);
            }
            EvalResult::Color([1.0, 1.0, 1.0, 1.0])
        }
        "hsla" => {
            if args.len() >= 4 {
                let h = args[0].evaluate(properties).as_number();
                let s = args[1].evaluate(properties).as_number();
                let l = args[2].evaluate(properties).as_number();
                let a = args[3].evaluate(properties).as_number();
                let rgb = hsl_to_rgb(h, s, l);
                return EvalResult::Color([rgb[0], rgb[1], rgb[2], a]);
            }
            EvalResult::Color([1.0, 1.0, 1.0, 1.0])
        }
        _ => EvalResult::Number(0.0),
    }
}

/// 将一个 CSS 颜色名解析为 [r, g, b]，范围为 0-1。
fn parse_color_name(name: &str) -> [f64; 3] {
    match name.to_lowercase().as_str() {
        "red" => [1.0, 0.0, 0.0],
        "green" => [0.0, 0.502, 0.0],
        "blue" => [0.0, 0.0, 1.0],
        "white" => [1.0, 1.0, 1.0],
        "black" => [0.0, 0.0, 0.0],
        "yellow" => [1.0, 1.0, 0.0],
        "cyan" => [0.0, 1.0, 1.0],
        "magenta" => [1.0, 0.0, 1.0],
        "orange" => [1.0, 0.647, 0.0],
        "purple" => [0.502, 0.0, 0.502],
        "pink" => [1.0, 0.753, 0.796],
        "gray" | "grey" => [0.502, 0.502, 0.502],
        "lime" => [0.0, 1.0, 0.0],
        "navy" => [0.0, 0.0, 0.502],
        "teal" => [0.0, 0.502, 0.502],
        "maroon" => [0.502, 0.0, 0.0],
        "olive" => [0.502, 0.502, 0.0],
        "aqua" => [0.0, 1.0, 1.0],
        "silver" => [0.753, 0.753, 0.753],
        _ => [1.0, 1.0, 1.0], // 默认为白色
    }
}

/// 将 HSL 转换为 RGB。h 范围为 [0,360]，s 范围为 [0,1]，l 范围为 [0,1]。
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    [r1 + m, g1 + m, b1 + m]
}

/// 将一个 JSON 值转换为 EvalResult。
fn json_to_eval_result(value: &Value) -> EvalResult {
    match value {
        Value::Bool(b) => EvalResult::Bool(*b),
        Value::Number(n) => EvalResult::Number(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => EvalResult::String(s.clone()),
        Value::Array(arr) => {
            if arr.len() >= 4 {
                let r = arr[0].as_f64().unwrap_or(0.0);
                let g = arr[1].as_f64().unwrap_or(0.0);
                let b = arr[2].as_f64().unwrap_or(0.0);
                let a = arr[3].as_f64().unwrap_or(1.0);
                EvalResult::Color([r, g, b, a])
            } else if arr.len() == 3 {
                let r = arr[0].as_f64().unwrap_or(0.0);
                let g = arr[1].as_f64().unwrap_or(0.0);
                let b = arr[2].as_f64().unwrap_or(0.0);
                EvalResult::Color([r, g, b, 1.0])
            } else {
                EvalResult::Number(0.0)
            }
        }
        _ => EvalResult::Number(0.0),
    }
}

// ── M7-D：JSEP 引擎 bridge（feature 适配器 + 值转换器）────────────────

/// 适配器：将 `&HashMap<String, serde_json::Value>` 暴露为一个
/// [`cesium_styling::ExpressionFeature`]，以便 JSEP 引擎在求值期间
/// 能读取 feature 属性。
struct JsonFeature<'a>(&'a HashMap<String, Value>);

impl cesium_styling::ExpressionFeature for JsonFeature<'_> {
    fn get_property_inherited(&self, name: &str) -> Option<cesium_styling::Value> {
        self.0.get(name).map(json_to_cesium_value)
    }
}

/// 将一个 `serde_json::Value` feature 属性转换为供 JSEP 引擎使用的
/// [`cesium_styling::Value`]。
fn json_to_cesium_value(value: &Value) -> cesium_styling::Value {
    match value {
        Value::Null => cesium_styling::Value::Null,
        Value::Bool(b) => cesium_styling::Value::Boolean(*b),
        Value::Number(n) => cesium_styling::Value::Number(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => cesium_styling::Value::String(s.clone()),
        Value::Array(arr) => {
            cesium_styling::Value::Array(arr.iter().map(json_to_cesium_value).collect())
        }
        Value::Object(_) => cesium_styling::Value::Undefined,
    }
}

/// 将一个 [`cesium_styling::Value`] 求值结果转换为 [`EvalResult`]。
///
/// Gate=1（JSEP）严格对齐上游的强转：`Undefined` -> `Number(NaN)` 且
/// `Null` -> `Number(0.0)`，镜像 JS 的 `Number(undefined)` / `Number(null)`。
/// 这使属性缺失时的 `${x} + 1` 保持为 `NaN` 而非 `1.0`。遗留的
/// “缺失属性 -> 0.0”回退仅在 gate=0 轨道上保留
///（[`Expression::PropertyRef`] / `json_to_eval_result`）。
fn cesium_value_to_eval_result(value: &cesium_styling::Value) -> EvalResult {
    use cesium_styling::Value as CV;
    match value {
        CV::Undefined => EvalResult::Number(f64::NAN),
        CV::Null => EvalResult::Number(0.0),
        CV::Boolean(b) => EvalResult::Bool(*b),
        CV::Number(n) => EvalResult::Number(*n),
        CV::String(s) => EvalResult::String(s.clone()),
        CV::Cartesian4(v) => EvalResult::Color([v.x, v.y, v.z, v.w]),
        CV::Cartesian3(v) => EvalResult::Color([v.x, v.y, v.z, 1.0]),
        CV::Cartesian2(v) => EvalResult::Color([v.x, v.y, 0.0, 1.0]),
        // RegExp / Array 结果没有对应的 EvalResult；匹配遗留的
        // 安全默认值（参见 `json_to_eval_result`）。
        CV::RegExp(_) | CV::Array(_) => EvalResult::Number(0.0),
    }
}

/// conditions 表达式中的一个条件：[condition, result]。
#[derive(Debug, Clone)]
pub struct Condition {
    /// 条件表达式（求值为布尔）。
    pub condition: Expression,
    /// 结果表达式（当条件为真时求值）。
    pub result: Expression,
}

/// 一个 conditions 表达式：[condition, result] 对的列表。
///
/// 映射到 CesiumJS `Scene/ConditionsExpression.js`
///
/// 第一个求值为真的条件决定结果。
#[derive(Debug, Clone)]
pub struct ConditionsExpression {
    /// 按顺序排列的条件。
    pub conditions: Vec<Condition>,
}

impl ConditionsExpression {
    /// 从 JSON 解析一个 conditions 表达式。
    ///
    /// 预期格式：
    /// ```json
    /// {
    ///   "conditions": [
    ///     ["${Height} >= 100", "color('red')"],
    ///     ["true", "color('blue')"]
    ///   ]
    /// }
    /// ```
    pub fn from_json(json: &Value) -> Option<Self> {
        let conditions_arr = json.get("conditions")?.as_array()?;
        let mut conditions = Vec::new();

        for cond_pair in conditions_arr {
            let pair = cond_pair.as_array()?;
            if pair.len() >= 2 {
                let condition_str = pair[0].as_str()?;
                let result_str = pair[1].as_str()?;
                conditions.push(Condition {
                    condition: Expression::parse(condition_str),
                    result: Expression::parse(result_str),
                });
            }
        }

        Some(Self { conditions })
    }

    /// 针对一组 feature 属性求值条件。
    ///
    /// 返回第一个求值为真的条件的结果。
    pub fn evaluate(&self, properties: &HashMap<String, Value>) -> EvalResult {
        for cond in &self.conditions {
            let cond_result = cond.condition.evaluate(properties);
            if cond_result.as_bool() {
                return cond.result.evaluate(properties);
            }
        }
        // 默认：对 color 表达式返回白色，对 show 返回 true
        EvalResult::Color([1.0, 1.0, 1.0, 1.0])
    }
}

/// 一个样式表达式，可以是简单表达式或 conditions。
#[derive(Debug, Clone)]
pub enum StyleExpression {
    /// 一个简单表达式。
    Simple(Expression),
    /// 一个 conditions 表达式。
    Conditions(ConditionsExpression),
}

impl StyleExpression {
    /// 从 JSON 解析一个样式表达式。
    pub fn from_json(json: &Value) -> Option<Self> {
        match json {
            Value::String(s) => Some(Self::Simple(Expression::parse(s))),
            Value::Bool(b) => Some(Self::Simple(Expression::BoolConstant(*b))),
            Value::Number(n) => {
                Some(Self::Simple(Expression::NumberConstant(n.as_f64()?)))
            }
            Value::Object(_) => {
                if json.get("conditions").is_some() {
                    ConditionsExpression::from_json(json).map(Self::Conditions)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 针对一组 feature 属性求值表达式。
    pub fn evaluate(&self, properties: &HashMap<String, Value>) -> EvalResult {
        match self {
            Self::Simple(expr) => expr.evaluate(properties),
            Self::Conditions(conds) => conds.evaluate(properties),
        }
    }
}

/// 一个 3D Tiles 样式定义。
///
/// 映射到 CesiumJS `Scene/Cesium3DTileStyle.js`
#[derive(Debug, Clone, Default)]
pub struct TileStyle {
    /// show 表达式（决定可见性）。
    pub show: Option<StyleExpression>,
    /// color 表达式。
    pub color: Option<StyleExpression>,
    /// point size 表达式（用于点云）。
    pub point_size: Option<StyleExpression>,
    /// 点轮廓颜色表达式。
    pub point_outline_color: Option<StyleExpression>,
    /// 点轮廓宽度表达式。
    pub point_outline_width: Option<StyleExpression>,
    /// 标签文本表达式。
    pub label_text: Option<StyleExpression>,
    /// 标签颜色表达式。
    pub label_color: Option<StyleExpression>,
    /// Meta 表达式（用于 feature 元数据）。
    pub meta: HashMap<String, StyleExpression>,
    /// Defines（可复用的表达式）。
    pub defines: HashMap<String, String>,
}

impl TileStyle {
    /// 从 JSON 解析一个样式。
    pub fn from_json(json: &Value) -> Self {
        let mut style = Self::default();

        if let Some(show) = json.get("show") {
            style.show = StyleExpression::from_json(show);
        }
        if let Some(color) = json.get("color") {
            style.color = StyleExpression::from_json(color);
        }
        if let Some(point_size) = json.get("pointSize") {
            style.point_size = StyleExpression::from_json(point_size);
        }
        if let Some(poc) = json.get("pointOutlineColor") {
            style.point_outline_color = StyleExpression::from_json(poc);
        }
        if let Some(pow) = json.get("pointOutlineWidth") {
            style.point_outline_width = StyleExpression::from_json(pow);
        }
        if let Some(lt) = json.get("labelText") {
            style.label_text = StyleExpression::from_json(lt);
        }
        if let Some(lc) = json.get("labelColor") {
            style.label_color = StyleExpression::from_json(lc);
        }

        // 解析 meta
        if let Some(meta_obj) = json.get("meta").and_then(|m| m.as_object()) {
            for (key, value) in meta_obj {
                if let Some(expr) = StyleExpression::from_json(value) {
                    style.meta.insert(key.clone(), expr);
                }
            }
        }

        // 解析 defines
        if let Some(defines_obj) = json.get("defines").and_then(|d| d.as_object()) {
            for (key, value) in defines_obj {
                if let Some(s) = value.as_str() {
                    style.defines.insert(key.clone(), s.to_string());
                }
            }
        }

        style
    }

    /// 针对一个 feature 求值 show 表达式。
    pub fn evaluate_show(&self, properties: &HashMap<String, Value>) -> bool {
        match &self.show {
            Some(expr) => expr.evaluate(properties).as_bool(),
            None => true, // 默认：全部显示
        }
    }

    /// 针对一个 feature 求值 color 表达式。
    pub fn evaluate_color(&self, properties: &HashMap<String, Value>) -> [f64; 4] {
        match &self.color {
            Some(expr) => expr.evaluate(properties).as_color(),
            None => [1.0, 1.0, 1.0, 1.0], // 默认：白色
        }
    }

    /// 针对一个 feature 求值 point size 表达式。
    pub fn evaluate_point_size(&self, properties: &HashMap<String, Value>) -> f64 {
        match &self.point_size {
            Some(expr) => expr.evaluate(properties).as_number(),
            None => 1.0, // 默认：1.0
        }
    }

    /// 针对一个 feature 求值一个 meta 表达式。
    pub fn evaluate_meta(
        &self,
        key: &str,
        properties: &HashMap<String, Value>,
    ) -> Option<EvalResult> {
        self.meta.get(key).map(|expr| expr.evaluate(properties))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_props(pairs: Vec<(&str, Value)>) -> HashMap<String, Value> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn test_expression_bool_constant() {
        let expr = Expression::parse("true");
        let props = make_props(vec![]);
        assert_eq!(expr.evaluate(&props), EvalResult::Bool(true));
    }

    #[test]
    fn test_expression_number_constant() {
        let expr = Expression::parse("42.5");
        let props = make_props(vec![]);
        assert_eq!(expr.evaluate(&props), EvalResult::Number(42.5));
    }

    #[test]
    fn test_expression_property_ref() {
        let expr = Expression::parse("${Height}");
        let props = make_props(vec![("Height", json!(100.0))]);
        assert_eq!(expr.evaluate(&props), EvalResult::Number(100.0));
    }

    #[test]
    fn test_expression_comparison_ge() {
        let expr = Expression::parse("${Height} >= 100");
        let props_high = make_props(vec![("Height", json!(150.0))]);
        let props_low = make_props(vec![("Height", json!(50.0))]);

        assert_eq!(expr.evaluate(&props_high), EvalResult::Bool(true));
        assert_eq!(expr.evaluate(&props_low), EvalResult::Bool(false));
    }

    #[test]
    fn test_expression_comparison_lt() {
        let expr = Expression::parse("${Height} < 100");
        let props = make_props(vec![("Height", json!(50.0))]);
        assert_eq!(expr.evaluate(&props), EvalResult::Bool(true));
    }

    #[test]
    fn test_expression_arithmetic() {
        let expr = Expression::parse("${Height} * 2.0");
        let props = make_props(vec![("Height", json!(50.0))]);
        assert_eq!(expr.evaluate(&props), EvalResult::Number(100.0));
    }

    #[test]
    fn test_expression_color_function() {
        let expr = Expression::parse("color('red')");
        let props = make_props(vec![]);
        assert_eq!(expr.evaluate(&props), EvalResult::Color([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn test_expression_color_with_alpha() {
        let expr = Expression::parse("color('blue', 0.5)");
        let props = make_props(vec![]);
        assert_eq!(expr.evaluate(&props), EvalResult::Color([0.0, 0.0, 1.0, 0.5]));
    }

    #[test]
    fn test_expression_color_rgba() {
        let expr = Expression::parse("color(1.0, 0.5, 0.0, 1.0)");
        let props = make_props(vec![]);
        assert_eq!(expr.evaluate(&props), EvalResult::Color([1.0, 0.5, 0.0, 1.0]));
    }

    #[test]
    fn test_expression_rgb_function() {
        let expr = Expression::parse("rgb(255, 128, 0)");
        let props = make_props(vec![]);
        let result = expr.evaluate(&props);
        if let EvalResult::Color(c) = result {
            assert!((c[0] - 1.0).abs() < 0.01);
            assert!((c[1] - 0.502).abs() < 0.01);
            assert!((c[2] - 0.0).abs() < 0.01);
        } else {
            panic!("Expected color");
        }
    }

    #[test]
    fn test_conditions_expression() {
        let json = json!({
            "conditions": [
                ["${Height} >= 100", "color('red')"],
                ["${Height} >= 50", "color('yellow')"],
                ["true", "color('blue')"]
            ]
        });

        let conds = ConditionsExpression::from_json(&json).unwrap();

        let props_high = make_props(vec![("Height", json!(150.0))]);
        let props_mid = make_props(vec![("Height", json!(75.0))]);
        let props_low = make_props(vec![("Height", json!(25.0))]);

        assert_eq!(conds.evaluate(&props_high), EvalResult::Color([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(conds.evaluate(&props_mid), EvalResult::Color([1.0, 1.0, 0.0, 1.0]));
        assert_eq!(conds.evaluate(&props_low), EvalResult::Color([0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn test_tile_style_from_json() {
        let json = json!({
            "color": {
                "conditions": [
                    ["${Height} >= 100", "color('purple', 0.5)"],
                    ["${Height} >= 50", "color('red')"],
                    ["true", "color('blue')"]
                ]
            },
            "show": "${Height} > 0",
            "pointSize": 2.0
        });

        let style = TileStyle::from_json(&json);

        let props_visible = make_props(vec![("Height", json!(150.0))]);
        let props_hidden = make_props(vec![("Height", json!(0.0))]);

        assert!(style.evaluate_show(&props_visible));
        assert!(!style.evaluate_show(&props_hidden));

        let color = style.evaluate_color(&props_visible);
        // purple = rgb(128,0,128)。遗留颜色表舍入到 0.502，而 JSEP
        // 引擎产生精确的 128/255 = 0.50196（忠实于上游）。基于
        // 容差的断言同时接受两种双轨结果，与本模块中其他颜色
        // 测试一致。
        assert!((color[0] - 0.502).abs() < 1e-3, "r={}", color[0]);
        assert!(color[1].abs() < 1e-3, "g={}", color[1]);
        assert!((color[2] - 0.502).abs() < 1e-3, "b={}", color[2]);
        assert!((color[3] - 0.5).abs() < 1e-3, "a={}", color[3]); // 带 alpha 的 purple

        assert_eq!(style.evaluate_point_size(&props_visible), 2.0);
    }

    #[test]
    fn test_tile_style_meta() {
        let json = json!({
            "meta": {
                "description": "'Building height: ${Height}'"
            }
        });

        let style = TileStyle::from_json(&json);
        let props = make_props(vec![("Height", json!(100.0))]);

        // 注意：字符串拼接尚未完全实现，
        // 但 meta 表达式应当可解析
        let result = style.evaluate_meta("description", &props);
        assert!(result.is_some());
    }

    #[test]
    fn test_eval_result_conversions() {
        assert!(EvalResult::Bool(true).as_bool());
        assert!(!EvalResult::Bool(false).as_bool());
        assert!(EvalResult::Number(1.0).as_bool());
        assert!(!EvalResult::Number(0.0).as_bool());

        assert_eq!(EvalResult::Number(42.0).as_number(), 42.0);
        assert_eq!(EvalResult::Bool(true).as_number(), 1.0);

        assert_eq!(EvalResult::Color([0.5, 0.5, 0.5, 1.0]).as_color(), [0.5, 0.5, 0.5, 1.0]);
    }

    #[test]
    fn test_expression_unary_not() {
        let expr = Expression::parse("!${visible}");
        let props_true = make_props(vec![("visible", json!(true))]);
        let props_false = make_props(vec![("visible", json!(false))]);

        assert_eq!(expr.evaluate(&props_true), EvalResult::Bool(false));
        assert_eq!(expr.evaluate(&props_false), EvalResult::Bool(true));
    }

    #[test]
    fn test_expression_logical_and() {
        let expr = Expression::parse("${A} && ${B}");
        let props = make_props(vec![("A", json!(true)), ("B", json!(true))]);
        assert_eq!(expr.evaluate(&props), EvalResult::Bool(true));

        let props2 = make_props(vec![("A", json!(true)), ("B", json!(false))]);
        assert_eq!(expr.evaluate(&props2), EvalResult::Bool(false));
    }

    #[test]
    fn test_math_functions() {
        let props = make_props(vec![("Value", json!(-5.0))]);

        let abs_expr = Expression::parse("abs(${Value})");
        assert_eq!(abs_expr.evaluate(&props), EvalResult::Number(5.0));

        let sqrt_expr = Expression::parse("sqrt(16.0)");
        assert_eq!(sqrt_expr.evaluate(&props), EvalResult::Number(4.0));

        let clamp_expr = Expression::parse("clamp(${Value}, 0.0, 10.0)");
        assert_eq!(clamp_expr.evaluate(&props), EvalResult::Number(0.0));
    }

    #[test]
    fn test_style_expression_simple() {
        let json = json!("${Height} > 50");
        let expr = StyleExpression::from_json(&json).unwrap();
        let props = make_props(vec![("Height", json!(100.0))]);
        assert!(expr.evaluate(&props).as_bool());
    }

    #[test]
    fn test_style_expression_conditions() {
        let json = json!({
            "conditions": [
                ["${Type} == 1", "color('red')"],
                ["true", "color('white')"]
            ]
        });
        let expr = StyleExpression::from_json(&json).unwrap();
        let props = make_props(vec![("Type", json!(1.0))]);
        assert_eq!(expr.evaluate(&props), EvalResult::Color([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn test_color_names() {
        assert_eq!(parse_color_name("red"), [1.0, 0.0, 0.0]);
        assert_eq!(parse_color_name("blue"), [0.0, 0.0, 1.0]);
        assert_eq!(parse_color_name("white"), [1.0, 1.0, 1.0]);
        assert_eq!(parse_color_name("black"), [0.0, 0.0, 0.0]);
    }

    // ── M7-D：gate 辅助函数 + JSEP bridge ──────────────────────────────

    #[test]
    fn test_truthy_tokens() {
        // M0.3 语义：大小写不敏感、去空白的 1|true|yes|on。
        for t in ["1", "true", "TRUE", "True", "yes", "YES", "on", " on ", "\ttrue\n"] {
            assert!(truthy(t), "expected truthy: {t:?}");
        }
        for f in ["0", "false", "no", "off", "", "  ", "maybe", "2", "truex"] {
            assert!(!truthy(f), "expected falsy: {f:?}");
        }
    }

    #[test]
    fn test_styling_jsep_gate_is_bool() {
        // 仅断言访问器是全函数；具体值取决于环境（默认 OFF）。
        // 避免修改环境变量以保持测试的确定性。
        let _ = styling_jsep_enabled();
    }

    /// 直接构造一个由 JSEP 支撑的表达式（与 gate 无关），以便在不修改
    /// 进程 env 的情况下确定性地调用 bridge。
    fn jsep(src: &str) -> Expression {
        Expression::Jsep(JsepExpression::compile(src).expect("jsep compile"))
    }

    #[test]
    fn test_jsep_bridge_arithmetic() {
        let expr = jsep("${a} + ${b}");
        let props = make_props(vec![("a", json!(10.0)), ("b", json!(5.0))]);
        assert_eq!(expr.evaluate(&props), EvalResult::Number(15.0));
    }

    #[test]
    fn test_jsep_bridge_comparison_and_color_name() {
        let cond = jsep("${Height} >= 100");
        assert_eq!(
            cond.evaluate(&make_props(vec![("Height", json!(150.0))])),
            EvalResult::Bool(true)
        );
        assert_eq!(
            jsep("color('red')").evaluate(&make_props(vec![])),
            EvalResult::Color([1.0, 0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn test_jsep_bridge_missing_property_is_nan() {
        // 任务 #46：gate=1（JSEP）严格对齐上游的语义——缺失的属性
        // 解析为 `undefined`，它强转为 `Number(NaN)`（JS 的
        // `Number(undefined)`），而非遗留的 `0.0`。NaN != NaN，因此通过
        // `is_nan()` 断言。遗留的“缺失 -> 0.0”回退现在仅存在于
        // gate=0 轨道（[`Expression::PropertyRef`] / `json_to_eval_result`）。
        match jsep("${missing}").evaluate(&make_props(vec![])) {
            EvalResult::Number(n) => assert!(n.is_nan(), "expected NaN, got {n}"),
            other => panic!("expected Number(NaN), got {other:?}"),
        }
        // JSON `null` -> Value::Null -> Number(0.0)（JS `Number(null)`）。
        assert_eq!(
            jsep("${n}").evaluate(&make_props(vec![("n", json!(null))])),
            EvalResult::Number(0.0)
        );
        // 注意：这里*不*断言 `${missing} + 1`。忠实引擎会对 `undefined + 1`
        // 报出 RuntimeError（CesiumJS 的 `evaluatePlus` 对非数值/非字符串
        // 操作数会抛错——它不会产出 NaN），而 `Expression::evaluate` 不会失败，
        // 因此该形式回退到遗留解析器。那条算术路径由引擎的
        // 类型检查运算符控制，而非由本值 bridge 控制。
    }

    #[test]
    fn test_legacy_math_functions_match_js_semantics() {
        // 直接调用已弃用的遗留 `eval_function` 路径（与 gate 无关），
        // 以证明任务 #46 的修复。
        fn call(name: &str, args: Vec<f64>) -> EvalResult {
            Expression::FunctionCall {
                name: name.to_string(),
                args: args.into_iter().map(Expression::NumberConstant).collect(),
            }
            .evaluate(&make_props(vec![]))
        }
        // clamp：当 min > max 时 `f64::clamp` 会 panic；不会 panic 的条件
        // 形式遵循 CesiumMath.clamp（`v<min?min:v>max?max:v`）。
        assert_eq!(call("clamp", vec![5.0, 10.0, 0.0]), EvalResult::Number(10.0));
        assert_eq!(call("clamp", vec![5.0, 0.0, 10.0]), EvalResult::Number(5.0));
        assert_eq!(call("clamp", vec![-5.0, 0.0, 10.0]), EvalResult::Number(0.0));
        assert_eq!(call("clamp", vec![50.0, 0.0, 10.0]), EvalResult::Number(10.0));
        // round：向 +inf 取半（JS Math.round），而非远离零取半。
        assert_eq!(call("round", vec![2.5]), EvalResult::Number(3.0));
        assert_eq!(call("round", vec![-0.5]), EvalResult::Number(0.0));
        assert_eq!(call("round", vec![-2.5]), EvalResult::Number(-2.0));
        // sign：对 +-0 返回原始值（CesiumMath.sign），其余返回 1/-1。
        assert_eq!(call("sign", vec![0.0]), EvalResult::Number(0.0));
        assert_eq!(call("sign", vec![3.0]), EvalResult::Number(1.0));
        assert_eq!(call("sign", vec![-3.0]), EvalResult::Number(-1.0));
    }

    #[test]
    fn test_jsep_bridge_numeric_color_falls_back_to_legacy() {
        // 引擎会对带数值分量的 color(r,g,b,a) 报错；evaluate()
        // 回退到遗留解析器，保留 M7-D 之前的结果。
        let expr = jsep("color(1.0, 0.5, 0.0, 1.0)");
        assert_eq!(
            expr.evaluate(&make_props(vec![])),
            EvalResult::Color([1.0, 0.5, 0.0, 1.0])
        );
    }

    #[test]
    fn test_jsep_expression_clone_debug_eq() {
        let a = jsep("${x} * 2.0");
        let b = a.clone();
        assert_eq!(a, b);
        assert!(format!("{a:?}").contains("${x} * 2.0"));
    }
}
