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
            Self::Number(n) => *n != 0.0,
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
        }
    }

    /// Parses an expression from a string.
    ///
    /// Supports:
    /// - Property references: `${Height}`
    /// - Comparisons: `${Height} >= 100`
    /// - Boolean literals: `true`, `false`
    /// - Numeric literals: `2.0`, `100`
    /// - Function calls: `color('red')`, `color(1.0, 0.0, 0.0, 1.0)`
    pub fn parse(input: &str) -> Self {
        let input = input.trim();

        // Boolean literals
        if input == "true" {
            return Self::BoolConstant(true);
        }
        if input == "false" {
            return Self::BoolConstant(false);
        }

        // Numeric literal
        if let Ok(n) = input.parse::<f64>() {
            return Self::NumberConstant(n);
        }

        // String literal
        if (input.starts_with('\'') && input.ends_with('\''))
            || (input.starts_with('"') && input.ends_with('"'))
        {
            return Self::StringConstant(input[1..input.len() - 1].to_string());
        }

        // Property reference (only if the entire string is a single property ref)
        if input.starts_with("${") && input.ends_with('}') {
            // Check if this is a single property ref (no other content after the closing })
            let inner = &input[2..input.len() - 1];
            // Make sure there's no nested ${ or } inside
            if !inner.contains("${") && !inner.contains('}') {
                return Self::PropertyRef(inner.to_string());
            }
        }

        // Function call: color(...), rgb(...), etc.
        if let Some(paren_start) = input.find('(') {
            if input.ends_with(')') {
                let func_name = input[..paren_start].trim();
                let args_str = &input[paren_start + 1..input.len() - 1];
                let args = parse_function_args(args_str);
                return Self::FunctionCall {
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
                let left = Self::parse(left_str);
                let right = Self::parse(right_str);
                let op = match op_str {
                    ">=" => BinaryOperator::Ge,
                    "<=" => BinaryOperator::Le,
                    "!=" => BinaryOperator::Ne,
                    "==" => BinaryOperator::Eq,
                    ">" => BinaryOperator::Gt,
                    "<" => BinaryOperator::Lt,
                    _ => unreachable!(),
                };
                return Self::BinaryOp {
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
                    let left = Self::parse(left_str);
                    let right = Self::parse(right_str);
                    let op = match op_str {
                        "+" => BinaryOperator::Add,
                        "-" => BinaryOperator::Sub,
                        "*" => BinaryOperator::Mul,
                        "/" => BinaryOperator::Div,
                        _ => unreachable!(),
                    };
                    return Self::BinaryOp {
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
                let left = Self::parse(left_str);
                let right = Self::parse(right_str);
                let op = if op_str == "&&" {
                    BinaryOperator::And
                } else {
                    BinaryOperator::Or
                };
                return Self::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            }
        }

        // Unary not
        if let Some(stripped) = input.strip_prefix('!') {
            let operand = Self::parse(stripped);
            return Self::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(operand),
            };
        }

        // Fallback: treat as string
        Self::StringConstant(input.to_string())
    }
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
                args.push(Expression::parse(current.trim()));
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        args.push(Expression::parse(current.trim()));
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
                return EvalResult::Number(v.clamp(min, max));
            }
            EvalResult::Number(0.0)
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
        assert_eq!(color, [0.502, 0.0, 0.502, 0.5]); // purple with alpha

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
}
