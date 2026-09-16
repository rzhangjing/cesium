//! Expression preprocessing: `${...}` defines, backslash escaping and variable
//! substitution for the 3D Tiles Styling language.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs` L372-440
//! (`VARIABLE_PATTERN` / `replace_defines` / `remove_backslashes` /
//! `replace_variables`), the Rust port of upstream
//! `packages/engine/Source/Scene/Expression.js` L573-625
//! (`replaceDefines` / `removeBackslashes` / `replaceBackslashes` /
//! `replaceVariables`).
//!
//! # CONFLICT resolution (task brief vs source of truth)
//!
//! The task brief described `replace_defines` as needing **recursive expansion +
//! cycle detection (`MAX_DEFINES_DEPTH`)**. Neither the blueprint
//! (`expression.rs` L377-390) nor upstream (`Expression.js` L573-587) does this:
//! both perform a **single pass** over `defines`, replacing each `${key}` once.
//! A single pass cannot loop forever, so `MAX_DEFINES_DEPTH` is moot. This port
//! follows the **source of truth** (single pass, blueprint-faithful) rather than
//! the brief's invented recursion guard; the deviation is recorded here and in
//! the milestone report.
//!
//! `replace_backslashes` (the reverse of `remove_backslashes`) lives in `ast.rs`
//! because `parse_literal` needs it; only the forward direction and the
//! define/variable passes are here.

use std::collections::HashMap;

use ::regex::Regex;

use crate::value::{runtime_error, RuntimeError};

/// The `${name}` placeholder pattern, mirroring Expression.js `VARIABLE_PATTERN`.
pub const VARIABLE_PATTERN: &str = r"\$\{(.*?)}";

/// The compiled `${name}` pattern, cached for the evaluation hot path.
///
/// Compiling a [`Regex`] on every `VariableInString` evaluation (and every
/// `get_variables` walk) is prohibitively expensive — string-template styling
/// expressions are the most common real-world case, so this compiles the
/// pattern exactly once per process via [`std::sync::OnceLock`].
pub fn variable_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| Regex::new(VARIABLE_PATTERN).expect("variable pattern is valid"))
}

/// The sentinel `\` is swapped to while lexing, mirroring `BACKSLASH_REPLACEMENT`.
pub(crate) const BACKSLASH_REPLACEMENT: &str = "@#%";

/// Mirrors `replaceDefines`: replaces each `${key}` placeholder with
/// `(<define value>)`. Single pass over `defines` (see module docs).
pub fn replace_defines(expression: &str, defines: &HashMap<String, String>) -> String {
    let mut result = expression.to_string();
    for (key, value) in defines {
        let placeholder = Regex::new(&format!(r"\$\{{{}}}", ::regex::escape(key)))
            .expect("escaped define key is a valid regex");
        let define_replace = format!("({value})");
        // NoExpand: the replacement value may itself contain `${...}` which
        // would otherwise be interpreted as capture-group references.
        result = placeholder
            .replace_all(&result, ::regex::NoExpand(define_replace.as_str()))
            .to_string();
    }
    result
}

/// Mirrors `removeBackslashes`: `\` -> `"@#%"`.
pub fn remove_backslashes(expression: &str) -> String {
    expression.replace('\\', BACKSLASH_REPLACEMENT)
}

/// Mirrors `replaceVariables`: `${name}` outside of quotes becomes `czm_name`;
/// an unterminated `${` throws `"Unmatched {."`.
pub fn replace_variables(expression: &str) -> Result<String, RuntimeError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_and_replace_backslashes_roundtrip() {
        // remove: `\` -> "@#%" (ast::replace_backslashes is the inverse).
        assert_eq!(remove_backslashes(r"a\b"), "a@#%b");
        assert_eq!(crate::ast::replace_backslashes("a@#%b"), r"a\b");
    }

    #[test]
    fn replace_variables_bare_and_quoted() {
        // Outside quotes -> czm_name.
        assert_eq!(replace_variables("${height}").unwrap(), "czm_height");
        assert_eq!(
            replace_variables("${a} + ${b}").unwrap(),
            "czm_a + czm_b"
        );
        // Inside a quoted string the placeholder is preserved verbatim so the
        // VariableInString node can interpolate it at evaluate time.
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
        // `${x}` -> "(1 + 2)".
        assert_eq!(replace_defines("${x} * 3", &defines), "(1 + 2) * 3");
        // NoExpand: a define value containing `${...}` is inserted literally,
        // not treated as a capture reference, and is NOT re-expanded (single
        // pass) — this is the blueprint/upstream behaviour.
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
