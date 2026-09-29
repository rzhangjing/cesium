//! 表达式预处理：3D Tiles Styling 语言的 `${...}` defines、反斜杠转义与变量替换。
//!
//! 移植自 `cesium-rs/crates/cesium-scene/src/expression.rs` L372-440
//! （`VARIABLE_PATTERN` / `replace_defines` / `remove_backslashes` /
//! `replace_variables`），它是上游 `packages/engine/Source/Scene/Expression.js`
//! L573-625（`replaceDefines` / `removeBackslashes` / `replaceBackslashes` /
//! `replaceVariables`）的 Rust 移植。
//!
//! # 冲突消解（任务简报 vs 事实来源）
//!
//! 任务简报把 `replace_defines` 描述为需要**递归展开 + 环检测（`MAX_DEFINES_DEPTH`）**。
//! 无论 blueprint（`expression.rs` L377-390）还是上游（`Expression.js` L573-587）
//! 都不这么做：两者都对 `defines` 做**单趟**遍历，每个 `${key}` 只替换一次。
//! 单趟不可能无限循环，所以 `MAX_DEFINES_DEPTH` 无关紧要。本移植遵循**事实来源**
//! （单趟，忠于 blueprint），而非简报里臆造的递归保护；该偏离记录在此处与里程碑报告里。
//!
//! `replace_backslashes`（`remove_backslashes` 的逆运算）位于 `ast.rs`，
//! 因为 `parse_literal` 需要它；这里只有正向以及 define/variable 遍历。

use std::collections::HashMap;

use ::regex::Regex;

use crate::value::{runtime_error, RuntimeError};

/// `${name}` 占位符模式，镜像 Expression.js 的 `VARIABLE_PATTERN`。
pub const VARIABLE_PATTERN: &str = r"\$\{(.*?)}";

/// 编译好的 `${name}` 模式，为求值热路径而缓存。
///
/// 在每次 `VariableInString` 求值（以及每次 `get_variables` 遍历）时
/// 编译一个 [`Regex`] 代价高得令人却步 —— 字符串模板 styling 表达式是现实世界
/// 最常见的情形，所以这里通过 [`std::sync::OnceLock`] 每进程恰好编译该模式一次。
pub fn variable_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| Regex::new(VARIABLE_PATTERN).expect("variable pattern is valid"))
}

/// 词法期间 `\` 被交换成的哨兵，镜像 `BACKSLASH_REPLACEMENT`。
pub(crate) const BACKSLASH_REPLACEMENT: &str = "@#%";

/// 镜像 `replaceDefines`：把每个 `${key}` 占位符替换为
/// `(<define value>)`。对 `defines` 单趟遍历（见模块文档）。
pub fn replace_defines(expression: &str, defines: &HashMap<String, String>) -> String {
    let mut result = expression.to_string();
    for (key, value) in defines {
        let placeholder = Regex::new(&format!(r"\$\{{{}}}", ::regex::escape(key)))
            .expect("escaped define key is a valid regex");
        let define_replace = format!("({value})");
        // NoExpand：替换值本身可能含有 `${...}`，
        // 否则它们会被解释为捕获组引用。
        result = placeholder
            .replace_all(&result, ::regex::NoExpand(define_replace.as_str()))
            .to_string();
    }
    result
}

/// 镜像 `removeBackslashes`：`\` -> `"@#%"`。
pub fn remove_backslashes(expression: &str) -> String {
    expression.replace('\\', BACKSLASH_REPLACEMENT)
}

/// 镜像 `replaceVariables`：引号之外的 `${name}` 变成 `czm_name`；
/// 一个未终止的 `${` 抛出 `"Unmatched {."`。
pub fn replace_variables(expression: &str) -> Result<String, RuntimeError> {
    let mut exp = expression.to_string();
    let mut result = String::new();
    while let Some(i) = exp.find("${") {
        // 检查字符串是否位于引号内
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_and_replace_backslashes_roundtrip() {
        // remove：`\` -> "@#%"（ast::replace_backslashes 是其逆运算）。
        assert_eq!(remove_backslashes(r"a\b"), "a@#%b");
        assert_eq!(crate::ast::replace_backslashes("a@#%b"), r"a\b");
    }

    #[test]
    fn replace_variables_bare_and_quoted() {
        // 引号之外 -> czm_name。
        assert_eq!(replace_variables("${height}").unwrap(), "czm_height");
        assert_eq!(
            replace_variables("${a} + ${b}").unwrap(),
            "czm_a + czm_b"
        );
        // 在带引号的字符串内，占位符逐字保留，以便
        // VariableInString 节点在求值时对其插值。
        assert_eq!(
            replace_variables("'${name}'").unwrap(),
            "'${name}'"
        );
        assert_eq!(
            replace_variables("\"${name}\"").unwrap(),
            "\"${name}\""
        );
    }

    #[test]
    fn replace_variables_unmatched_errors() {
        let err = replace_variables("${height").unwrap_err();
        assert!(err.message().contains("Unmatched {."));
    }

    #[test]
    fn replace_defines_single_pass() {
        let mut defines = HashMap::new();
        defines.insert("x".to_string(), "1 + 2".to_string());
        // `${x}` -> "(1 + 2)"。
        assert_eq!(replace_defines("${x} * 3", &defines), "(1 + 2) * 3");
        // NoExpand：含 `${...}` 的 define 值被逐字插入，
        // 不被当作捕获引用，且不会被重新展开（单趟）
        // —— 这就是 blueprint/上游的行为。
        let mut nested = HashMap::new();
        nested.insert("a".to_string(), "${b}".to_string());
        assert_eq!(replace_defines("${a}", &nested), "(${b})");
    }

    #[test]
    fn variable_pattern_matches_placeholder() {
        let re = Regex::new(VARIABLE_PATTERN).unwrap();
        let caps = re.captures("pre${name}post").unwrap();
        assert_eq!(&caps[1], "name");
    }
}
