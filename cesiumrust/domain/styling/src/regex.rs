//! Regular-expression support for the 3D Tiles Styling expression engine.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs`:
//! - `RegExpValue` (+ `compile`/`test`/`exec_first_capture`/`to_js_string`) ← L47-104
//! - `node_value_string`                                                     ← L1131-1140
//! - `parse_regex`                                                           ← L1143-1201
//!
//! which is itself the Rust port of upstream
//! `packages/engine/Source/Scene/Expression.js` (`regExp()` + `addBinaryOp("=~"/"!~", 0)`).
//!
//! # M7-B scope notes
//!
//! * The `regex` crate IS available in the offline registry (locked at 1.13.x in
//!   the workspace), so [`RegExpValue`] holds a **real compiled** [`::regex::Regex`]
//!   here rather than the M7-A source/flags placeholder. The M7-A placeholder lived
//!   in `value.rs`; it is relocated here (its natural home per the blueprint, which
//!   keeps `RegExpValue` beside the regex helpers) and upgraded to compile.
//! * The external crate is referenced as `::regex` (leading `::`) to disambiguate
//!   it from this `crate::regex` module.
//! * DEVIATION (Rust `regex` vs JS `RegExp`): the Rust engine has **no**
//!   look-behind/look-ahead and no `u`/`y` flags. `compile` accepts `g`/`u`/`y`
//!   and ignores them (the `regex` crate is stateless, so `g` is meaningless);
//!   `i`/`m`/`s` map to inline `(?i)`/`(?m)`/`(?s)` prefixes. Upstream spec cases
//!   relying on look-around or `u`/`y` semantics are expected to be `#[ignore]`d
//!   in M7-C.

use ::regex::Regex;

use crate::ast::{create_runtime_ast, replace_backslashes, ExpressionNodeType, JsepNode, Node, NodeValue};
use crate::value::{number_to_js_string, runtime_error, RuntimeError};

// ---------------------------------------------------------------------------
// RegExpValue (mirrors a JS `RegExp` produced by `regExp()`)
// ---------------------------------------------------------------------------

/// A compiled regular expression value, mirroring a JS `RegExp` produced by the
/// `regExp()` function.
#[derive(Debug, Clone)]
pub struct RegExpValue {
    compiled: Regex,
    /// The original (backslash-restored) pattern source.
    pub source: String,
    /// The JS flag string (e.g. `"gi"`).
    pub flags: String,
}

impl RegExpValue {
    /// Compiles a pattern with JS-style flags (`i`, `m`, `s`; `g`/`u`/`y` are
    /// accepted and ignored since the `regex` crate has no global state).
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

    /// Mirrors `RegExp.prototype.exec`, returning capture group 1 when present
    /// (the full match otherwise), as used by `_evaluateRegExpExec`.
    pub fn exec_first_capture(&self, text: &str) -> Option<String> {
        let captures = self.compiled.captures(text)?;
        let group = captures.get(1).or_else(|| captures.get(0))?;
        Some(group.as_str().to_string())
    }

    /// Mirrors `String(regExp)` -> `"/pattern/flags"`. JS sorts the flags in
    /// `dgimsuy` order when stringifying a RegExp.
    pub fn to_js_string(&self) -> String {
        let mut sorted: Vec<char> = self.flags.chars().collect();
        sorted.sort_by_key(|c| "dgimsuy".find(*c).unwrap_or(usize::MAX));
        let flags: String = sorted.into_iter().collect();
        format!("/{}/{}", self.source, flags)
    }
}

// ---------------------------------------------------------------------------
// parse_regex (mirrors `parseRegex`)
// ---------------------------------------------------------------------------

/// Mirrors `getDefaultValueString` for a literal node: the string a literal
/// contributes to a `regExp(...)` pattern/flags argument.
pub(crate) fn node_value_string(node: &Node) -> String {
    match &node.value {
        NodeValue::Null => "null".to_string(),
        NodeValue::Undefined => "undefined".to_string(),
        NodeValue::Bool(value) => value.to_string(),
        NodeValue::Number(value) => number_to_js_string(*value),
        NodeValue::Str(value) => replace_backslashes(value),
        NodeValue::None | NodeValue::Nodes(_) | NodeValue::Regex(_) => String::new(),
    }
}

/// Mirrors `parseRegex`: builds a LITERAL_REGEX node when the pattern (and
/// optional flags) are literal, otherwise a REGEX node compiled at evaluate time.
pub(crate) fn parse_regex(arguments: &[JsepNode]) -> Result<Node, RuntimeError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_and_test_basic() {
        let re = RegExpValue::compile("ab", "").unwrap();
        assert!(re.test("xxabyy"));
        assert!(!re.test("xxayy"));
        assert_eq!(re.source, "ab");
        assert_eq!(re.flags, "");
    }

    #[test]
    fn compile_flags_case_insensitive() {
        let re = RegExpValue::compile("ab", "i").unwrap();
        assert!(re.test("AB"));
        // Without the flag it would not match uppercase.
        let plain = RegExpValue::compile("ab", "").unwrap();
        assert!(!plain.test("AB"));
    }

    #[test]
    fn compile_global_flag_is_accepted_and_ignored() {
        // `g` has no meaning for the stateless Rust engine but must not error.
        let re = RegExpValue::compile("a", "g").unwrap();
        assert!(re.test("aaa"));
        assert_eq!(re.flags, "g");
    }

    #[test]
    fn compile_invalid_flag_errors() {
        let err = RegExpValue::compile("a", "z").unwrap_err();
        assert!(err.message().contains("Invalid flags"));
    }

    #[test]
    fn compile_invalid_pattern_errors() {
        // Unbalanced group is a compile error surfaced as a RuntimeError.
        assert!(RegExpValue::compile("(", "").is_err());
    }

    #[test]
    fn exec_first_capture_prefers_group_one() {
        let re = RegExpValue::compile("a(b)c", "").unwrap();
        assert_eq!(re.exec_first_capture("xxabcyy").as_deref(), Some("b"));
        // No capture group -> whole match.
        let re2 = RegExpValue::compile("abc", "").unwrap();
        assert_eq!(re2.exec_first_capture("xxabcyy").as_deref(), Some("abc"));
        // No match -> None.
        assert_eq!(re2.exec_first_capture("zzz"), None);
    }

    #[test]
    fn to_js_string_sorts_flags() {
        assert_eq!(RegExpValue::compile("ab", "gi").unwrap().to_js_string(), "/ab/gi");
        assert_eq!(RegExpValue::compile("ab", "ig").unwrap().to_js_string(), "/ab/gi");
        assert_eq!(RegExpValue::compile("x", "").unwrap().to_js_string(), "/x/");
    }

    #[test]
    fn node_value_string_by_type() {
        let n = Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(5.0),
            None,
            None,
            None,
        );
        assert_eq!(node_value_string(&n), "5");
        let s = Node::new(
            ExpressionNodeType::LiteralString,
            NodeValue::Str("ab".to_string()),
            None,
            None,
            None,
        );
        assert_eq!(node_value_string(&s), "ab");
    }

    #[test]
    fn parse_regex_no_args_is_default() {
        let node = parse_regex(&[]).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralRegex);
        assert!(matches!(node.value, NodeValue::Regex(_)));
    }

    #[test]
    fn parse_regex_literal_pattern_compiles_eagerly() {
        let args = vec![JsepNode::Literal(crate::ast::JsepLiteral::Str("a.c".to_string()))];
        let node = parse_regex(&args).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralRegex);
    }

    #[test]
    fn parse_regex_non_literal_pattern_defers_to_runtime() {
        // A variable pattern cannot be compiled at parse time -> REGEX node.
        let args = vec![JsepNode::Identifier("czm_pattern".to_string())];
        let node = parse_regex(&args).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::Regex);
        assert!(node.left.is_some());
        assert!(node.right.is_none());
    }
}
