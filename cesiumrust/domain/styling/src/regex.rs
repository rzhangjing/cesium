//! 3D Tiles Styling 表达式引擎的正则表达式支持。
//!
//! 本模块提供正则支持：
//! - `RegExpValue` (+ `compile`/`test`/`exec_first_capture`/`to_js_string`)：编译好的正则值
//! - `node_value_string`：从 AST 节点还原正则字面量文本
//! - `parse_regex`：把 `regExp()` 调用解析为 [`RegExpValue`]
//!
//! styling 语言通过 `regExp()` 函数与自定义二元运算符 `=~`/`!~` 使用正则，
//! 本模块把二者接到工作区锁定的 `regex` crate 上。
//!
//! # M7-B 作用域说明
//!
//! * `regex` crate 在离线 registry 中**确实可用**（工作区里锁定为 1.13.x），
//!   所以这里 [`RegExpValue`] 持有的是一个**真正编译好的** [`::regex::Regex`]，
//!   而非 M7-A 的 source/flags 占位。M7-A 的占位原本位于 `value.rs`；它被
//!   迁移到这里（按 blueprint 它是自然归属，将 `RegExpValue` 与正则辅助函数
//!   放在一起），并升级为真正编译。
//! * 外部 crate 以 `::regex`（前导 `::`）引用，以与本页的 `crate::regex`
//!   模块区分。
//! * 偏离（Rust `regex` vs JS `RegExp`）：Rust 引擎**没有**后行/先行断言
//!   （look-behind/look-ahead），也没有 `u`/`y` flag。`compile` 接受 `g`/`u`/`y`
//!   并忽略它们（`regex` crate 无状态，故 `g` 无意义）；`i`/`m`/`s` 映射为
//!   内联 `(?i)`/`(?m)`/`(?s)` 前缀。上游依赖 look-around 或 `u`/`y` 语义的
//!   spec 用例预计在 M7-C 中会被 `#[ignore]`。

use ::regex::Regex;

use crate::ast::{create_runtime_ast, replace_backslashes, ExpressionNodeType, JsepNode, Node, NodeValue};
use crate::value::{number_to_js_string, runtime_error, RuntimeError};

// ---------------------------------------------------------------------------
// RegExpValue（镜像 `regExp()` 产生的 JS `RegExp`）
// ---------------------------------------------------------------------------

/// 一个编译好的正则表达式值，镜像由 `regExp()` 函数产生的 JS `RegExp`。
#[derive(Debug, Clone)]
pub struct RegExpValue {
    /// 真正编译后的正则引擎句柄（`regex` crate 的 `Regex`）。
    compiled: Regex,
    /// 原始（反斜杠已还原的）模式 source。
    pub source: String,
    /// JS flag 字符串（例如 `"gi"`）。
    pub flags: String,
}

impl RegExpValue {
    /// 用 JS 风格 flags 编译模式（`i`、`m`、`s`；`g`/`u`/`y` 被接受并忽略，
    /// 因为 `regex` crate 没有全局状态）。
    /// 镜像包裹在 try/catch 中的 `new RegExp(pattern, flags)`。
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

    /// 镜像 `RegExp.prototype.test`。
    pub fn test(&self, text: &str) -> bool {
        self.compiled.is_match(text)
    }

    /// 镜像 `RegExp.prototype.exec`，存在时返回捕获组 1（否则返回整个匹配），
    /// 供 `_evaluateRegExpExec` 使用。
    pub fn exec_first_capture(&self, text: &str) -> Option<String> {
        let captures = self.compiled.captures(text)?;
        let group = captures.get(1).or_else(|| captures.get(0))?;
        Some(group.as_str().to_string())
    }

    /// 镜像 `String(regExp)` -> `"/pattern/flags"`。JS 在把 RegExp 转成字符串时
    /// 按 `dgimsuy` 顺序对 flags 排序。
    pub fn to_js_string(&self) -> String {
        let mut sorted: Vec<char> = self.flags.chars().collect();
        sorted.sort_by_key(|c| "dgimsuy".find(*c).unwrap_or(usize::MAX));
        let flags: String = sorted.into_iter().collect();
        format!("/{}/{}", self.source, flags)
    }
}

// ---------------------------------------------------------------------------
// parse_regex（镜像 `parseRegex`）
// ---------------------------------------------------------------------------

/// 对一个字面量节点镜像 `getDefaultValueString`：字面量在
/// `regExp(...)` 的 pattern/flags 参数中贡献的字符串。
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

/// 镜像 `parseRegex`：当 pattern（及可选 flags）是字面量时构建 LITERAL_REGEX
/// 节点，否则构建在求值时才编译的 REGEX 节点。
pub(crate) fn parse_regex(arguments: &[JsepNode]) -> Result<Node, RuntimeError> {
    // 无参数，返回默认正则
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

    // 提供了可选的 flag 参数
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

    // 仅提供了 pattern 参数
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

    /// 验证基本编译+test 命中/未命中，并保留 source/flags。
    #[test]
    fn compile_and_test_basic() {
        let re = RegExpValue::compile("ab", "").unwrap();
        assert!(re.test("xxabyy"));
        assert!(!re.test("xxayy"));
        assert_eq!(re.source, "ab");
        assert_eq!(re.flags, "");
    }

    /// 验证 `i` flag 映射为内联 `(?i)` 从而实现大小写不敏感。
    #[test]
    fn compile_flags_case_insensitive() {
        let re = RegExpValue::compile("ab", "i").unwrap();
        assert!(re.test("AB"));
        // 没有该 flag 时不会匹配大写。
        let plain = RegExpValue::compile("ab", "").unwrap();
        assert!(!plain.test("AB"));
    }

    #[test]
    fn compile_global_flag_is_accepted_and_ignored() {
        // `g` 对无状态的 Rust 引擎无意义，但不得报错。
        let re = RegExpValue::compile("a", "g").unwrap();
        assert!(re.test("aaa"));
        assert_eq!(re.flags, "g");
    }

    /// 验证非法 flag 以 "Invalid flags" 报错。
    #[test]
    fn compile_invalid_flag_errors() {
        let err = RegExpValue::compile("a", "z").unwrap_err();
        assert!(err.message().contains("Invalid flags"));
    }

    #[test]
    fn compile_invalid_pattern_errors() {
        // 括号不配对是编译错误，以 RuntimeError 形式抛出。
        assert!(RegExpValue::compile("(", "").is_err());
    }

    /// 验证 `exec_first_capture` 优先取捕获组 1，否则整匹配，无匹配回 None。
    #[test]
    fn exec_first_capture_prefers_group_one() {
        let re = RegExpValue::compile("a(b)c", "").unwrap();
        assert_eq!(re.exec_first_capture("xxabcyy").as_deref(), Some("b"));
        // 无捕获组 -> 整个匹配。
        let re2 = RegExpValue::compile("abc", "").unwrap();
        assert_eq!(re2.exec_first_capture("xxabcyy").as_deref(), Some("abc"));
        // 无匹配 -> None。
        assert_eq!(re2.exec_first_capture("zzz"), None);
    }

    /// 验证 `to_js_string` 按 dgimsuy 顺序对 flags 排序后输出 `/pattern/flags`。
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
        // 变量 pattern 无法在解析期编译 -> REGEX 节点。
        let args = vec![JsepNode::Identifier("czm_pattern".to_string())];
        let node = parse_regex(&args).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::Regex);
        assert!(node.left.is_some());
        assert!(node.right.is_none());
    }
}
