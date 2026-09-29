//! 3D Tiles Styling 表达式语言的词法分析器。
//!
//! 移植自 `cesium-rs/crates/cesium-scene/src/expression.rs` L447-633
//! （`Token` + `Tokenizer` + 字符谓词 + 字符串/数字/标识符/运算符
//! 词法），它是 styling 语言所需的 jsep 1.3.8 语义子集的 Rust 复现
//! （上游 `packages/engine/Source/Scene/Expression.js`
//! 用 jsep 加 `addBinaryOp("=~"/"!~", 0)` 解析）。
//!
//! # M7-A 作用域说明
//!
//! * 偏离（jsep）：**没有正则字面量词法**。[`Token::RegEx`] 是一个
//!   为 M7-B（`regex.rs`）预留的占位变体；M7-A 词法器从不产出它。Cesium styling
//!   通过 `regExp()` 函数构造正则而非 `/.../` 字面量，所以 blueprint 词法器
//!   也没有正则字面量路径 —— 该变体存在是为了让 M7-B 能在不破坏 `Token`
//!   的前提下扩展词法。
//! * 任务简报里提到的 `is_whitespace` 谓词就是内建的
//!   `char::is_whitespace`，在 [`Tokenizer::skip_whitespace`] 中直接使用
//!   （忠于 blueprint；不引入冗余包装）。
//! * 小的惯用法调整：双字符运算符分派用 `matches!`
//!   而非 blueprint 的 `match { .. , _ => {} }`，以保持 clippy 干净
//!   （`clippy::single_match`）；语义完全一致。

use crate::value::{runtime_error, RuntimeError};

/// 一个词法 token，镜像 jsep 的 token 流。
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Str(String),
    Ident(String),
    Op(String),
    /// 一个 `/pattern/flags` 正则字面量的 M7-B 占位 —— M7-A 词法器
    /// 从不产出（见模块文档）。
    #[allow(dead_code)] // M7-B 占位；在 M7-A 基础层中不构造
    RegEx { pattern: String, flags: String },
}

/// 一个手写的词法分析器，镜像 jsep 1.3.8 的词法器。
pub struct Tokenizer<'a> {
    chars: Vec<char>,
    index: usize,
    source: &'a str,
}

/// 对 `[A-Za-z_$]` 为 `true` —— jsep 允许标识符起始的字符集。
pub fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

/// 对 `[A-Za-z0-9_$]` 为 `true` —— jsep 允许出现在标识符内的字符集。
pub fn is_identifier_char(c: char) -> bool {
    is_identifier_start(c) || c.is_ascii_digit()
}

/// 对 `[0-9]` 为 `true`。
pub fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

impl<'a> Tokenizer<'a> {
    /// 在 `source` 上创建一个词法分析器。
    pub fn new(source: &'a str) -> Tokenizer<'a> {
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

    /// 把整个输入词法分析为一个 token 向量。
    pub fn tokenize(&mut self) -> Result<Vec<Token>, RuntimeError> {
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

    /// 字符串不做转义处理：反斜杠已由 `removeBackslashes` 被替换为
    /// `"@#%"`，模仿 jsep 在预处理步骤之后的行为。
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
        Err(runtime_error(&format!("Unclosed quote after \"{value}\"")))
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
        // jsep 以 parseFloat 语义解析；词法器在此保证一个
        // 良构的数字字面量。
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
        if matches!(
            two.as_str(),
            "&&" | "||" | "=~" | "!~" | ">=" | "<=" | "<<" | ">>"
        ) {
            self.index += 2;
            return Ok(two);
        }
        let c = self.advance().unwrap();
        // jsep 接受这些运算符；不支持的稍后由
        // create_runtime_ast 以 `Unexpected operator "{op}".` 拒绝。
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
        let _ = self.source; // 为与 jsep 错误上下文对齐而保留
        Err(runtime_error(&format!("Unexpected \"{c}\"")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<Token> {
        Tokenizer::new(input).tokenize().expect("tokenize failed")
    }

    #[test]
    fn numbers_and_operators() {
        assert_eq!(
            lex("1 + 2"),
            vec![Token::Number(1.0), Token::Op("+".into()), Token::Number(2.0)]
        );
    }

    #[test]
    fn identifiers() {
        assert_eq!(lex("foo"), vec![Token::Ident("foo".into())]);
        assert_eq!(lex("_bar$9"), vec![Token::Ident("_bar$9".into())]);
    }

    #[test]
    fn strings_single_and_double_quotes() {
        assert_eq!(lex("'hello'"), vec![Token::Str("hello".into())]);
        assert_eq!(lex("\"world\""), vec![Token::Str("world".into())]);
    }

    #[test]
    fn number_forms() {
        assert_eq!(lex("1.5"), vec![Token::Number(1.5)]);
        assert_eq!(lex(".5"), vec![Token::Number(0.5)]);
        assert_eq!(lex("1e3"), vec![Token::Number(1000.0)]);
        assert_eq!(lex("1E-2"), vec![Token::Number(0.01)]);
        assert_eq!(lex("12."), vec![Token::Number(12.0)]);
    }

    #[test]
    fn multi_char_operators() {
        assert_eq!(lex("==="), vec![Token::Op("===".into())]);
        assert_eq!(lex("!=="), vec![Token::Op("!==".into())]);
        assert_eq!(lex("=~"), vec![Token::Op("=~".into())]);
        assert_eq!(lex("!~"), vec![Token::Op("!~".into())]);
        assert_eq!(lex("&&"), vec![Token::Op("&&".into())]);
        assert_eq!(lex(">="), vec![Token::Op(">=".into())]);
    }

    #[test]
    fn member_and_index_access() {
        assert_eq!(
            lex("a.b"),
            vec![Token::Ident("a".into()), Token::Op(".".into()), Token::Ident("b".into())]
        );
        assert_eq!(
            lex("a[0]"),
            vec![
                Token::Ident("a".into()),
                Token::Op("[".into()),
                Token::Number(0.0),
                Token::Op("]".into())
            ]
        );
    }

    #[test]
    fn conditional_operators() {
        assert_eq!(
            lex("x ? y : z"),
            vec![
                Token::Ident("x".into()),
                Token::Op("?".into()),
                Token::Ident("y".into()),
                Token::Op(":".into()),
                Token::Ident("z".into())
            ]
        );
    }

    #[test]
    fn whitespace_is_skipped() {
        assert_eq!(lex("  1   2  "), vec![Token::Number(1.0), Token::Number(2.0)]);
        assert_eq!(lex(""), vec![]);
    }

    #[test]
    fn unclosed_quote_errors() {
        let err = Tokenizer::new("'abc").tokenize().unwrap_err();
        assert!(err.message().contains("Unclosed quote"));
    }

    #[test]
    fn unexpected_char_errors() {
        let err = Tokenizer::new("#").tokenize().unwrap_err();
        assert!(err.message().contains("Unexpected"));
    }

    #[test]
    fn char_predicates() {
        assert!(is_identifier_start('_'));
        assert!(is_identifier_start('$'));
        assert!(is_identifier_start('a'));
        assert!(is_identifier_start('Z'));
        assert!(!is_identifier_start('5'));
        assert!(is_identifier_char('5'));
        assert!(is_identifier_char('a'));
        assert!(is_digit('5'));
        assert!(!is_digit('a'));
    }

    #[test]
    fn regex_token_placeholder_is_constructible() {
        // M7-B 占位变体存在且可被匹配，即便 M7-A 词法器从不发出它。
        let t = Token::RegEx {
            pattern: r"\d+".to_string(),
            flags: "g".to_string(),
        };
        match t {
            Token::RegEx { pattern, flags } => {
                assert_eq!(pattern, r"\d+");
                assert_eq!(flags, "g");
            }
            _ => panic!("expected RegEx placeholder"),
        }
    }
}
