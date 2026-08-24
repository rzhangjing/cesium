//! Ported from `packages/engine/Source/Scene/Expression.js`.
//!
//! An expression for a style applied to a `Cesium3DTileset`. Evaluates an
//! expression defined using the 3D Tiles Styling language.
//!
//! The implementation mirrors the original pipeline:
//! `replaceDefines` -> `removeBackslashes` -> `replaceVariables` -> parse
//! (a hand-written parser mirroring jsep 1.x with the `=~`/`!~` binary
//! operators added at precedence 0, see the `jsep.addBinaryOp` calls in the
//! original constructor) -> `createRuntimeAst` -> per-node evaluate
//! functions.
//!
//! DEVIATION: jsep itself is not embedded; the tokenizer/Pratt parser below
//! reproduces the subset of jsep 1.3.8 semantics the styling language needs
//! (no regex literals, no template literals; `=~`/`!~` at precedence 0).
//! DEVIATION: scratch-storage reuse (`scratchStorage`) is unnecessary in Rust
//! because evaluation returns owned values.
//! DEVIATION: the `regex` crate does not support JS lookbehind/lookahead or
//! the `u`/`y` flag semantics; patterns using them fail to compile at the
//! same point (parse or evaluation) the original would succeed.

use std::collections::HashMap;

use regex::Regex;

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::color::Color;
use cesium_core::math::CesiumMath;
use cesium_core::runtime_error::RuntimeError;

use crate::expression_node_type::ExpressionNodeType;

/// Convenience constructor mirroring `new RuntimeError(message)`.
fn runtime_error(message: &str) -> RuntimeError {
    RuntimeError::new(Some(message))
}

// ---------------------------------------------------------------------------
// Dynamic values (mirrors the JS values the expression language produces)
// ---------------------------------------------------------------------------

/// A compiled regular expression value, mirroring a JS `RegExp` produced by
/// the `regExp()` function.
#[derive(Debug, Clone)]
pub struct RegExpValue {
    compiled: Regex,
    /// The original (backslash-restored) pattern source.
    pub source: String,
    pub flags: String,
}

impl RegExpValue {
    /// Compiles a pattern with JS-style flags (`i`, `m`, `s`; `g`/`u`/`y`
    /// are accepted and ignored since the `regex` crate has no global state).
    /// Mirrors `new RegExp(pattern, flags)` wrapped in try/catch.
    pub fn compile(pattern: &str, flags: &str) -> Result<RegExpValue, RuntimeError> {
        let mut prefix = String::new();
        for c in flags.chars() {
            match c {
                'i' => prefix.push_str("(?i)"),
                'm' => prefix.push_str("(?m)"),
                's' => prefix.push_str("(?s)"),
                'g' | 'u' | 'y' => {}
                _ => {
                    return Err(runtime_error(&format!(
                        "Invalid flags given to RegExp constructor: {flags}"
                    )))
                }
            }
        }
        match Regex::new(&format!("{prefix}{pattern}")) {
            Ok(compiled) => Ok(RegExpValue {
                compiled,
                source: pattern.to_string(),
                flags: flags.to_string(),
            }),
            Err(e) => Err(runtime_error(&e.to_string())),
        }
    }

    /// Mirrors `RegExp.prototype.test`.
    pub fn test(&self, text: &str) -> bool {
        self.compiled.is_match(text)
    }

    /// Mirrors `RegExp.prototype.exec`, returning capture group 1 when
    /// present (the full match otherwise), as used by `_evaluateRegExpExec`.
    pub fn exec_first_capture(&self, text: &str) -> Option<String> {
        let captures = self.compiled.captures(text)?;
        let group = captures.get(1).or_else(|| captures.get(0))?;
        Some(group.as_str().to_string())
    }

    /// Mirrors `String(regExp)` -> `"/pattern/flags"`. JS sorts the flags
    /// in `dgimsuy` order when stringifying a RegExp.
    pub fn to_js_string(&self) -> String {
        let mut sorted: Vec<char> = self.flags.chars().collect();
        sorted.sort_by_key(|c| "dgimsuy".find(*c).unwrap_or(usize::MAX));
        let flags: String = sorted.into_iter().collect();
        format!("/{}/{}", self.source, flags)
    }
}

/// The dynamic value type produced by expression evaluation, mirroring the
/// union of JS values the styling language returns.
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Cartesian2(Cartesian2),
    Cartesian3(Cartesian3),
    Cartesian4(Cartesian4),
    RegExp(RegExpValue),
    Array(Vec<Value>),
}

impl Value {
    pub fn is_defined(&self) -> bool {
        !matches!(self, Value::Undefined | Value::Null)
    }

    /// JS truthiness.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    /// Mirrors `Boolean(value)`.
    pub fn boolean_conversion(&self) -> bool {
        self.is_truthy()
    }

    /// Mirrors `Number(value)` for the value types this language produces.
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
            // undefined, vectors, regex and arrays all become NaN when
            // coerced through Number().
            _ => f64::NAN,
        }
    }

    /// Mirrors `String(value)`.
    pub fn string_conversion(&self) -> String {
        match self {
            Value::Undefined => "undefined".to_string(),
            Value::Null => "null".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => number_to_js_string(*n),
            Value::String(s) => s.clone(),
            Value::Cartesian2(v) => format!("({}, {})", v.x, v.y),
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

    /// Mirrors `left === right` (strict equality; Cartesian values compare
    /// componentwise, as in `_evaluateEqualsStrict`).
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
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string_conversion())
    }
}

/// Deep equality used by tests and callers: strict-equality semantics for
/// scalars/vectors (mirrors `_evaluateEqualsStrict`), plus source/flags
/// comparison for regex values and elementwise comparison for arrays.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::RegExp(a), Value::RegExp(b)) => {
                a.source == b.source && a.flags == b.flags
            }
            (Value::Array(a), Value::Array(b)) => a == b,
            _ => self.equals_strict(other),
        }
    }
}

/// Formats a number the way JS string interpolation does (mirrors the `${}`
/// interpolation used in all RuntimeError messages of Expression.js).
pub(crate) fn number_to_js_string(number: f64) -> String {
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

/// Mirrors JS `Number("...")` string coercion (trimmed, decimal, exponent,
/// hex; anything else is NaN).
fn js_parse_number(text: &str) -> f64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
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

// ---------------------------------------------------------------------------
// Feature interface (mirrors the Cesium3DTileFeature methods used here)
// ---------------------------------------------------------------------------

/// The feature properties interface used by expression evaluation, mirroring
/// the `Cesium3DTileFeature` methods `getPropertyInherited`, `isExactClass`,
/// `isClass` and `getExactClassName`.
pub trait ExpressionFeature {
    /// Mirrors `getPropertyInherited(name)`; `None` is `undefined`.
    fn get_property_inherited(&self, name: &str) -> Option<Value>;

    /// Mirrors `isExactClass(className)`.
    fn is_exact_class(&self, _class_name: &Value) -> bool {
        false
    }

    /// Mirrors `isClass(className)`.
    fn is_class(&self, _class_name: &Value) -> bool {
        false
    }

    /// Mirrors `getExactClassName()`.
    fn get_exact_class_name(&self) -> Option<Value> {
        None
    }
}

/// Mirrors `getFeatureProperty`: returns undefined when the feature is not
/// defined or the property is missing.
fn get_feature_property(feature: Option<&dyn ExpressionFeature>, name: &str) -> Value {
    match feature {
        Some(feature) => feature.get_property_inherited(name).unwrap_or(Value::Undefined),
        None => Value::Undefined,
    }
}

fn check_feature(node: &Node) -> bool {
    matches!(&node.value, NodeValue::Str(value) if value == "feature")
}

// ---------------------------------------------------------------------------
// Runtime AST node
// ---------------------------------------------------------------------------

/// The payload of a runtime AST node, mirroring JS `_value`.
#[derive(Debug, Clone)]
pub enum NodeValue {
    /// No value (e.g. `getExactClassName`).
    None,
    Null,
    Undefined,
    Bool(bool),
    Number(f64),
    Str(String),
    /// Child expressions (the ARRAY node type).
    Nodes(Vec<Node>),
    /// A pre-compiled regular expression (LITERAL_REGEX).
    Regex(RegExpValue),
}

/// A runtime AST node, mirroring the `Node` constructor of Expression.js.
/// `_left` may be either a single node or an array of argument nodes
/// (LITERAL_COLOR / LITERAL_VECTOR); `left_children` holds the latter.
#[derive(Debug, Clone)]
pub struct Node {
    pub node_type: ExpressionNodeType,
    pub value: NodeValue,
    pub left: Option<Box<Node>>,
    pub left_children: Option<Vec<Node>>,
    pub right: Option<Box<Node>>,
    pub test: Option<Box<Node>>,
}

impl Node {
    fn new(
        node_type: ExpressionNodeType,
        value: NodeValue,
        left: Option<Node>,
        right: Option<Node>,
        test: Option<Node>,
    ) -> Node {
        Node {
            node_type,
            value,
            left: left.map(Box::new),
            left_children: None,
            right: right.map(Box::new),
            test: test.map(Box::new),
        }
    }

    fn with_children(
        node_type: ExpressionNodeType,
        value: NodeValue,
        children: Vec<Node>,
    ) -> Node {
        Node {
            node_type,
            value,
            left: None,
            left_children: Some(children),
            right: None,
            test: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Preprocessing (replaceDefines / removeBackslashes / replaceVariables)
// ---------------------------------------------------------------------------

const VARIABLE_PATTERN: &str = r"\$\{(.*?)}";
const BACKSLASH_REPLACEMENT: &str = "@#%";

/// Mirrors `replaceDefines`: replaces each `${key}` placeholder with
/// `(<define value>)`.
fn replace_defines(expression: &str, defines: &HashMap<String, String>) -> String {
    let mut result = expression.to_string();
    for (key, value) in defines {
        let placeholder = Regex::new(&format!(r"\$\{{{}}}", regex::escape(key)))
            .expect("escaped define key is a valid regex");
        let define_replace = format!("({value})");
        // NoExpand: the replacement value may itself contain `${...}` which
        // would otherwise be interpreted as capture-group references.
        result = placeholder
            .replace_all(&result, regex::NoExpand(define_replace.as_str()))
            .to_string();
    }
    result
}

/// Mirrors `removeBackslashes`: `\` -> `"@#%"`.
fn remove_backslashes(expression: &str) -> String {
    expression.replace('\\', BACKSLASH_REPLACEMENT)
}

/// Mirrors `replaceBackslashes`: `"@#%"` -> `\`.
fn replace_backslashes(expression: &str) -> String {
    expression.replace(BACKSLASH_REPLACEMENT, "\\")
}

/// Mirrors `replaceVariables`: `${name}` outside of quotes becomes
/// `czm_name`; an unterminated `${` throws `"Unmatched {."`.
fn replace_variables(expression: &str) -> Result<String, RuntimeError> {
    let mut exp = expression.to_string();
    let mut result = String::new();
    while let Some(i) = exp.find("${") {
        // Check if string is inside quotes
        let open_single_quote = exp.find('\'');
        let open_double_quote = exp.find('"');
        if let Some(open) = open_single_quote {
            if open < i {
                let close = exp[open + 1..].find('\'').map(|index| open + 1 + index);
                let close_quote = close.unwrap_or(exp.len() - 1);
                result.push_str(&exp[..close_quote + 1]);
                exp = exp[close_quote + 1..].to_string();
                continue;
            }
        }
        if let Some(open) = open_double_quote {
            if open < i {
                let close = exp[open + 1..].find('"').map(|index| open + 1 + index);
                let close_quote = close.unwrap_or(exp.len() - 1);
                result.push_str(&exp[..close_quote + 1]);
                exp = exp[close_quote + 1..].to_string();
                continue;
            }
        }
        result.push_str(&exp[..i]);
        let j = match exp.find('}') {
            Some(j) => j,
            None => return Err(runtime_error("Unmatched {.")),
        };
        result.push_str("czm_");
        result.push_str(&exp[i + 2..j]);
        exp = exp[j + 1..].to_string();
    }
    result.push_str(&exp);
    Ok(result)
}

// ---------------------------------------------------------------------------
// Tokenizer + Pratt parser (mirrors jsep 1.3.8 + addBinaryOp("=~"/"!~", 0))
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Str(String),
    Ident(String),
    Op(String),
}

struct Tokenizer<'a> {
    chars: Vec<char>,
    index: usize,
    source: &'a str,
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_identifier_char(c: char) -> bool {
    is_identifier_start(c) || c.is_ascii_digit()
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

impl<'a> Tokenizer<'a> {
    fn new(source: &'a str) -> Tokenizer<'a> {
        Tokenizer {
            chars: source.chars().collect(),
            index: 0,
            source,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.index += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.index += 1;
            } else {
                break;
            }
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, RuntimeError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            let c = match self.peek() {
                Some(c) => c,
                None => break,
            };
            if c == '"' || c == '\'' {
                tokens.push(Token::Str(self.read_string(c)?));
            } else if is_digit(c) || (c == '.' && self.peek_at(1).map(is_digit) == Some(true)) {
                tokens.push(Token::Number(self.read_number()));
            } else if is_identifier_start(c) {
                tokens.push(Token::Ident(self.read_identifier()));
            } else {
                tokens.push(Token::Op(self.read_operator()?));
            }
        }
        Ok(tokens)
    }

    /// Strings carry no escape processing: backslashes were already replaced
    /// with `"@#%"` by `removeBackslashes`, mirroring jsep's behavior after
    /// the preprocessing step.
    fn read_string(&mut self, quote: char) -> Result<String, RuntimeError> {
        self.advance();
        let start = self.index;
        while let Some(c) = self.peek() {
            if c == quote {
                let value: String = self.chars[start..self.index].iter().collect();
                self.advance();
                return Ok(value);
            }
            self.index += 1;
        }
        let value: String = self.chars[start..].iter().collect();
        Err(runtime_error(&format!(
            "Unclosed quote after \"{value}\""
        )))
    }

    fn read_number(&mut self) -> f64 {
        let start = self.index;
        while self.peek().map(is_digit) == Some(true) {
            self.index += 1;
        }
        if self.peek() == Some('.') {
            self.index += 1;
            while self.peek().map(is_digit) == Some(true) {
                self.index += 1;
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            let mut lookahead = self.index + 1;
            if matches!(self.chars.get(lookahead), Some('+') | Some('-')) {
                lookahead += 1;
            }
            if self.chars.get(lookahead).map(|c| is_digit(*c)) == Some(true) {
                self.index = lookahead;
                while self.peek().map(is_digit) == Some(true) {
                    self.index += 1;
                }
            }
        }
        let text: String = self.chars[start..self.index].iter().collect();
        // jsep parses with parseFloat semantics; the tokenizer guarantees a
        // well-formed numeric literal here.
        text.parse::<f64>().unwrap_or(f64::NAN)
    }

    fn read_identifier(&mut self) -> String {
        let start = self.index;
        while self.peek().map(is_identifier_char) == Some(true) {
            self.index += 1;
        }
        self.chars[start..self.index].iter().collect()
    }

    fn read_operator(&mut self) -> Result<String, RuntimeError> {
        let three: String = self.chars[self.index..].iter().take(3).collect();
        if three == "===" || three == "!==" || three == ">>>" {
            self.index += 3;
            return Ok(three);
        }
        let two: String = self.chars[self.index..].iter().take(2).collect();
        match two.as_str() {
            "&&" | "||" | "=~" | "!~" | ">=" | "<=" | "<<" | ">>" => {
                self.index += 2;
                return Ok(two);
            }
            _ => {}
        }
        let c = self.advance().unwrap();
        // jsep accepts these operators; unsupported ones are rejected later
        // by create_runtime_ast with `Unexpected operator "{op}".`.
        if matches!(
            c,
            '+' | '-'
                | '*'
                | '/'
                | '%'
                | '>'
                | '<'
                | '!'
                | '~'
                | '|'
                | '&'
                | '^'
                | '('
                | ')'
                | '['
                | ']'
                | ','
                | '?'
                | ':'
                | '.'
                | ';'
        ) {
            return Ok(c.to_string());
        }
        let _ = self.source; // kept for parity with jsep error context
        Err(runtime_error(&format!("Unexpected \"{c}\"")))
    }
}

/// jsep-style AST produced by the parser before `create_runtime_ast`.
#[derive(Debug, Clone)]
enum JsepNode {
    Literal(JsepLiteral),
    Identifier(String),
    ThisExpression,
    Unary {
        operator: String,
        argument: Box<JsepNode>,
    },
    Binary {
        operator: String,
        left: Box<JsepNode>,
        right: Box<JsepNode>,
    },
    Conditional {
        test: Box<JsepNode>,
        consequent: Box<JsepNode>,
        alternate: Box<JsepNode>,
    },
    Member {
        object: Box<JsepNode>,
        property: Box<JsepNode>,
        computed: bool,
    },
    Call {
        callee: Box<JsepNode>,
        arguments: Vec<JsepNode>,
    },
    Array(Vec<JsepNode>),
}

#[derive(Debug, Clone)]
enum JsepLiteral {
    Null,
    Boolean(bool),
    Number(f64),
    Str(String),
}

/// Mirrors `jsep.binary_ops` (jsep 1.x) with the Cesium customizations
/// `addBinaryOp("=~", 0)` and `addBinaryOp("!~", 0)`.
fn binary_precedence(operator: &str) -> Option<u8> {
    Some(match operator {
        "=~" | "!~" => 0,
        "||" => 1,
        "&&" => 2,
        "|" => 3,
        "^" => 4,
        "&" => 5,
        "==" | "!=" | "===" | "!==" => 6,
        "<" | ">" | "<=" | ">=" => 7,
        "<<" | ">>" | ">>>" => 8,
        "+" | "-" => 9,
        "*" | "/" | "%" => 10,
        _ => return None,
    })
}

const UNARY_PRECEDENCE: u8 = 15;

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn parse(expression: &str) -> Result<JsepNode, RuntimeError> {
        let mut tokenizer = Tokenizer::new(expression);
        let tokens = tokenizer.tokenize()?;
        let mut parser = Parser { tokens, index: 0 };
        let node = parser.parse_expression(0)?;
        // Multiple expressions (or a trailing ";") yield a Compound node in
        // jsep, which createRuntimeAst rejects with
        // "Provide exactly one expression."
        if parser.peek().is_some() {
            return Err(runtime_error("Provide exactly one expression."));
        }
        Ok(node)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    fn parse_expression(&mut self, min_prec: u8) -> Result<JsepNode, RuntimeError> {
        let mut left = self.parse_unary()?;
        loop {
            let operator = match self.peek() {
                Some(Token::Op(op)) => op.clone(),
                _ => break,
            };
            if operator == "?" {
                if min_prec > 3 {
                    break;
                }
                self.advance();
                let consequent = self.parse_expression(0)?;
                match self.advance() {
                    Some(Token::Op(op)) if op == ":" => {}
                    _ => return Err(runtime_error("Expected :")),
                }
                let alternate = self.parse_expression(3)?;
                left = JsepNode::Conditional {
                    test: Box::new(left),
                    consequent: Box::new(consequent),
                    alternate: Box::new(alternate),
                };
                continue;
            }
            let prec = match binary_precedence(&operator) {
                Some(prec) => prec,
                None => break,
            };
            if prec < min_prec {
                break;
            }
            self.advance();
            let right = self.parse_expression(prec + 1)?;
            left = JsepNode::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<JsepNode, RuntimeError> {
        if let Some(Token::Op(op)) = self.peek() {
            if op == "!" || op == "-" || op == "+" || op == "~" {
                let operator = op.clone();
                self.advance();
                let argument = self.parse_expression(UNARY_PRECEDENCE)?;
                // jsep folds negative numeric literals (-2 parses as a
                // Literal in binary contexts); fold here for parity.
                if operator == "-" {
                    if let JsepNode::Literal(JsepLiteral::Number(value)) = argument {
                        return Ok(JsepNode::Literal(JsepLiteral::Number(-value)));
                    }
                }
                return Ok(JsepNode::Unary {
                    operator,
                    argument: Box::new(argument),
                });
            }
        }
        self.parse_member_and_call()
    }

    fn parse_member_and_call(&mut self) -> Result<JsepNode, RuntimeError> {
        let mut node = self.parse_primary()?;
        loop {
            match self.peek() {
                Some(Token::Op(op)) if op == "." => {
                    self.advance();
                    let name = match self.advance() {
                        Some(Token::Ident(name)) => name,
                        _ => return Err(runtime_error("Unexpected .")),
                    };
                    node = JsepNode::Member {
                        object: Box::new(node),
                        property: Box::new(JsepNode::Identifier(name)),
                        computed: false,
                    };
                }
                Some(Token::Op(op)) if op == "[" => {
                    self.advance();
                    let property = self.parse_expression(0)?;
                    match self.advance() {
                        Some(Token::Op(op)) if op == "]" => {}
                        _ => return Err(runtime_error("Unclosed [")),
                    }
                    node = JsepNode::Member {
                        object: Box::new(node),
                        property: Box::new(property),
                        computed: true,
                    };
                }
                Some(Token::Op(op)) if op == "(" => {
                    self.advance();
                    let mut arguments = Vec::new();
                    let mut first = true;
                    loop {
                        match self.peek() {
                            Some(Token::Op(op)) if op == ")" => {
                                self.advance();
                                break;
                            }
                            Some(Token::Op(op)) if op == "," && !first => {
                                self.advance();
                            }
                            None => return Err(runtime_error("Unclosed (")),
                            _ => {}
                        }
                        if matches!(self.peek(), Some(Token::Op(op)) if op == ")") {
                            continue;
                        }
                        arguments.push(self.parse_expression(0)?);
                        first = false;
                    }
                    node = JsepNode::Call {
                        callee: Box::new(node),
                        arguments,
                    };
                }
                _ => break,
            }
        }
        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<JsepNode, RuntimeError> {
        match self.advance() {
            Some(Token::Number(value)) => Ok(JsepNode::Literal(JsepLiteral::Number(value))),
            Some(Token::Str(value)) => Ok(JsepNode::Literal(JsepLiteral::Str(value))),
            Some(Token::Ident(name)) => match name.as_str() {
                "true" => Ok(JsepNode::Literal(JsepLiteral::Boolean(true))),
                "false" => Ok(JsepNode::Literal(JsepLiteral::Boolean(false))),
                "null" => Ok(JsepNode::Literal(JsepLiteral::Null)),
                "this" => Ok(JsepNode::ThisExpression),
                _ => Ok(JsepNode::Identifier(name)),
            },
            Some(Token::Op(op)) if op == "(" => {
                let node = self.parse_expression(0)?;
                match self.advance() {
                    Some(Token::Op(op)) if op == ")" => Ok(node),
                    _ => Err(runtime_error("Unclosed (")),
                }
            }
            Some(Token::Op(op)) if op == "[" => {
                let mut elements = Vec::new();
                let mut first = true;
                loop {
                    match self.peek() {
                        Some(Token::Op(op)) if op == "]" => {
                            self.advance();
                            break;
                        }
                        Some(Token::Op(op)) if op == "," && !first => {
                            self.advance();
                        }
                        None => return Err(runtime_error("Unclosed [")),
                        _ => {}
                    }
                    if matches!(self.peek(), Some(Token::Op(op)) if op == "]") {
                        continue;
                    }
                    elements.push(self.parse_expression(0)?);
                    first = false;
                }
                Ok(JsepNode::Array(elements))
            }
            Some(token) => Err(runtime_error(&format!("Unexpected {token:?}"))),
            None => Err(runtime_error("Provide exactly one expression.")),
        }
    }
}

// ---------------------------------------------------------------------------
// createRuntimeAst (jsep AST -> runtime nodes)
// ---------------------------------------------------------------------------

const UNARY_OPERATORS: [&str; 3] = ["!", "-", "+"];
const BINARY_OPERATORS: [&str; 15] = [
    "+", "-", "*", "/", "%", "===", "!==", ">", ">=", "<", "<=", "&&", "||", "!~", "=~",
];

/// Mirrors `parseLiteral`.
fn parse_literal(literal: &JsepLiteral) -> Node {
    match literal {
        JsepLiteral::Null => Node::new(
            ExpressionNodeType::LiteralNull,
            NodeValue::Null,
            None,
            None,
            None,
        ),
        JsepLiteral::Boolean(value) => Node::new(
            ExpressionNodeType::LiteralBoolean,
            NodeValue::Bool(*value),
            None,
            None,
            None,
        ),
        JsepLiteral::Number(value) => Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(*value),
            None,
            None,
            None,
        ),
        JsepLiteral::Str(value) => {
            if value.contains("${") {
                Node::new(
                    ExpressionNodeType::VariableInString,
                    NodeValue::Str(value.clone()),
                    None,
                    None,
                    None,
                )
            } else {
                Node::new(
                    ExpressionNodeType::LiteralString,
                    NodeValue::Str(replace_backslashes(value)),
                    None,
                    None,
                    None,
                )
            }
        }
    }
}

fn is_variable(name: &str) -> bool {
    name.starts_with("czm_")
}

fn get_property_name(variable: &str) -> &str {
    &variable[4..]
}

/// Mirrors `parseKeywordsAndVariables`.
fn parse_keywords_and_variables(name: &str) -> Result<Node, RuntimeError> {
    if is_variable(name) {
        let property = get_property_name(name);
        if property.starts_with("tiles3d_") {
            return Ok(Node::new(
                ExpressionNodeType::BuiltinVariable,
                NodeValue::Str(property.to_string()),
                None,
                None,
                None,
            ));
        }
        return Ok(Node::new(
            ExpressionNodeType::Variable,
            NodeValue::Str(property.to_string()),
            None,
            None,
            None,
        ));
    } else if name == "NaN" {
        return Ok(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::NAN),
            None,
            None,
            None,
        ));
    } else if name == "Infinity" {
        return Ok(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::INFINITY),
            None,
            None,
            None,
        ));
    } else if name == "undefined" {
        return Ok(Node::new(
            ExpressionNodeType::LiteralUndefined,
            NodeValue::Undefined,
            None,
            None,
            None,
        ));
    }

    Err(runtime_error(&format!("{name} is not defined.")))
}

/// Mirrors `parseMathConstant` (returns None for unknown constants, like the
/// original returns undefined).
fn parse_math_constant(name: &str) -> Option<Node> {
    if name == "PI" {
        Some(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(std::f64::consts::PI),
            None,
            None,
            None,
        ))
    } else if name == "E" {
        Some(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(std::f64::consts::E),
            None,
            None,
            None,
        ))
    } else {
        None
    }
}

/// Mirrors `parseNumberConstant`.
fn parse_number_constant(name: &str) -> Option<Node> {
    if name == "POSITIVE_INFINITY" {
        Some(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::INFINITY),
            None,
            None,
            None,
        ))
    } else {
        None
    }
}

fn is_unary_function(call: &str) -> bool {
    matches!(
        call,
        "abs" | "sqrt" | "cos" | "sin" | "tan" | "acos" | "asin" | "atan" | "radians"
            | "degrees" | "sign" | "floor" | "ceil" | "round" | "exp" | "exp2" | "log"
            | "log2" | "fract" | "length" | "normalize"
    )
}

fn is_binary_function(call: &str) -> bool {
    matches!(call, "atan2" | "pow" | "min" | "max" | "distance" | "dot" | "cross")
}

fn is_ternary_function(call: &str) -> bool {
    matches!(call, "clamp" | "mix")
}

/// Mirrors `parseMemberExpression`.
fn parse_member_expression(
    object: &JsepNode,
    property: &JsepNode,
    computed: bool,
) -> Result<Node, RuntimeError> {
    let object_name = match object {
        JsepNode::Identifier(name) => Some(name.as_str()),
        _ => None,
    };
    let property_name = match property {
        JsepNode::Identifier(name) => Some(name.as_str()),
        _ => None,
    };
    if object_name == Some("Math") {
        if let Some(name) = property_name {
            if let Some(node) = parse_math_constant(name) {
                return Ok(node);
            }
        }
        return Err(runtime_error("Cannot parse expression."));
    } else if object_name == Some("Number") {
        if let Some(name) = property_name {
            if let Some(node) = parse_number_constant(name) {
                return Ok(node);
            }
        }
        return Err(runtime_error("Cannot parse expression."));
    }

    let obj = create_runtime_ast(object)?;
    if computed {
        let val = create_runtime_ast(property)?;
        return Ok(Node::new(
            ExpressionNodeType::Member,
            NodeValue::Str("brackets".to_string()),
            Some(obj),
            Some(val),
            None,
        ));
    }

    let property_name = match property_name {
        Some(name) => name,
        None => return Err(runtime_error("Cannot parse expression.")),
    };
    let val = Node::new(
        ExpressionNodeType::LiteralString,
        NodeValue::Str(property_name.to_string()),
        None,
        None,
        None,
    );
    Ok(Node::new(
        ExpressionNodeType::Member,
        NodeValue::Str("dot".to_string()),
        Some(obj),
        Some(val),
        None,
    ))
}

/// String form of a literal node value, mirroring `String(pattern._value)`
/// in `parseRegex`.
fn node_value_string(node: &Node) -> String {
    match &node.value {
        NodeValue::Null => "null".to_string(),
        NodeValue::Undefined => "undefined".to_string(),
        NodeValue::Bool(value) => value.to_string(),
        NodeValue::Number(value) => number_to_js_string(*value),
        NodeValue::Str(value) => replace_backslashes(value),
        NodeValue::None | NodeValue::Nodes(_) | NodeValue::Regex(_) => String::new(),
    }
}

/// Mirrors `parseRegex`.
fn parse_regex(arguments: &[JsepNode]) -> Result<Node, RuntimeError> {
    // no arguments, return default regex
    if arguments.is_empty() {
        let regex = RegExpValue::compile("(?:)", "")?;
        return Ok(Node::new(
            ExpressionNodeType::LiteralRegex,
            NodeValue::Regex(regex),
            None,
            None,
            None,
        ));
    }

    let pattern = create_runtime_ast(&arguments[0])?;

    // optional flag argument supplied
    if arguments.len() > 1 {
        let flags = create_runtime_ast(&arguments[1])?;
        if pattern.node_type.is_literal_type() && flags.node_type.is_literal_type() {
            let regex = RegExpValue::compile(
                &node_value_string(&pattern),
                &node_value_string(&flags),
            )?;
            return Ok(Node::new(
                ExpressionNodeType::LiteralRegex,
                NodeValue::Regex(regex),
                None,
                None,
                None,
            ));
        }
        return Ok(Node::new(
            ExpressionNodeType::Regex,
            NodeValue::None,
            Some(pattern),
            Some(flags),
            None,
        ));
    }

    // only pattern argument supplied
    if pattern.node_type.is_literal_type() {
        let regex = RegExpValue::compile(&node_value_string(&pattern), "")?;
        return Ok(Node::new(
            ExpressionNodeType::LiteralRegex,
            NodeValue::Regex(regex),
            None,
            None,
            None,
        ));
    }
    Ok(Node::new(
        ExpressionNodeType::Regex,
        NodeValue::None,
        Some(pattern),
        None,
        None,
    ))
}

/// Mirrors `parseCall`.
fn parse_call(callee: &JsepNode, arguments: &[JsepNode]) -> Result<Node, RuntimeError> {
    let args_length = arguments.len();

    // Member function calls
    if let JsepNode::Member {
        object,
        property,
        computed: false,
    } = callee
    {
        let call = match property.as_ref() {
            JsepNode::Identifier(name) => name.as_str(),
            _ => return Err(runtime_error("Cannot parse expression.")),
        };
        if call == "test" || call == "exec" {
            // Make sure this is called on a valid type
            let is_reg_exp = matches!(
                object.as_ref(),
                JsepNode::Call { callee, .. }
                    if matches!(callee.as_ref(), JsepNode::Identifier(name) if name == "regExp")
            );
            if !is_reg_exp {
                return Err(runtime_error(&format!("{call} is not a function.")));
            }
            if args_length == 0 {
                if call == "test" {
                    return Ok(Node::new(
                        ExpressionNodeType::LiteralBoolean,
                        NodeValue::Bool(false),
                        None,
                        None,
                        None,
                    ));
                }
                return Ok(Node::new(
                    ExpressionNodeType::LiteralNull,
                    NodeValue::Null,
                    None,
                    None,
                    None,
                ));
            }
            let left = create_runtime_ast(object)?;
            let right = create_runtime_ast(&arguments[0])?;
            return Ok(Node::new(
                ExpressionNodeType::FunctionCall,
                NodeValue::Str(call.to_string()),
                Some(left),
                Some(right),
                None,
            ));
        } else if call == "toString" {
            let val = create_runtime_ast(object)?;
            return Ok(Node::new(
                ExpressionNodeType::FunctionCall,
                NodeValue::Str("toString".to_string()),
                Some(val),
                None,
                None,
            ));
        }

        return Err(runtime_error(&format!(
            "Unexpected function call \"{call}\"."
        )));
    }

    // Non-member function calls
    let call = match callee {
        JsepNode::Identifier(name) => name.as_str(),
        _ => return Err(runtime_error("Unexpected function call.")),
    };
    if call == "color" {
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralColor,
                NodeValue::Str("color".to_string()),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        let mut children = vec![val];
        if args_length > 1 {
            children.push(create_runtime_ast(&arguments[1])?);
        }
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralColor,
            NodeValue::Str("color".to_string()),
            children,
        ));
    } else if call == "rgb" || call == "hsl" {
        if args_length < 3 {
            return Err(runtime_error(&format!("{call} requires three arguments.")));
        }
        let children = vec![
            create_runtime_ast(&arguments[0])?,
            create_runtime_ast(&arguments[1])?,
            create_runtime_ast(&arguments[2])?,
        ];
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralColor,
            NodeValue::Str(call.to_string()),
            children,
        ));
    } else if call == "rgba" || call == "hsla" {
        if args_length < 4 {
            return Err(runtime_error(&format!("{call} requires four arguments.")));
        }
        let children = vec![
            create_runtime_ast(&arguments[0])?,
            create_runtime_ast(&arguments[1])?,
            create_runtime_ast(&arguments[2])?,
            create_runtime_ast(&arguments[3])?,
        ];
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralColor,
            NodeValue::Str(call.to_string()),
            children,
        ));
    } else if call == "vec2" || call == "vec3" || call == "vec4" {
        // Check for invalid constructors at evaluation time
        let mut children = Vec::with_capacity(args_length);
        for argument in arguments {
            children.push(create_runtime_ast(argument)?);
        }
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralVector,
            NodeValue::Str(call.to_string()),
            children,
        ));
    } else if call == "isNaN" || call == "isFinite" {
        if args_length == 0 {
            let value = call == "isNaN";
            return Ok(Node::new(
                ExpressionNodeType::LiteralBoolean,
                NodeValue::Bool(value),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "isExactClass" || call == "isClass" {
        if args_length != 1 {
            return Err(runtime_error(&format!(
                "{call} requires exactly one argument."
            )));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "getExactClassName" {
        if !arguments.is_empty() {
            return Err(runtime_error(&format!(
                "{call} does not take any argument."
            )));
        }
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            None,
            None,
            None,
        ));
    } else if is_unary_function(call) {
        if args_length != 1 {
            return Err(runtime_error(&format!(
                "{call} requires exactly one argument."
            )));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            Some(val),
            None,
            None,
        ));
    } else if is_binary_function(call) {
        if args_length != 2 {
            return Err(runtime_error(&format!(
                "{call} requires exactly two arguments."
            )));
        }
        let left = create_runtime_ast(&arguments[0])?;
        let right = create_runtime_ast(&arguments[1])?;
        return Ok(Node::new(
            ExpressionNodeType::Binary,
            NodeValue::Str(call.to_string()),
            Some(left),
            Some(right),
            None,
        ));
    } else if is_ternary_function(call) {
        if args_length != 3 {
            return Err(runtime_error(&format!(
                "{call} requires exactly three arguments."
            )));
        }
        let left = create_runtime_ast(&arguments[0])?;
        let right = create_runtime_ast(&arguments[1])?;
        let test = create_runtime_ast(&arguments[2])?;
        return Ok(Node::new(
            ExpressionNodeType::Ternary,
            NodeValue::Str(call.to_string()),
            Some(left),
            Some(right),
            Some(test),
        ));
    } else if call == "Boolean" {
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralBoolean,
                NodeValue::Bool(false),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("Boolean".to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "Number" {
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralNumber,
                NodeValue::Number(0.0),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("Number".to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "String" {
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralString,
                NodeValue::Str(String::new()),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("String".to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "regExp" {
        return parse_regex(arguments);
    }

    Err(runtime_error(&format!(
        "Unexpected function call \"{call}\"."
    )))
}

/// Mirrors `createRuntimeAst`.
fn create_runtime_ast(ast: &JsepNode) -> Result<Node, RuntimeError> {
    match ast {
        JsepNode::Literal(literal) => Ok(parse_literal(literal)),
        JsepNode::Call { callee, arguments } => parse_call(callee, arguments),
        JsepNode::Identifier(name) => parse_keywords_and_variables(name),
        JsepNode::ThisExpression => parse_keywords_and_variables("this"),
        JsepNode::Unary { operator, argument } => {
            let child = create_runtime_ast(argument)?;
            if UNARY_OPERATORS.contains(&operator.as_str()) {
                Ok(Node::new(
                    ExpressionNodeType::Unary,
                    NodeValue::Str(operator.clone()),
                    Some(child),
                    None,
                    None,
                ))
            } else {
                Err(runtime_error(&format!(
                    "Unexpected operator \"{operator}\"."
                )))
            }
        }
        JsepNode::Binary {
            operator,
            left,
            right,
        } => {
            let left = create_runtime_ast(left)?;
            let right = create_runtime_ast(right)?;
            if BINARY_OPERATORS.contains(&operator.as_str()) {
                Ok(Node::new(
                    ExpressionNodeType::Binary,
                    NodeValue::Str(operator.clone()),
                    Some(left),
                    Some(right),
                    None,
                ))
            } else {
                Err(runtime_error(&format!(
                    "Unexpected operator \"{operator}\"."
                )))
            }
        }
        JsepNode::Conditional {
            test,
            consequent,
            alternate,
        } => {
            let test = create_runtime_ast(test)?;
            let left = create_runtime_ast(consequent)?;
            let right = create_runtime_ast(alternate)?;
            Ok(Node::new(
                ExpressionNodeType::Conditional,
                NodeValue::Str("?".to_string()),
                Some(left),
                Some(right),
                Some(test),
            ))
        }
        JsepNode::Member {
            object,
            property,
            computed,
        } => parse_member_expression(object, property, *computed),
        JsepNode::Array(elements) => {
            let mut children = Vec::with_capacity(elements.len());
            for element in elements {
                children.push(create_runtime_ast(element)?);
            }
            Ok(Node::new(
                ExpressionNodeType::Array,
                NodeValue::Nodes(children),
                None,
                None,
                None,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluation (mirrors the per-node `evaluate` functions)
// ---------------------------------------------------------------------------

/// Mirrors JS `Math.round` (halves round towards +infinity).
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// Mirrors JS `Math.min` NaN propagation.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// Mirrors JS `Math.max` NaN propagation.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn unary_arg_error(call: &str, value: &Value) -> RuntimeError {
    runtime_error(&format!(
        "Function \"{call}\" requires a vector or number argument. Argument is {value}."
    ))
}

fn binary_arg_error(call: &str, left: &Value, right: &Value) -> RuntimeError {
    runtime_error(&format!(
        "Function \"{call}\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
    ))
}

fn ternary_arg_error(call: &str, left: &Value, right: &Value, test: &Value) -> RuntimeError {
    runtime_error(&format!(
        "Function \"{call}\" requires vector or number arguments of matching types. Arguments are {left}, {right}, and {test}."
    ))
}

/// Mirrors `getEvaluateUnaryComponentwise` + `length`/`normalize`.
fn evaluate_unary_function(call: &str, left: Value) -> Result<Value, RuntimeError> {
    if call == "length" {
        return match left {
            Value::Number(n) => Ok(Value::Number(n.abs())),
            Value::Cartesian2(v) => Ok(Value::Number(Cartesian2::magnitude(&v))),
            Value::Cartesian3(v) => Ok(Value::Number(Cartesian3::magnitude(&v))),
            Value::Cartesian4(v) => Ok(Value::Number(Cartesian4::magnitude(&v))),
            _ => Err(unary_arg_error(call, &left)),
        };
    }
    if call == "normalize" {
        return match left {
            Value::Number(_) => Ok(Value::Number(1.0)),
            Value::Cartesian2(v) => Ok(Value::Cartesian2(Cartesian2::multiply_by_scalar_new(
                &v,
                1.0 / Cartesian2::magnitude(&v),
            ))),
            Value::Cartesian3(v) => Ok(Value::Cartesian3(Cartesian3::multiply_by_scalar_new(
                &v,
                1.0 / Cartesian3::magnitude(&v),
            ))),
            Value::Cartesian4(v) => Ok(Value::Cartesian4(Cartesian4::multiply_by_scalar_new(
                &v,
                1.0 / Cartesian4::magnitude(&v),
            ))),
            _ => Err(unary_arg_error(call, &left)),
        };
    }

    let operation: fn(f64) -> f64 = match call {
        "abs" => f64::abs,
        "sqrt" => f64::sqrt,
        "cos" => f64::cos,
        "sin" => f64::sin,
        "tan" => f64::tan,
        "acos" => f64::acos,
        "asin" => f64::asin,
        "atan" => f64::atan,
        "radians" => CesiumMath::to_radians,
        "degrees" => CesiumMath::to_degrees,
        "sign" => CesiumMath::sign,
        "floor" => f64::floor,
        "ceil" => f64::ceil,
        "round" => js_round,
        "exp" => f64::exp,
        "exp2" => |x| 2.0_f64.powf(x),
        "log" => f64::ln,
        "log2" => CesiumMath::log2,
        "fract" => |x| x - x.floor(),
        _ => {
            return Err(runtime_error(&format!(
                "Unexpected function call \"{call}\"."
            )))
        }
    };

    match left {
        Value::Number(n) => Ok(Value::Number(operation(n))),
        Value::Cartesian2(v) => Ok(Value::Cartesian2(Cartesian2::from_elements_new(
            operation(v.x),
            operation(v.y),
        ))),
        Value::Cartesian3(v) => Ok(Value::Cartesian3(Cartesian3::from_elements_new(
            operation(v.x),
            operation(v.y),
            operation(v.z),
        ))),
        Value::Cartesian4(v) => Ok(Value::Cartesian4(Cartesian4::from_elements_new(
            operation(v.x),
            operation(v.y),
            operation(v.z),
            operation(v.w),
        ))),
        _ => Err(unary_arg_error(call, &left)),
    }
}

/// Mirrors `getEvaluateBinaryComponentwise` + `distance`/`dot`/`cross`.
fn evaluate_binary_function(
    call: &str,
    left: Value,
    right: Value,
) -> Result<Value, RuntimeError> {
    if call == "distance" {
        return match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number((l - r).abs())),
            (Value::Cartesian2(l), Value::Cartesian2(r)) => {
                Ok(Value::Number(Cartesian2::distance(l, r)))
            }
            (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                Ok(Value::Number(Cartesian3::distance(l, r)))
            }
            (Value::Cartesian4(l), Value::Cartesian4(r)) => {
                Ok(Value::Number(Cartesian4::distance(l, r)))
            }
            _ => Err(binary_arg_error(call, &left, &right)),
        };
    }
    if call == "dot" {
        return match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Cartesian2(l), Value::Cartesian2(r)) => {
                Ok(Value::Number(Cartesian2::dot(l, r)))
            }
            (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                Ok(Value::Number(Cartesian3::dot(l, r)))
            }
            (Value::Cartesian4(l), Value::Cartesian4(r)) => {
                Ok(Value::Number(Cartesian4::dot(l, r)))
            }
            _ => Err(binary_arg_error(call, &left, &right)),
        };
    }
    if call == "cross" {
        return match (&left, &right) {
            (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                Ok(Value::Cartesian3(Cartesian3::cross_new(l, r)))
            }
            _ => Err(runtime_error(&format!(
                "Function \"{call}\" requires vec3 arguments. Arguments are {left} and {right}."
            ))),
        };
    }

    let (operation, allow_scalar): (fn(f64, f64) -> f64, bool) = match call {
        "atan2" => (f64::atan2, false),
        "pow" => (f64::powf, false),
        "min" => (js_min, true),
        "max" => (js_max, true),
        _ => {
            return Err(runtime_error(&format!(
                "Unexpected function call \"{call}\"."
            )))
        }
    };

    if allow_scalar {
        if let Value::Number(r) = &right {
            match &left {
                Value::Number(l) => return Ok(Value::Number(operation(*l, *r))),
                Value::Cartesian2(v) => {
                    return Ok(Value::Cartesian2(Cartesian2::from_elements_new(
                        operation(v.x, *r),
                        operation(v.y, *r),
                    )))
                }
                Value::Cartesian3(v) => {
                    return Ok(Value::Cartesian3(Cartesian3::from_elements_new(
                        operation(v.x, *r),
                        operation(v.y, *r),
                        operation(v.z, *r),
                    )))
                }
                Value::Cartesian4(v) => {
                    return Ok(Value::Cartesian4(Cartesian4::from_elements_new(
                        operation(v.x, *r),
                        operation(v.y, *r),
                        operation(v.z, *r),
                        operation(v.w, *r),
                    )))
                }
                _ => {}
            }
        }
    }

    match (&left, &right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(operation(*l, *r))),
        (Value::Cartesian2(l), Value::Cartesian2(r)) => {
            Ok(Value::Cartesian2(Cartesian2::from_elements_new(
                operation(l.x, r.x),
                operation(l.y, r.y),
            )))
        }
        (Value::Cartesian3(l), Value::Cartesian3(r)) => {
            Ok(Value::Cartesian3(Cartesian3::from_elements_new(
                operation(l.x, r.x),
                operation(l.y, r.y),
                operation(l.z, r.z),
            )))
        }
        (Value::Cartesian4(l), Value::Cartesian4(r)) => {
            Ok(Value::Cartesian4(Cartesian4::from_elements_new(
                operation(l.x, r.x),
                operation(l.y, r.y),
                operation(l.z, r.z),
                operation(l.w, r.w),
            )))
        }
        _ => Err(binary_arg_error(call, &left, &right)),
    }
}

/// Mirrors `getEvaluateTernaryComponentwise` (`clamp`/`mix`).
fn evaluate_ternary_function(
    call: &str,
    left: Value,
    right: Value,
    test: Value,
) -> Result<Value, RuntimeError> {
    let operation: fn(f64, f64, f64) -> f64 = match call {
        "clamp" => CesiumMath::clamp,
        "mix" => CesiumMath::lerp,
        _ => {
            return Err(runtime_error(&format!(
                "Unexpected function call \"{call}\"."
            )))
        }
    };

    // allowScalar: a scalar `test` applies componentwise.
    if let Value::Number(t) = &test {
        match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => {
                return Ok(Value::Number(operation(*l, *r, *t)))
            }
            (Value::Cartesian2(l), Value::Cartesian2(r)) => {
                return Ok(Value::Cartesian2(Cartesian2::from_elements_new(
                    operation(l.x, r.x, *t),
                    operation(l.y, r.y, *t),
                )))
            }
            (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                return Ok(Value::Cartesian3(Cartesian3::from_elements_new(
                    operation(l.x, r.x, *t),
                    operation(l.y, r.y, *t),
                    operation(l.z, r.z, *t),
                )))
            }
            (Value::Cartesian4(l), Value::Cartesian4(r)) => {
                return Ok(Value::Cartesian4(Cartesian4::from_elements_new(
                    operation(l.x, r.x, *t),
                    operation(l.y, r.y, *t),
                    operation(l.z, r.z, *t),
                    operation(l.w, r.w, *t),
                )))
            }
            _ => {}
        }
    }

    match (&left, &right, &test) {
        (Value::Number(l), Value::Number(r), Value::Number(t)) => {
            Ok(Value::Number(operation(*l, *r, *t)))
        }
        (Value::Cartesian2(l), Value::Cartesian2(r), Value::Cartesian2(t)) => {
            Ok(Value::Cartesian2(Cartesian2::from_elements_new(
                operation(l.x, r.x, t.x),
                operation(l.y, r.y, t.y),
            )))
        }
        (Value::Cartesian3(l), Value::Cartesian3(r), Value::Cartesian3(t)) => {
            Ok(Value::Cartesian3(Cartesian3::from_elements_new(
                operation(l.x, r.x, t.x),
                operation(l.y, r.y, t.y),
                operation(l.z, r.z, t.z),
            )))
        }
        (Value::Cartesian4(l), Value::Cartesian4(r), Value::Cartesian4(t)) => {
            Ok(Value::Cartesian4(Cartesian4::from_elements_new(
                operation(l.x, r.x, t.x),
                operation(l.y, r.y, t.y),
                operation(l.z, r.z, t.z),
                operation(l.w, r.w, t.w),
            )))
        }
        _ => Err(ternary_arg_error(call, &left, &right, &test)),
    }
}

/// Component access on vectors, mirroring the `.r/.g/.b/.a`, `.x/.y/.z/.w`
/// and `[0]-[3]` member handling.
fn vector_component(property: &Value, member: &Value) -> Option<Value> {
    let name: &str = match member {
        Value::Number(n) => match n {
            0.0 => "x",
            1.0 => "y",
            2.0 => "z",
            3.0 => "w",
            _ => return None,
        },
        Value::String(s) => s.as_str(),
        _ => return None,
    };
    match property {
        Value::Cartesian2(v) => match name {
            "r" | "x" => Some(Value::Number(v.x)),
            "g" | "y" => Some(Value::Number(v.y)),
            _ => None,
        },
        Value::Cartesian3(v) => match name {
            "r" | "x" => Some(Value::Number(v.x)),
            "g" | "y" => Some(Value::Number(v.y)),
            "b" | "z" => Some(Value::Number(v.z)),
            _ => None,
        },
        Value::Cartesian4(v) => match name {
            "r" | "x" => Some(Value::Number(v.x)),
            "g" | "y" => Some(Value::Number(v.y)),
            "b" | "z" => Some(Value::Number(v.z)),
            "a" | "w" => Some(Value::Number(v.w)),
            _ => None,
        },
        _ => None,
    }
}

/// Generic member access for arrays and strings (JS `property[member]`).
fn member_access(property: &Value, member: &Value) -> Value {
    match property {
        Value::Array(items) => {
            let index = match member {
                Value::Number(n) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as usize),
                Value::String(s) => s.parse::<usize>().ok(),
                _ => None,
            };
            index
                .and_then(|i| items.get(i).cloned())
                .unwrap_or(Value::Undefined)
        }
        Value::String(s) => match member {
            Value::String(m) if m == "length" => Value::Number(s.chars().count() as f64),
            Value::String(m) => m
                .parse::<usize>()
                .ok()
                .and_then(|i| s.chars().nth(i))
                .map(|c| Value::String(c.to_string()))
                .unwrap_or(Value::Undefined),
            Value::Number(n) if *n >= 0.0 && n.fract() == 0.0 => s
                .chars()
                .nth(*n as usize)
                .map(|c| Value::String(c.to_string()))
                .unwrap_or(Value::Undefined),
            _ => Value::Undefined,
        },
        _ => Value::Undefined,
    }
}

/// Mirrors `Color.fromBytes` argument clamping (`CesiumMath.clamp` to byte).
fn to_byte(value: f64) -> u8 {
    CesiumMath::clamp(value, 0.0, 255.0).round() as u8
}

/// Mirrors `_evaluateLiteralColor`.
fn evaluate_literal_color(
    name: &str,
    args: Option<&[Node]>,
    feature: Option<&dyn ExpressionFeature>,
) -> Result<Value, RuntimeError> {
    let color = match name {
        "color" => match args {
            None => Color::from_bytes(255, 255, 255, 255),
            Some(a) if a.len() > 1 => {
                let css = a[0].evaluate(feature)?.string_conversion();
                let mut color = Color::from_css_color_string(&css).ok_or_else(|| {
                    runtime_error(&format!("{css} is not a valid color."))
                })?;
                color.alpha = a[1].evaluate(feature)?.number_conversion();
                color
            }
            Some(a) => {
                let css = a[0].evaluate(feature)?.string_conversion();
                Color::from_css_color_string(&css).ok_or_else(|| {
                    runtime_error(&format!("{css} is not a valid color."))
                })?
            }
        },
        "rgb" => {
            let a = args.expect("rgb requires arguments");
            // Mirrors Color.fromBytes(r, g, b, 255): byteToFloat is a plain
            // divide by 255 with no rounding/clamping.
            Color::new(
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
            Color::new(
                a[0].evaluate(feature)?.number_conversion() / 255.0,
                a[1].evaluate(feature)?.number_conversion() / 255.0,
                a[2].evaluate(feature)?.number_conversion() / 255.0,
                alpha / 255.0,
            )
        }
        "hsl" => {
            let a = args.expect("hsl requires arguments");
            Color::from_hsl(
                a[0].evaluate(feature)?.number_conversion(),
                a[1].evaluate(feature)?.number_conversion(),
                a[2].evaluate(feature)?.number_conversion(),
                1.0,
            )
        }
        "hsla" => {
            let a = args.expect("hsla requires arguments");
            Color::from_hsl(
                a[0].evaluate(feature)?.number_conversion(),
                a[1].evaluate(feature)?.number_conversion(),
                a[2].evaluate(feature)?.number_conversion(),
                a[3].evaluate(feature)?.number_conversion(),
            )
        }
        _ => unreachable!("literal color name is one of color/rgb/rgba/hsl/hsla"),
    };
    Ok(Value::Cartesian4(Cartesian4::from_elements_new(
        color.red,
        color.green,
        color.blue,
        color.alpha,
    )))
}

/// Mirrors `_evaluateLiteralVector`.
fn evaluate_literal_vector(
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
        Ok(Value::Cartesian2(Cartesian2::from_array_new(
            &components,
            None,
        )))
    } else if call == "vec3" {
        Ok(Value::Cartesian3(Cartesian3::from_array_new(
            &components,
            None,
        )))
    } else {
        Ok(Value::Cartesian4(Cartesian4::from_array_new(
            &components,
            None,
        )))
    }
}

impl Node {
    /// Mirrors the per-node `evaluate` functions assigned by
    /// `setEvaluateFunction`.
    pub fn evaluate(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        match self.node_type {
            ExpressionNodeType::Conditional => {
                let test = self.test.as_ref().unwrap().evaluate(feature)?;
                let Value::Boolean(test) = test else {
                    return Err(runtime_error(&format!(
                        "Conditional argument of conditional expression must be a boolean. Argument is {test}."
                    )));
                };
                if test {
                    self.left.as_ref().unwrap().evaluate(feature)
                } else {
                    self.right.as_ref().unwrap().evaluate(feature)
                }
            }
            ExpressionNodeType::FunctionCall => {
                let call = match &self.value {
                    NodeValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                match call.as_str() {
                    "test" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        let right = self.right.as_ref().unwrap().evaluate(feature)?;
                        match (&left, &right) {
                            (Value::RegExp(regex), Value::String(text)) => {
                                Ok(Value::Boolean(regex.test(text)))
                            }
                            _ => Err(runtime_error(&format!(
                                "RegExp.test requires the first argument to be a RegExp and the second argument to be a string. Arguments are {left} and {right}."
                            ))),
                        }
                    }
                    "exec" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        let right = self.right.as_ref().unwrap().evaluate(feature)?;
                        match (&left, &right) {
                            (Value::RegExp(regex), Value::String(text)) => {
                                Ok(match regex.exec_first_capture(text) {
                                    Some(capture) => Value::String(capture),
                                    None => Value::Null,
                                })
                            }
                            _ => Err(runtime_error(&format!(
                                "RegExp.exec requires the first argument to be a RegExp and the second argument to be a string. Arguments are {left} and {right}."
                            ))),
                        }
                    }
                    "toString" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        match &left {
                            Value::RegExp(_)
                            | Value::Cartesian2(_)
                            | Value::Cartesian3(_)
                            | Value::Cartesian4(_) => Ok(Value::String(left.string_conversion())),
                            _ => Err(runtime_error(&format!(
                                "Unexpected function call \"{call}\"."
                            ))),
                        }
                    }
                    _ => Err(runtime_error(&format!(
                        "Unexpected function call \"{call}\"."
                    ))),
                }
            }
            ExpressionNodeType::Unary => self.evaluate_unary(feature),
            ExpressionNodeType::Binary => self.evaluate_binary(feature),
            ExpressionNodeType::Ternary => {
                let call = match &self.value {
                    NodeValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let left = self.left.as_ref().unwrap().evaluate(feature)?;
                let right = self.right.as_ref().unwrap().evaluate(feature)?;
                let test = self.test.as_ref().unwrap().evaluate(feature)?;
                evaluate_ternary_function(&call, left, right, test)
            }
            ExpressionNodeType::Member => {
                let is_brackets = matches!(&self.value, NodeValue::Str(s) if s == "brackets");
                let left_node = self.left.as_ref().unwrap();
                if check_feature(left_node) {
                    let name = self.right.as_ref().unwrap().evaluate(feature)?;
                    return Ok(get_feature_property(
                        feature,
                        &name.string_conversion(),
                    ));
                }
                let property = left_node.evaluate(feature)?;
                if !property.is_defined() {
                    return Ok(Value::Undefined);
                }
                let member = self.right.as_ref().unwrap().evaluate(feature)?;
                if let Some(component) = vector_component(&property, &member) {
                    return Ok(component);
                }
                let _ = is_brackets;
                Ok(member_access(&property, &member))
            }
            ExpressionNodeType::Array => {
                let NodeValue::Nodes(nodes) = &self.value else {
                    return Ok(Value::Array(Vec::new()));
                };
                let mut array = Vec::with_capacity(nodes.len());
                for node in nodes {
                    array.push(node.evaluate(feature)?);
                }
                Ok(Value::Array(array))
            }
            ExpressionNodeType::Variable => {
                let name = match &self.value {
                    NodeValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                Ok(get_feature_property(feature, &name))
            }
            ExpressionNodeType::VariableInString => {
                let template = match &self.value {
                    NodeValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let pattern =
                    Regex::new(VARIABLE_PATTERN).expect("variable pattern is valid");
                let mut result = String::new();
                let mut last = 0usize;
                for captures in pattern.captures_iter(&template) {
                    let whole = captures.get(0).unwrap();
                    result.push_str(&template[last..whole.start()]);
                    let property = get_feature_property(feature, &captures[1]);
                    if property.is_defined() {
                        result.push_str(&property.string_conversion());
                    }
                    last = whole.end();
                }
                result.push_str(&template[last..]);
                Ok(Value::String(result))
            }
            ExpressionNodeType::LiteralColor => {
                let name = match &self.value {
                    NodeValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                evaluate_literal_color(&name, self.left_children.as_deref(), feature)
            }
            ExpressionNodeType::LiteralVector => {
                let call = match &self.value {
                    NodeValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                match &self.left_children {
                    Some(args) => evaluate_literal_vector(&call, args, feature),
                    None => Err(runtime_error(&format!(
                        "Invalid {call} constructor. No valid arguments."
                    ))),
                }
            }
            ExpressionNodeType::LiteralString => match &self.value {
                NodeValue::Str(s) => Ok(Value::String(s.clone())),
                _ => Ok(Value::String(String::new())),
            },
            ExpressionNodeType::Regex => {
                let pattern = self.left.as_ref().unwrap().evaluate(feature)?;
                let flags = match &self.right {
                    Some(flags) => flags.evaluate(feature)?.string_conversion(),
                    None => String::new(),
                };
                let regex = RegExpValue::compile(&pattern.string_conversion(), &flags)?;
                Ok(Value::RegExp(regex))
            }
            ExpressionNodeType::BuiltinVariable => {
                // DEVIATION: `tiles3d_tileset_time` reads
                // `feature.content.tileset.timeSinceLoad` in the original; the
                // CPU-side port has no tileset context, so it evaluates to 0.0
                // (the same value returned when the feature is undefined).
                Ok(Value::Number(0.0))
            }
            ExpressionNodeType::LiteralNull => Ok(Value::Null),
            ExpressionNodeType::LiteralBoolean => match &self.value {
                NodeValue::Bool(b) => Ok(Value::Boolean(*b)),
                _ => Ok(Value::Undefined),
            },
            ExpressionNodeType::LiteralNumber => match &self.value {
                NodeValue::Number(n) => Ok(Value::Number(*n)),
                _ => Ok(Value::Undefined),
            },
            ExpressionNodeType::LiteralRegex => match &self.value {
                NodeValue::Regex(regex) => Ok(Value::RegExp(regex.clone())),
                _ => Ok(Value::Undefined),
            },
            ExpressionNodeType::LiteralUndefined => Ok(Value::Undefined),
        }
    }

    /// UNARY node evaluation, mirroring `_evaluateNot`/`_evaluateNegative`/
    /// `_evaluatePositive`/conversion calls and the unary function table.
    fn evaluate_unary(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        let op = match &self.value {
            NodeValue::Str(s) => s.clone(),
            _ => String::new(),
        };
        if op == "getExactClassName" {
            return Ok(match feature {
                Some(feature) => feature
                    .get_exact_class_name()
                    .unwrap_or(Value::Undefined),
                None => Value::Undefined,
            });
        }
        let left = self.left.as_ref().unwrap().evaluate(feature)?;
        match op.as_str() {
            "!" => match left {
                Value::Boolean(b) => Ok(Value::Boolean(!b)),
                _ => Err(runtime_error(&format!(
                    "Operator \"!\" requires a boolean argument. Argument is {left}."
                ))),
            },
            "-" => match left {
                Value::Number(n) => Ok(Value::Number(-n)),
                Value::Cartesian2(v) => Ok(Value::Cartesian2(Cartesian2::negate_new(&v))),
                Value::Cartesian3(v) => Ok(Value::Cartesian3(Cartesian3::negate_new(&v))),
                Value::Cartesian4(v) => Ok(Value::Cartesian4(Cartesian4::negate_new(&v))),
                _ => Err(runtime_error(&format!(
                    "Operator \"-\" requires a vector or number argument. Argument is {left}."
                ))),
            },
            "+" => match &left {
                Value::Number(_)
                | Value::Cartesian2(_)
                | Value::Cartesian3(_)
                | Value::Cartesian4(_) => Ok(left),
                _ => Err(runtime_error(&format!(
                    "Operator \"+\" requires a vector or number argument. Argument is {left}."
                ))),
            },
            "isNaN" => Ok(Value::Boolean(left.number_conversion().is_nan())),
            "isFinite" => Ok(Value::Boolean({
                let n = left.number_conversion();
                !n.is_nan() && !n.is_infinite()
            })),
            "isExactClass" => Ok(Value::Boolean(match feature {
                Some(feature) => feature.is_exact_class(&left),
                None => false,
            })),
            "isClass" => Ok(Value::Boolean(match feature {
                Some(feature) => feature.is_class(&left),
                None => false,
            })),
            "Boolean" => Ok(Value::Boolean(left.boolean_conversion())),
            "Number" => Ok(Value::Number(left.number_conversion())),
            "String" => Ok(Value::String(left.string_conversion())),
            _ => evaluate_unary_function(&op, left),
        }
    }

    /// BINARY node evaluation, mirroring `_evaluatePlus`/.../`_evaluateOr`
    /// and the regex match operators.
    fn evaluate_binary(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        let op = match &self.value {
            NodeValue::Str(s) => s.clone(),
            _ => String::new(),
        };
        // Short-circuit operators evaluate the right side lazily.
        if op == "&&" || op == "||" {
            let left = self.left.as_ref().unwrap().evaluate(feature)?;
            let Value::Boolean(left) = left else {
                return Err(runtime_error(&format!(
                    "Operator \"{op}\" requires boolean arguments. First argument is {left}."
                )));
            };
            if op == "&&" && !left {
                return Ok(Value::Boolean(false));
            }
            if op == "||" && left {
                return Ok(Value::Boolean(true));
            }
            let right = self.right.as_ref().unwrap().evaluate(feature)?;
            let Value::Boolean(right) = right else {
                return Err(runtime_error(&format!(
                    "Operator \"{op}\" requires boolean arguments. Second argument is {right}."
                )));
            };
            return Ok(Value::Boolean(if op == "&&" { left && right } else { left || right }));
        }

        let left = self.left.as_ref().unwrap().evaluate(feature)?;
        let right = self.right.as_ref().unwrap().evaluate(feature)?;
        match op.as_str() {
            "+" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => {
                    Ok(Value::Cartesian2(Cartesian2::add_new(l, r)))
                }
                (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                    Ok(Value::Cartesian3(Cartesian3::add_new(l, r)))
                }
                (Value::Cartesian4(l), Value::Cartesian4(r)) => {
                    Ok(Value::Cartesian4(Cartesian4::add_new(l, r)))
                }
                (Value::String(_), _) | (_, Value::String(_)) => Ok(Value::String(format!(
                    "{}{}",
                    left.string_conversion(),
                    right.string_conversion()
                ))),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"+\" requires vector or number arguments of matching types, or at least one string argument. Arguments are {left} and {right}."
                ))),
            },
            "-" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => {
                    Ok(Value::Cartesian2(Cartesian2::subtract_new(l, r)))
                }
                (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                    Ok(Value::Cartesian3(Cartesian3::subtract_new(l, r)))
                }
                (Value::Cartesian4(l), Value::Cartesian4(r)) => {
                    Ok(Value::Cartesian4(Cartesian4::subtract_new(l, r)))
                }
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"-\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
                ))),
            },
            "*" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(
                    Cartesian2::multiply_components_new(l, r),
                )),
                (Value::Cartesian2(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian2(v)) => Ok(Value::Cartesian2(
                    Cartesian2::multiply_by_scalar_new(v, *n),
                )),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(
                    Cartesian3::multiply_components_new(l, r),
                )),
                (Value::Cartesian3(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian3(v)) => Ok(Value::Cartesian3(
                    Cartesian3::multiply_by_scalar_new(v, *n),
                )),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(
                    Cartesian4::multiply_components_new(l, r),
                )),
                (Value::Cartesian4(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian4(v)) => Ok(Value::Cartesian4(
                    Cartesian4::multiply_by_scalar_new(v, *n),
                )),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"*\" requires vector or number arguments. If both arguments are vectors they must be matching types. Arguments are {left} and {right}."
                ))),
            },
            "/" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(
                    Cartesian2::divide_components_new(l, r),
                )),
                (Value::Cartesian2(v), Value::Number(n)) => Ok(Value::Cartesian2(
                    Cartesian2::divide_by_scalar_new(v, *n),
                )),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(
                    Cartesian3::divide_components_new(l, r),
                )),
                (Value::Cartesian3(v), Value::Number(n)) => Ok(Value::Cartesian3(
                    Cartesian3::divide_by_scalar_new(v, *n),
                )),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(
                    Cartesian4::divide_components_new(l, r),
                )),
                (Value::Cartesian4(v), Value::Number(n)) => Ok(Value::Cartesian4(
                    Cartesian4::divide_by_scalar_new(v, *n),
                )),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l / r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"/\" requires vector or number arguments of matching types, or a number as the second argument. Arguments are {left} and {right}."
                ))),
            },
            "%" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => {
                    Ok(Value::Cartesian2(Cartesian2::from_elements_new(
                        l.x % r.x,
                        l.y % r.y,
                    )))
                }
                (Value::Cartesian3(l), Value::Cartesian3(r)) => {
                    Ok(Value::Cartesian3(Cartesian3::from_elements_new(
                        l.x % r.x,
                        l.y % r.y,
                        l.z % r.z,
                    )))
                }
                (Value::Cartesian4(l), Value::Cartesian4(r)) => {
                    Ok(Value::Cartesian4(Cartesian4::from_elements_new(
                        l.x % r.x,
                        l.y % r.y,
                        l.z % r.z,
                        l.w % r.w,
                    )))
                }
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l % r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"%\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
                ))),
            },
            "===" => Ok(Value::Boolean(left.equals_strict(&right))),
            "!==" => Ok(Value::Boolean(!left.equals_strict(&right))),
            "<" | "<=" | ">" | ">=" => match (&left, &right) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Boolean(match op.as_str() {
                    "<" => l < r,
                    "<=" => l <= r,
                    ">" => l > r,
                    _ => l >= r,
                })),
                _ => Err(runtime_error(&format!(
                    "Operator \"{op}\" requires number arguments. Arguments are {left} and {right}."
                ))),
            },
            "=~" | "!~" => {
                let matched = match (&left, &right) {
                    (Value::RegExp(regex), Value::String(text)) => regex.test(text),
                    (Value::String(text), Value::RegExp(regex)) => regex.test(text),
                    _ => {
                        return Err(runtime_error(&format!(
                            "Operator \"{op}\" requires one RegExp argument and one string argument. Arguments are {left} and {right}."
                        )))
                    }
                };
                Ok(Value::Boolean(if op == "=~" { matched } else { !matched }))
            }
            _ => {
                if is_binary_function(&op) {
                    evaluate_binary_function(&op, left, right)
                } else {
                    Err(runtime_error(&format!("Unexpected operator \"{op}\".")))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Expression (mirrors the `Expression` constructor and prototype)
// ---------------------------------------------------------------------------

/// Mirrors `shaderState` used by `getShaderExpression`: stores information
/// about the generated shader function, including whether it is translucent.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShaderState {
    pub translucent: bool,
}

/// Mirrors `numberToString`: whole numbers get a `.0` suffix.
fn number_to_string(number: f64) -> String {
    if number.fract() == 0.0 {
        format!("{number:.1}")
    } else {
        number_to_js_string(number)
    }
}

fn color_to_vec3(color: &Color) -> String {
    format!(
        "vec3({}, {}, {})",
        number_to_string(color.red),
        number_to_string(color.green),
        number_to_string(color.blue)
    )
}

fn color_to_vec4(color: &Color) -> String {
    format!(
        "vec4({}, {}, {}, {})",
        number_to_string(color.red),
        number_to_string(color.green),
        number_to_string(color.blue),
        number_to_string(color.alpha)
    )
}

/// Mirrors `convertHSLToColor`: returns None when any channel is not a
/// literal number.
fn convert_hsl_to_color(node: &Node) -> Option<Color> {
    let channels = node.left_children.as_ref()?;
    for channel in channels {
        if channel.node_type != ExpressionNodeType::LiteralNumber {
            return None;
        }
    }
    let channel = |index: usize| match &channels[index].value {
        NodeValue::Number(value) => *value,
        _ => 0.0,
    };
    let alpha = if channels.len() == 4 {
        channel(3)
    } else {
        1.0
    };
    Some(Color::from_hsl(channel(0), channel(1), channel(2), alpha))
}

/// Mirrors `convertRGBToColor`: returns None when any channel is not a
/// literal number.
fn convert_rgb_to_color(node: &Node) -> Option<Color> {
    let channels = node.left_children.as_ref()?;
    for channel in channels {
        if channel.node_type != ExpressionNodeType::LiteralNumber {
            return None;
        }
    }
    let channel = |index: usize| match &channels[index].value {
        NodeValue::Number(value) => *value,
        _ => 0.0,
    };
    let mut color = Color::default();
    color.red = channel(0) / 255.0;
    color.green = channel(1) / 255.0;
    color.blue = channel(2) / 255.0;
    color.alpha = if channels.len() == 4 {
        channel(3)
    } else {
        1.0
    };
    Some(color)
}

impl Node {
    /// Mirrors `getVariableName`.
    fn shader_variable_name<'a>(
        variable_name: &str,
        variable_substitution_map: &'a HashMap<String, String>,
    ) -> String {
        match variable_substitution_map.get(variable_name) {
            Some(name) => name.clone(),
            None => Expression::NULL_SENTINEL.to_string(),
        }
    }

    /// Mirrors `Node.prototype.getShaderExpression`.
    pub fn get_shader_expression(
        &self,
        variable_substitution_map: &HashMap<String, String>,
        shader_state: &mut ShaderState,
        parent: Option<&Node>,
    ) -> Result<Option<String>, RuntimeError> {
        let left_array: Option<Vec<Option<String>>> = match &self.left_children {
            Some(children) => {
                let mut expressions = Vec::with_capacity(children.len());
                for child in children {
                    expressions.push(child.get_shader_expression(
                        variable_substitution_map,
                        shader_state,
                        Some(self),
                    )?);
                }
                Some(expressions)
            }
            None => None,
        };
        let left: Option<Option<String>> = match &self.left {
            Some(left) => Some(left.get_shader_expression(
                variable_substitution_map,
                shader_state,
                Some(self),
            )?),
            None => None,
        };
        let right: Option<Option<String>> = match &self.right {
            Some(right) => Some(right.get_shader_expression(
                variable_substitution_map,
                shader_state,
                Some(self),
            )?),
            None => None,
        };
        let test: Option<Option<String>> = match &self.test {
            Some(test) => Some(test.get_shader_expression(
                variable_substitution_map,
                shader_state,
                Some(self),
            )?),
            None => None,
        };

        let render = |value: &Option<String>| match value {
            Some(value) => value.clone(),
            None => "undefined".to_string(),
        };
        let left_str = || render(left.as_ref().unwrap_or(&None));
        let right_str = || render(right.as_ref().unwrap_or(&None));
        let test_str = || render(test.as_ref().unwrap_or(&None));

        let value_string = match &self.value {
            NodeValue::Str(s) => s.clone(),
            NodeValue::Number(n) => number_to_string(*n),
            NodeValue::Bool(b) => b.to_string(),
            _ => String::new(),
        };

        Ok(match self.node_type {
            ExpressionNodeType::Variable => {
                if check_feature(self) {
                    None
                } else {
                    Some(Node::shader_variable_name(&value_string, variable_substitution_map))
                }
            }
            ExpressionNodeType::Unary => {
                if value_string == "Boolean" {
                    Some(format!("bool({})", left_str()))
                } else if value_string == "Number" {
                    Some(format!("float({})", left_str()))
                } else if value_string == "round" {
                    Some(format!("floor({} + 0.5)", left_str()))
                } else if is_unary_function(&value_string) {
                    Some(format!("{}({})", value_string, left_str()))
                } else if value_string == "isNaN" {
                    // In GLSL 2.0 use isnan instead
                    Some(format!("({0} != {0})", left_str()))
                } else if value_string == "isFinite" {
                    Some(format!("(abs({}) < czm_infinity)", left_str()))
                } else if matches!(
                    value_string.as_str(),
                    "String" | "isExactClass" | "isClass" | "getExactClassName"
                ) {
                    return Err(runtime_error(&format!(
                        "Error generating style shader: \"{value_string}\" is not supported."
                    )));
                } else {
                    Some(format!("{}{}", value_string, left_str()))
                }
            }
            ExpressionNodeType::Binary => {
                if value_string == "%" {
                    Some(format!("mod({}, {})", left_str(), right_str()))
                } else if value_string == "===" {
                    Some(format!("({} == {})", left_str(), right_str()))
                } else if value_string == "!==" {
                    Some(format!("({} != {})", left_str(), right_str()))
                } else if value_string == "atan2" {
                    Some(format!("atan({}, {})", left_str(), right_str()))
                } else if is_binary_function(&value_string) {
                    Some(format!("{}({}, {})", value_string, left_str(), right_str()))
                } else {
                    Some(format!("({} {} {})", left_str(), value_string, right_str()))
                }
            }
            ExpressionNodeType::Ternary => {
                if is_ternary_function(&value_string) {
                    Some(format!(
                        "{}({}, {}, {})",
                        value_string,
                        left_str(),
                        right_str(),
                        test_str()
                    ))
                } else {
                    None
                }
            }
            ExpressionNodeType::Conditional => Some(format!(
                "({} ? {} : {})",
                test_str(),
                left_str(),
                right_str()
            )),
            ExpressionNodeType::Member => {
                if let Some(left_node) = &self.left {
                    if check_feature(left_node) {
                        return Ok(Some(Node::shader_variable_name(
                            &right_str(),
                            variable_substitution_map,
                        )));
                    }
                }
                // This is intended for accessing the components of vector
                // properties. String members aren't supported.
                // Check for 0.0 rather than 0 because all numbers are
                // previously converted to decimals.
                let right_value = right_str();
                let left_value = left_str();
                match right_value.as_str() {
                    "r" | "x" | "0.0" => Some(format!("{left_value}[0]")),
                    "g" | "y" | "1.0" => Some(format!("{left_value}[1]")),
                    "b" | "z" | "2.0" => Some(format!("{left_value}[2]")),
                    "a" | "w" | "3.0" => Some(format!("{left_value}[3]")),
                    _ => Some(format!("{left_value}[int({right_value})]")),
                }
            }
            ExpressionNodeType::FunctionCall => {
                return Err(runtime_error(&format!(
                    "Error generating style shader: \"{value_string}\" is not supported."
                )));
            }
            ExpressionNodeType::Array => {
                let NodeValue::Nodes(nodes) = &self.value else {
                    return Ok(None);
                };
                let mut expressions = Vec::with_capacity(nodes.len());
                for node in nodes {
                    expressions.push(render(&node.get_shader_expression(
                        variable_substitution_map,
                        shader_state,
                        Some(self),
                    )?));
                }
                match expressions.len() {
                    4 => Some(format!(
                        "vec4({}, {}, {}, {})",
                        expressions[0], expressions[1], expressions[2], expressions[3]
                    )),
                    3 => Some(format!(
                        "vec3({}, {}, {})",
                        expressions[0], expressions[1], expressions[2]
                    )),
                    2 => Some(format!("vec2({}, {})", expressions[0], expressions[1])),
                    _ => {
                        return Err(runtime_error(
                            "Error generating style shader: Invalid array length. Array length should be 2, 3, or 4.",
                        ))
                    }
                }
            }
            ExpressionNodeType::Regex => {
                return Err(runtime_error(
                    "Error generating style shader: Regular expressions are not supported.",
                ));
            }
            ExpressionNodeType::VariableInString => {
                return Err(runtime_error(
                    "Error generating style shader: Converting a variable to a string is not supported.",
                ));
            }
            ExpressionNodeType::LiteralNull => Some(Expression::NULL_SENTINEL.to_string()),
            ExpressionNodeType::LiteralBoolean => match &self.value {
                NodeValue::Bool(true) => Some("true".to_string()),
                _ => Some("false".to_string()),
            },
            ExpressionNodeType::LiteralNumber => match &self.value {
                NodeValue::Number(n) => Some(number_to_string(*n)),
                _ => None,
            },
            ExpressionNodeType::LiteralString => {
                if let Some(parent) = parent {
                    if parent.node_type == ExpressionNodeType::Member {
                        let feature_left = parent
                            .left
                            .as_ref()
                            .map(|left| check_feature(left))
                            .unwrap_or(false);
                        if matches!(
                            value_string.as_str(),
                            "r" | "g" | "b" | "a" | "x" | "y" | "z" | "w"
                        ) || feature_left
                        {
                            return Ok(Some(value_string));
                        }
                    }
                }
                // Check for css color strings
                match Color::from_css_color_string(&value_string) {
                    Some(color) => Some(color_to_vec3(&color)),
                    None => {
                        return Err(runtime_error(
                            "Error generating style shader: String literals are not supported.",
                        ))
                    }
                }
            }
            ExpressionNodeType::LiteralColor => {
                let args = left_array;
                let arg = |args: &Vec<Option<String>>, index: usize| {
                    render(args.get(index).unwrap_or(&None))
                };
                if value_string == "color" {
                    match &args {
                        None => Some("vec4(1.0)".to_string()),
                        Some(args) if args.len() > 1 => {
                            let rgb = arg(args, 0);
                            let alpha = arg(args, 1);
                            if alpha != "1.0" {
                                shader_state.translucent = true;
                            }
                            Some(format!("vec4({rgb}, {alpha})"))
                        }
                        Some(args) => Some(format!("vec4({}, 1.0)", arg(args, 0))),
                    }
                } else if value_string == "rgb" {
                    match convert_rgb_to_color(self) {
                        Some(color) => Some(color_to_vec4(&color)),
                        None => {
                            let args = args.expect("rgb has arguments");
                            Some(format!(
                                "vec4({} / 255.0, {} / 255.0, {} / 255.0, 1.0)",
                                arg(&args, 0),
                                arg(&args, 1),
                                arg(&args, 2)
                            ))
                        }
                    }
                } else if value_string == "rgba" {
                    let args = args.expect("rgba has arguments");
                    if arg(&args, 3) != "1.0" {
                        shader_state.translucent = true;
                    }
                    match convert_rgb_to_color(self) {
                        Some(color) => Some(color_to_vec4(&color)),
                        None => Some(format!(
                            "vec4({} / 255.0, {} / 255.0, {} / 255.0, {})",
                            arg(&args, 0),
                            arg(&args, 1),
                            arg(&args, 2),
                            arg(&args, 3)
                        )),
                    }
                } else if value_string == "hsl" {
                    match convert_hsl_to_color(self) {
                        Some(color) => Some(color_to_vec4(&color)),
                        None => {
                            let args = args.expect("hsl has arguments");
                            Some(format!(
                                "vec4(czm_HSLToRGB(vec3({}, {}, {})), 1.0)",
                                arg(&args, 0),
                                arg(&args, 1),
                                arg(&args, 2)
                            ))
                        }
                    }
                } else if value_string == "hsla" {
                    match convert_hsl_to_color(self) {
                        Some(color) => {
                            if color.alpha != 1.0 {
                                shader_state.translucent = true;
                            }
                            Some(color_to_vec4(&color))
                        }
                        None => {
                            let args = args.expect("hsla has arguments");
                            if arg(&args, 3) != "1.0" {
                                shader_state.translucent = true;
                            }
                            Some(format!(
                                "vec4(czm_HSLToRGB(vec3({}, {}, {})), {})",
                                arg(&args, 0),
                                arg(&args, 1),
                                arg(&args, 2),
                                arg(&args, 3)
                            ))
                        }
                    }
                } else {
                    None
                }
            }
            ExpressionNodeType::LiteralVector => {
                let args = left_array.expect("left should always be defined for LITERAL_VECTOR");
                let mut vector_expression = format!("{value_string}(");
                for (i, arg) in args.iter().enumerate() {
                    vector_expression.push_str(&render(arg));
                    if i < args.len() - 1 {
                        vector_expression.push_str(", ");
                    }
                }
                vector_expression.push(')');
                Some(vector_expression)
            }
            ExpressionNodeType::LiteralRegex => {
                return Err(runtime_error(
                    "Error generating style shader: Regular expressions are not supported.",
                ));
            }
            ExpressionNodeType::LiteralUndefined => Some(Expression::NULL_SENTINEL.to_string()),
            ExpressionNodeType::BuiltinVariable => {
                if value_string == "tiles3d_tileset_time" {
                    Some(value_string)
                } else {
                    None
                }
            }
        })
    }

    /// Mirrors `Node.prototype.getVariables`.
    pub fn get_variables(&self, variables: &mut Vec<String>, parent: Option<&Node>) {
        if let Some(children) = &self.left_children {
            for child in children {
                child.get_variables(variables, Some(self));
            }
        }
        if let Some(left) = &self.left {
            left.get_variables(variables, Some(self));
        }
        if let Some(right) = &self.right {
            right.get_variables(variables, Some(self));
        }
        if let Some(test) = &self.test {
            test.get_variables(variables, Some(self));
        }
        if let NodeValue::Nodes(nodes) = &self.value {
            // For ARRAY type
            for node in nodes {
                node.get_variables(variables, Some(self));
            }
        }

        match self.node_type {
            ExpressionNodeType::Variable => {
                if !check_feature(self) {
                    if let NodeValue::Str(value) = &self.value {
                        variables.push(value.clone());
                    }
                }
            }
            ExpressionNodeType::VariableInString => {
                if let NodeValue::Str(value) = &self.value {
                    let pattern =
                        Regex::new(VARIABLE_PATTERN).expect("variable pattern is valid");
                    for captures in pattern.captures_iter(value) {
                        variables.push(captures[1].to_string());
                    }
                }
            }
            ExpressionNodeType::LiteralString => {
                if let Some(parent) = parent {
                    let feature_left = parent
                        .left
                        .as_ref()
                        .map(|left| check_feature(left))
                        .unwrap_or(false);
                    if parent.node_type == ExpressionNodeType::Member && feature_left {
                        if let NodeValue::Str(value) = &self.value {
                            variables.push(value.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// An expression for a style applied to a `Cesium3DTileset`. Evaluates an
/// expression defined using the 3D Tiles Styling language. Implements the
/// `StyleExpression` interface.
pub struct Expression {
    expression_string: String,
    runtime_ast: Node,
}

impl Expression {
    /// null just needs to be some sentinel value that will cause
    /// "[expression] === null" to be false in nearly all cases. GLSL doesn't
    /// have a NaN constant so use czm_infinity.
    pub const NULL_SENTINEL: &'static str = "czm_infinity";

    /// Mirrors `new Expression(expression, defines)`; parse failures are
    /// returned as `Err` instead of being thrown.
    pub fn try_new(
        expression: &str,
        defines: Option<&HashMap<String, String>>,
    ) -> Result<Expression, RuntimeError> {
        let expression_string = expression.to_string();
        let mut processed = expression.to_string();
        if let Some(defines) = defines {
            processed = replace_defines(&processed, defines);
        }
        processed = replace_variables(&remove_backslashes(&processed))?;

        // jsep customization mirrored by the Pratt parser: addBinaryOp("=~", 0)
        // and addBinaryOp("!~", 0).
        let ast = Parser::parse(&processed)?;
        let runtime_ast = create_runtime_ast(&ast)?;
        Ok(Expression {
            expression_string,
            runtime_ast,
        })
    }

    /// Mirrors `new Expression(expression, defines)`; panics on parse
    /// errors, like the JS constructor throws.
    pub fn new(expression: &str, defines: Option<&HashMap<String, String>>) -> Expression {
        match Self::try_new(expression, defines) {
            Ok(expression) => expression,
            Err(error) => panic!("{error}"),
        }
    }

    /// Gets the expression defined in the 3D Tiles Styling language.
    pub fn expression(&self) -> &str {
        &self.expression_string
    }

    /// Exposes the runtime AST, mirroring the spec's access to the private
    /// `_runtimeAst` field (used to assert node types such as LITERAL_REGEX).
    pub fn runtime_ast(&self) -> &Node {
        &self.runtime_ast
    }

    /// Mirrors `Expression.prototype.evaluate`. Results are returned as
    /// owned values (DEVIATION: the JS `result` parameter is handled by
    /// `evaluate_color` for colors; Cartesian results are simply cloned).
    pub fn evaluate(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        self.runtime_ast.evaluate(feature)
    }

    /// Mirrors `Expression.prototype.evaluateColor`.
    pub fn evaluate_color<'a>(
        &self,
        feature: Option<&dyn ExpressionFeature>,
        result: &'a mut Color,
    ) -> Result<&'a mut Color, RuntimeError> {
        let value = self.runtime_ast.evaluate(feature)?;
        match value {
            Value::Cartesian4(cartesian) => {
                result.red = cartesian.x;
                result.green = cartesian.y;
                result.blue = cartesian.z;
                result.alpha = cartesian.w;
                Ok(result)
            }
            other => Err(runtime_error(&format!(
                "Expression does not evaluate to a color. Result is {other}."
            ))),
        }
    }

    /// Mirrors `Expression.prototype.getShaderFunction`.
    pub fn get_shader_function(
        &self,
        function_signature: &str,
        variable_substitution_map: &HashMap<String, String>,
        shader_state: &mut ShaderState,
        return_type: &str,
    ) -> Result<String, RuntimeError> {
        let shader_expression = self
            .get_shader_expression(variable_substitution_map, shader_state)?
            .unwrap_or_else(|| "undefined".to_string());
        Ok(format!(
            "{return_type} {function_signature}\n{{\n    return {shader_expression};\n}}\n"
        ))
    }

    /// Mirrors `Expression.prototype.getShaderExpression`.
    pub fn get_shader_expression(
        &self,
        variable_substitution_map: &HashMap<String, String>,
        shader_state: &mut ShaderState,
    ) -> Result<Option<String>, RuntimeError> {
        self.runtime_ast
            .get_shader_expression(variable_substitution_map, shader_state, None)
    }

    /// Mirrors `Expression.prototype.getVariables`.
    pub fn get_variables(&self) -> Vec<String> {
        let mut variables = Vec::new();
        self.runtime_ast.get_variables(&mut variables, None);

        // Remove duplicates
        let mut deduped = Vec::with_capacity(variables.len());
        for variable in variables.drain(..) {
            if !deduped.contains(&variable) {
                deduped.push(variable);
            }
        }
        deduped
    }
}
