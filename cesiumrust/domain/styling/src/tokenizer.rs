//! Tokenizer for the 3D Tiles Styling expression language.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs` L447-633
//! (`Token` + `Tokenizer` + char predicates + string/number/identifier/operator
//! lexing), the Rust reproduction of the subset of jsep 1.3.8 semantics the
//! styling language needs (upstream `packages/engine/Source/Scene/Expression.js`
//! parses with jsep plus `addBinaryOp("=~"/"!~", 0)`).
//!
//! # M7-A scope notes
//!
//! * DEVIATION (jsep): there is **no regex-literal lexing**. [`Token::RegEx`] is
//!   a PLACEHOLDER variant reserved for M7-B (`regex.rs`); the M7-A tokenizer
//!   never produces it. Cesium styling builds regexes via the `regExp()`
//!   function rather than `/.../` literals, so the blueprint tokenizer has no
//!   regex-literal path either — the variant exists so M7-B can extend lexing
//!   without a breaking change to `Token`.
//! * The `is_whitespace` predicate mentioned in the task brief is the builtin
//!   `char::is_whitespace`, used directly in [`Tokenizer::skip_whitespace`]
//!   (blueprint-faithful; no redundant wrapper is introduced).
//! * Minor idiom adaptation: the two-character operator dispatch uses `matches!`
//!   instead of the blueprint's `match { .. , _ => {} }` to stay clippy-clean
//!   (`clippy::single_match`); semantics are identical.

use crate::value::{runtime_error, RuntimeError};

/// A lexical token, mirroring the jsep token stream.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Str(String),
    Ident(String),
    Op(String),
    /// M7-B placeholder for a `/pattern/flags` regex literal — never produced by
    /// the M7-A tokenizer (see module docs).
    #[allow(dead_code)] // M7-B placeholder; not constructed in the M7-A base layer
    RegEx { pattern: String, flags: String },
}

/// A hand-written tokenizer mirroring jsep 1.3.8's lexer.
pub struct Tokenizer<'a> {
    chars: Vec<char>,
    index: usize,
    source: &'a str,
}

/// `true` for `[A-Za-z_$]` — the set jsep allows to start an identifier.
pub fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

/// `true` for `[A-Za-z0-9_$]` — the set jsep allows inside an identifier.
pub fn is_identifier_char(c: char) -> bool {
    is_identifier_start(c) || c.is_ascii_digit()
}

/// `true` for `[0-9]`.
pub fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

impl<'a> Tokenizer<'a> {
    /// Creates a tokenizer over `source`.
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

    /// Lexes the whole input into a token vector.
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

    /// Strings carry no escape processing: backslashes were already replaced
    /// with `"@#%"` by `removeBackslashes`, mirroring jsep's behavior after the
    /// preprocessing step.
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
        if matches!(
            two.as_str(),
            "&&" | "||" | "=~" | "!~" | ">=" | "<=" | "<<" | ">>"
        ) {
            self.index += 2;
            return Ok(two);
        }
        let c = self.advance().unwrap();
        // jsep accepts these operators; unsupported ones are rejected later by
        // create_runtime_ast with `Unexpected operator "{op}".`.
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
        // The M7-B placeholder variant exists and is matchable even though the
        // M7-A tokenizer never emits it.
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
