//! 3D Tiles Styling language implementation.
//!
//! Maps to CesiumJS:
//! - `Scene/Cesium3DTileStyle.js`
//! - `Scene/Expression.js`
//! - `Scene/ConditionsExpression.js`
//!
//! The 3D Tiles Styling language allows defining styles based on feature properties:
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

// ── M7-D: JSEP styling-engine feature gate ───────────────────────────────

/// Env var toggling the new `cesium-styling` JSEP expression engine.
///
/// Mirrors the M0.3 feature-flag registry constant `ENV_ENABLE_STYLING_JSEP`
/// (env string `CESIUM_ENABLE_STYLING_JSEP`). This crate is a **domain** crate
/// and must not depend on the application-layer `feature_flags` module (DDD
/// dependency direction), so the truthy read is duplicated locally with
/// byte-identical M0.3 semantics. Default OFF → legacy naive parser.
const ENV_ENABLE_STYLING_JSEP: &str = "CESIUM_ENABLE_STYLING_JSEP";

/// M0.3 truthy tokens: case-insensitive, whitespace-trimmed `1|true|yes|on`.
/// Byte-identical to `cesium-app::feature_flags::truthy`.
fn truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Returns `true` when the JSEP styling engine gate is enabled (default OFF).
///
/// When enabled, [`Expression::parse`] compiles via `cesium_styling` and
/// evaluation delegates to the new engine (with a legacy fallback for forms the
/// engine rejects). When disabled, the legacy naive parser is used unchanged.
pub fn styling_jsep_enabled() -> bool {
    match std::env::var(ENV_ENABLE_STYLING_JSEP) {
        Ok(v) => truthy(&v),
        Err(_) => false,
    }
}

/// A parsed expression that can be evaluated against feature properties.
///
/// Maps to CesiumJS `Scene/Expression.js`
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A constant boolean value.
    BoolConstant(bool),
    /// A constant numeric value.
    NumberConstant(f64),
    /// A constant string value.
    StringConstant(String),
    /// A property reference: `${propertyName}`
    PropertyRef(String),
    /// A binary operation (e.g., `${Height} >= 100`)
    BinaryOp {
        /// Left operand.
        left: Box<Expression>,
        /// Operator.
        op: BinaryOperator,
        /// Right operand.
        right: Box<Expression>,
    },
    /// A unary operation (e.g., `!${visible}`)
    UnaryOp {
        /// Operator.
        op: UnaryOperator,
        /// Operand.
        operand: Box<Expression>,
    },
    /// A function call (e.g., `color('red', 0.5)`)
    FunctionCall {
        /// Function name.
        name: String,
        /// Arguments.
        args: Vec<Expression>,
    },
    /// A color literal (parsed from `color('name')` or `color(r, g, b, a)`)
    ColorLiteral([f64; 4]),
    /// M7-D: an expression compiled by the `cesium-styling` JSEP engine.
    ///
    /// Only produced when [`styling_jsep_enabled()`] is true. Evaluation
    /// delegates to the new engine and falls back to the legacy parser when the
    /// engine rejects an expression form it does not model (e.g. `color(r,g,b,a)`
    /// with numeric components). Additive variant: no existing consumer matches
    /// [`Expression`] exhaustively.
    Jsep(JsepExpression),
}

/// A JSEP-compiled expression (M7-D double-track wrapper).
///
/// `cesium_styling::Expression` carries no `Clone`/`Debug`/`PartialEq` derives,
/// so it is held behind an [`Arc`] and the three traits are implemented
/// manually: `Clone` shares the compiled AST, `Debug` prints only the source
/// text, and `PartialEq` compares sources. This keeps [`Expression`]'s derives
/// (and thus `TileStyle: Clone/Debug`) intact.
pub struct JsepExpression {
    /// The original expression source (pre-define/variable expansion), also used
    /// to re-parse via the legacy fallback when the engine errors at eval time.
    pub source: String,
    /// The compiled engine expression, shared cheaply on clone.
    compiled: Arc<cesium_styling::Expression>,
}

impl JsepExpression {
    /// Compiles `source` with the JSEP engine, or `None` on parse failure.
    ///
    /// `defines` are not threaded through [`Expression::parse`]'s signature; the
    /// legacy parser likewise stored-but-never-expanded defines, so passing
    /// `None` preserves bit-exact parity. Define expansion needs an API change
    /// and is deferred (M7.7).
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

/// Binary operators for expressions.
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

/// Unary operators for expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// `!` (logical not)
    Not,
    /// `-` (negation)
    Negate,
}

/// The result of evaluating an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    /// Boolean result.
    Bool(bool),
    /// Numeric result.
    Number(f64),
    /// String result.
    String(String),
    /// Color result [r, g, b, a] in 0-1 range.
    Color([f64; 4]),
}

impl EvalResult {
    /// Converts to boolean (for show expressions).
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            // JS `Boolean(NaN)` === false; guard NaN so an `undefined`-derived
            // NaN (gate=1 strict coercion) stays falsy, matching upstream `show`.
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::String(s) => !s.is_empty() && s != "false",
            Self::Color(_) => true,
        }
    }

    /// Converts to number (for pointSize expressions).
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

    /// Converts to color (for color expressions).
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
    /// Evaluates the expression against a set of feature properties.
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
                    // The engine rejects some legacy-accepted forms (e.g.
                    // `color(r,g,b,a)` with numeric components). Fall back to the
                    // legacy parser on the original source so `evaluate` stays
                    // total and bit-exact with the pre-M7-D behaviour.
                    Err(_) => {
                        #[allow(deprecated)]
                        legacy_parse(&jsep.source).evaluate(properties)
                    }
                }
            }
        }
    }

    /// Parses an expression from a string.
    ///
    /// M7-D double-track dispatch: when [`styling_jsep_enabled()`] is true the
    /// input is compiled by the `cesium-styling` JSEP engine and wrapped as
    /// [`Expression::Jsep`]; otherwise (gate default OFF) it delegates to the
    /// legacy naive parser [`legacy_parse`]. `parse` stays infallible: a JSEP
    /// compile failure falls back to the legacy parser.
    ///
    /// Supports:
    /// - Property references: `${Height}`
    /// - Comparisons: `${Height} >= 100`
    /// - Boolean literals: `true`, `false`
    /// - Numeric literals: `2.0`, `100`
    /// - Function calls: `color('red')`, `color(1.0, 0.0, 0.0, 1.0)`
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

/// The legacy naive expression parser (pre-M7-D).
///
/// Retained verbatim behind the [`styling_jsep_enabled()`] gate and as the
/// fallback for expression forms the JSEP engine rejects (e.g. `color(r,g,b,a)`
/// with numeric components). Known defects: first-match operator precedence and
/// `defines` stored but never expanded. The JSEP engine is the corrected
/// replacement; scheduled for removal after 2026-10-15 (M7.7).
#[deprecated(note = "use cesium_styling::Expression; remove after 2026-10-15")]
fn legacy_parse(input: &str) -> Expression {
    let input = input.trim();

    // Boolean literals
    if input == "true" {
        return Expression::BoolConstant(true);
    }
    if input == "false" {
        return Expression::BoolConstant(false);
    }

    // Numeric literal
    if let Ok(n) = input.parse::<f64>() {
        return Expression::NumberConstant(n);
    }

    // String literal
    if (input.starts_with('\'') && input.ends_with('\''))
        || (input.starts_with('"') && input.ends_with('"'))
    {
        return Expression::StringConstant(input[1..input.len() - 1].to_string());
    }

    // Property reference (only if the entire string is a single property ref)
    if input.starts_with("${") && input.ends_with('}') {
        // Check if this is a single property ref (no other content after the closing })
        let inner = &input[2..input.len() - 1];
        // Make sure there's no nested ${ or } inside
        if !inner.contains("${") && !inner.contains('}') {
            return Expression::PropertyRef(inner.to_string());
        }
    }

    // Function call: color(...), rgb(...), etc.
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

    // Binary operations (simple parsing for common cases)
    // Try comparison operators first
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

    // Arithmetic operators
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

    // Logical operators
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

    // Unary not
    if let Some(stripped) = input.strip_prefix('!') {
        let operand = legacy_parse(stripped);
        return Expression::UnaryOp {
            op: UnaryOperator::Not,
            operand: Box::new(operand),
        };
    }

    // Fallback: treat as string
    Expression::StringConstant(input.to_string())
}

/// Finds an operator position, avoiding matches inside `${...}` or quotes.
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

        // Check for operator match
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

/// Parses function arguments from a comma-separated string.
///
/// Legacy-only helper: recursion stays on [`legacy_parse`] so the fallback path
/// never mixes in JSEP-compiled sub-expressions.
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

/// Evaluates a binary operation.
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

/// Evaluates a function call.
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

            // color('name') or color('name', alpha)
            if let Expression::StringConstant(color_name) = &args[0] {
                let base_color = parse_color_name(color_name);
                let alpha = if args.len() > 1 {
                    args[1].evaluate(properties).as_number()
                } else {
                    1.0
                };
                return EvalResult::Color([base_color[0], base_color[1], base_color[2], alpha]);
            }

            // color(r, g, b) or color(r, g, b, a)
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
            // vec4(value) -> grayscale color
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
                // `f64::clamp` panics when `min > max` or on NaN (a DoS vector
                // for attacker-supplied styling). Use the non-panicking
                // conditional form; NaN falls through to `v`.
                let clamped = if v < min { min } else if v > max { max } else { v };
                return EvalResult::Number(clamped);
            }
            EvalResult::Number(0.0)
        }
        // Trigonometric
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
        // Angle conversion
        "radians" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.to_radians())
        }
        "degrees" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v.to_degrees())
        }
        // Rounding / sign
        "sign" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            // `else { v }` (not `0.0`): returns the original value for NaN and
            // ±0, matching CesiumMath.sign.
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
            // JS Math.round is half-toward-+∞ (Math.round(-0.5) === 0), whereas
            // `f64::round` is half-away-from-zero. Use `(v + 0.5).floor()`.
            EvalResult::Number((v + 0.5).floor())
        }
        "fract" => {
            let v = args.first().map(|a| a.evaluate(properties).as_number()).unwrap_or(0.0);
            EvalResult::Number(v - v.floor())
        }
        // Exponential / logarithmic
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
        // Interpolation
        "mix" => {
            if args.len() >= 3 {
                let a = args[0].evaluate(properties).as_number();
                let b = args[1].evaluate(properties).as_number();
                let t = args[2].evaluate(properties).as_number();
                return EvalResult::Number(a * (1.0 - t) + b * t);
            }
            EvalResult::Number(0.0)
        }
        // HSL color
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

/// Parses a CSS color name to [r, g, b] in 0-1 range.
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
        _ => [1.0, 1.0, 1.0], // default white
    }
}

/// Converts HSL to RGB. h in [0,360], s in [0,1], l in [0,1].
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

/// Converts a JSON value to an EvalResult.
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

// ── M7-D: JSEP engine bridge (feature adapter + value converters) ────────

/// Adapter exposing `&HashMap<String, serde_json::Value>` as a
/// [`cesium_styling::ExpressionFeature`] so the JSEP engine can read feature
/// properties during evaluation.
struct JsonFeature<'a>(&'a HashMap<String, Value>);

impl cesium_styling::ExpressionFeature for JsonFeature<'_> {
    fn get_property_inherited(&self, name: &str) -> Option<cesium_styling::Value> {
        self.0.get(name).map(json_to_cesium_value)
    }
}

/// Converts a `serde_json::Value` feature property into a
/// [`cesium_styling::Value`] for the JSEP engine.
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

/// Converts a [`cesium_styling::Value`] evaluation result into an [`EvalResult`].
///
/// Gate=1 (JSEP) strict-upstream coercion: `Undefined` -> `Number(NaN)` and
/// `Null` -> `Number(0.0)`, mirroring JS `Number(undefined)` / `Number(null)`.
/// This keeps `${x} + 1` on a missing property `NaN` rather than `1.0`. The
/// legacy "missing property -> 0.0" fallback is preserved only on the gate=0
/// track (`Expression::PropertyRef` / `json_to_eval_result`).
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
        // RegExp / Array results have no EvalResult equivalent; match the legacy
        // safe default (see `json_to_eval_result`).
        CV::RegExp(_) | CV::Array(_) => EvalResult::Number(0.0),
    }
}

/// A condition in a conditions expression: [condition, result].
#[derive(Debug, Clone)]
pub struct Condition {
    /// The condition expression (evaluates to boolean).
    pub condition: Expression,
    /// The result expression (evaluated if condition is true).
    pub result: Expression,
}

/// A conditions expression: a list of [condition, result] pairs.
///
/// Maps to CesiumJS `Scene/ConditionsExpression.js`
///
/// The first condition that evaluates to true determines the result.
#[derive(Debug, Clone)]
pub struct ConditionsExpression {
    /// The conditions in order.
    pub conditions: Vec<Condition>,
}

impl ConditionsExpression {
    /// Parses a conditions expression from JSON.
    ///
    /// Expected format:
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

    /// Evaluates the conditions against feature properties.
    ///
    /// Returns the result of the first condition that evaluates to true.
    pub fn evaluate(&self, properties: &HashMap<String, Value>) -> EvalResult {
        for cond in &self.conditions {
            let cond_result = cond.condition.evaluate(properties);
            if cond_result.as_bool() {
                return cond.result.evaluate(properties);
            }
        }
        // Default: return white color for color expressions, true for show
        EvalResult::Color([1.0, 1.0, 1.0, 1.0])
    }
}

/// A style expression that can be either a simple expression or conditions.
#[derive(Debug, Clone)]
pub enum StyleExpression {
    /// A simple expression.
    Simple(Expression),
    /// A conditions expression.
    Conditions(ConditionsExpression),
}

impl StyleExpression {
    /// Parses a style expression from JSON.
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

    /// Evaluates the expression against feature properties.
    pub fn evaluate(&self, properties: &HashMap<String, Value>) -> EvalResult {
        match self {
            Self::Simple(expr) => expr.evaluate(properties),
            Self::Conditions(conds) => conds.evaluate(properties),
        }
    }
}

/// A 3D Tiles style definition.
///
/// Maps to CesiumJS `Scene/Cesium3DTileStyle.js`
#[derive(Debug, Clone, Default)]
pub struct TileStyle {
    /// The show expression (determines visibility).
    pub show: Option<StyleExpression>,
    /// The color expression.
    pub color: Option<StyleExpression>,
    /// The point size expression (for point clouds).
    pub point_size: Option<StyleExpression>,
    /// Point outline color expression.
    pub point_outline_color: Option<StyleExpression>,
    /// Point outline width expression.
    pub point_outline_width: Option<StyleExpression>,
    /// Label text expression.
    pub label_text: Option<StyleExpression>,
    /// Label color expression.
    pub label_color: Option<StyleExpression>,
    /// Meta expressions (for feature metadata).
    pub meta: HashMap<String, StyleExpression>,
    /// Defines (reusable expressions).
    pub defines: HashMap<String, String>,
}

impl TileStyle {
    /// Parses a style from JSON.
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

        // Parse meta
        if let Some(meta_obj) = json.get("meta").and_then(|m| m.as_object()) {
            for (key, value) in meta_obj {
                if let Some(expr) = StyleExpression::from_json(value) {
                    style.meta.insert(key.clone(), expr);
                }
            }
        }

        // Parse defines
        if let Some(defines_obj) = json.get("defines").and_then(|d| d.as_object()) {
            for (key, value) in defines_obj {
                if let Some(s) = value.as_str() {
                    style.defines.insert(key.clone(), s.to_string());
                }
            }
        }

        style
    }

    /// Evaluates the show expression for a feature.
    pub fn evaluate_show(&self, properties: &HashMap<String, Value>) -> bool {
        match &self.show {
            Some(expr) => expr.evaluate(properties).as_bool(),
            None => true, // default: show all
        }
    }

    /// Evaluates the color expression for a feature.
    pub fn evaluate_color(&self, properties: &HashMap<String, Value>) -> [f64; 4] {
        match &self.color {
            Some(expr) => expr.evaluate(properties).as_color(),
            None => [1.0, 1.0, 1.0, 1.0], // default: white
        }
    }

    /// Evaluates the point size expression for a feature.
    pub fn evaluate_point_size(&self, properties: &HashMap<String, Value>) -> f64 {
        match &self.point_size {
            Some(expr) => expr.evaluate(properties).as_number(),
            None => 1.0, // default: 1.0
        }
    }

    /// Evaluates a meta expression for a feature.
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
        // purple = rgb(128,0,128). The legacy color table rounds to 0.502 while
        // the JSEP engine yields the exact 128/255 = 0.50196 (upstream-faithful).
        // Tolerance-based assertion accepts both double-track results, matching
        // the sibling color tests in this module.
        assert!((color[0] - 0.502).abs() < 1e-3, "r={}", color[0]);
        assert!(color[1].abs() < 1e-3, "g={}", color[1]);
        assert!((color[2] - 0.502).abs() < 1e-3, "b={}", color[2]);
        assert!((color[3] - 0.5).abs() < 1e-3, "a={}", color[3]); // purple with alpha

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

        // Note: string concatenation is not fully implemented,
        // but the meta expression should be parseable
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

    // ── M7-D: gate helper + JSEP bridge ──────────────────────────────────

    #[test]
    fn test_truthy_tokens() {
        // M0.3 semantics: case-insensitive, whitespace-trimmed 1|true|yes|on.
        for t in ["1", "true", "TRUE", "True", "yes", "YES", "on", " on ", "\ttrue\n"] {
            assert!(truthy(t), "expected truthy: {t:?}");
        }
        for f in ["0", "false", "no", "off", "", "  ", "maybe", "2", "truex"] {
            assert!(!truthy(f), "expected falsy: {f:?}");
        }
    }

    #[test]
    fn test_styling_jsep_gate_is_bool() {
        // Only asserts the accessor is total; the value depends on the ambient
        // env (default OFF). Env mutation is avoided to keep tests deterministic.
        let _ = styling_jsep_enabled();
    }

    /// Builds a JSEP-backed expression directly (gate-independent) so the bridge
    /// is exercised deterministically without mutating the process env.
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
        // Task #46: gate=1 (JSEP) strict-upstream semantics — a missing property
        // resolves to `undefined`, which coerces to `Number(NaN)` (JS
        // `Number(undefined)`), NOT the legacy `0.0`. NaN != NaN, so assert via
        // `is_nan()`. The legacy "missing -> 0.0" fallback now lives only on the
        // gate=0 track (`Expression::PropertyRef` / `json_to_eval_result`).
        match jsep("${missing}").evaluate(&make_props(vec![])) {
            EvalResult::Number(n) => assert!(n.is_nan(), "expected NaN, got {n}"),
            other => panic!("expected Number(NaN), got {other:?}"),
        }
        // JSON `null` -> Value::Null -> Number(0.0) (JS `Number(null)`).
        assert_eq!(
            jsep("${n}").evaluate(&make_props(vec![("n", json!(null))])),
            EvalResult::Number(0.0)
        );
        // Note: `${missing} + 1` is *not* asserted here. The faithful engine
        // raises a RuntimeError for `undefined + 1` (CesiumJS `evaluatePlus`
        // throws on a non-number/non-string operand — it does not yield NaN),
        // and `Expression::evaluate` is infallible, so that form falls back to
        // the legacy parser. That arithmetic path is governed by the engine's
        // type-checked operators, not by this value bridge.
    }

    #[test]
    fn test_legacy_math_functions_match_js_semantics() {
        // Directly exercise the deprecated legacy `eval_function` path
        // (gate-independent) to prove the task #46 fixes.
        fn call(name: &str, args: Vec<f64>) -> EvalResult {
            Expression::FunctionCall {
                name: name.to_string(),
                args: args.into_iter().map(Expression::NumberConstant).collect(),
            }
            .evaluate(&make_props(vec![]))
        }
        // clamp: `f64::clamp` panics when min > max; the non-panicking
        // conditional form follows CesiumMath.clamp (`v<min?min:v>max?max:v`).
        assert_eq!(call("clamp", vec![5.0, 10.0, 0.0]), EvalResult::Number(10.0));
        assert_eq!(call("clamp", vec![5.0, 0.0, 10.0]), EvalResult::Number(5.0));
        assert_eq!(call("clamp", vec![-5.0, 0.0, 10.0]), EvalResult::Number(0.0));
        assert_eq!(call("clamp", vec![50.0, 0.0, 10.0]), EvalResult::Number(10.0));
        // round: half-toward-+inf (JS Math.round), not half-away-from-zero.
        assert_eq!(call("round", vec![2.5]), EvalResult::Number(3.0));
        assert_eq!(call("round", vec![-0.5]), EvalResult::Number(0.0));
        assert_eq!(call("round", vec![-2.5]), EvalResult::Number(-2.0));
        // sign: original value for +-0 (CesiumMath.sign), 1/-1 otherwise.
        assert_eq!(call("sign", vec![0.0]), EvalResult::Number(0.0));
        assert_eq!(call("sign", vec![3.0]), EvalResult::Number(1.0));
        assert_eq!(call("sign", vec![-3.0]), EvalResult::Number(-1.0));
    }

    #[test]
    fn test_jsep_bridge_numeric_color_falls_back_to_legacy() {
        // The engine errors on color(r,g,b,a) numeric components; evaluate()
        // falls back to the legacy parser, preserving the pre-M7-D result.
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
