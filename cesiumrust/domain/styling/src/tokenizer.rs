//! 3D Tiles Styling 表达式语言的词法分析器。
//!
//! 提供 `Token` + `Tokenizer` + 字符谓词 + 字符串/数字/标识符/运算符
//! 词法，是 styling 语言所需的 jsep 1.3.8 语义子集的 Rust 复现
//! （jsep 加 `addBinaryOp("=~"/"!~", 0)` 解析）。
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
    /// 数字字面量。
    Number(f64),
    /// 字符串字面量（已去外层引号）。
    Str(String),
    /// 标识符（变量名/函数名/关键字）。
    Ident(String),
    /// 运算符或分隔符文本。
    Op(String),
    /// 一个 `/pattern/flags` 正则字面量的 M7-B 占位 —— M7-A 词法器
    /// 从不产出（见模块文档）。
    #[allow(dead_code)] // M7-B 占位；在 M7-A 基础层中不构造
    RegEx { pattern: String, flags: String },
}

/// 一个手写的词法分析器，镜像 jsep 1.3.8 的词法器。
pub struct Tokenizer<'a> {
    /// 输入按字符展开的缓存，便于随机游标访问。
    chars: Vec<char>,
    /// 当前读取游标。
    index: usize,
    /// 原始输入串（用于错误消息回显）。
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

    /// 窥视当前字符而不前进游标。
    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    /// 从当前游标向后偏移 `offset` 窥视字符（用于多字符运算符前瞻）。
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    /// 前进游标一个字符并返回它（越界返回 None）。
    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.index += 1;
        }
        c
    }

    /// 跳过游标处的所有空白字符。
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
            // 每轮先吃掉游标处的空白，再判断下一 token 类型。
            self.skip_whitespace();
            // 窥视首个非空白字符；越界即词法结束。
            let c = match self.peek() {
                Some(c) => c,
                None => break,
            };
            // 分派四类：引号 -> 字符串；数字/前导小数点 -> 数字；
            // 标识符起始 -> 标识符；其余 -> 运算符。
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

    /// 扫描一个数字字面量：整数部分、可选小数部分与可选指数部分。
    fn read_number(&mut self) -> f64 {
        let start = self.index;
        // 第一段：整数部分的连续数字。
        while self.peek().map(is_digit) == Some(true) {
            self.index += 1;
        }
        // 第二段：可选小数点 + 小数位。
        if self.peek() == Some('.') {
            self.index += 1;
            while self.peek().map(is_digit) == Some(true) {
                self.index += 1;
            }
        }
        // 第三段：可选指数 e/E，带可选正负号时必须至少跟一位数字，
        // 否则回退（把 e 留给标识符/运算符）。
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

    /// 扫描一个标识符（从当前位置消耗所有合法标识符字符）。
    fn read_identifier(&mut self) -> String {
        let start = self.index;
        while self.peek().map(is_identifier_char) == Some(true) {
            self.index += 1;
        }
        self.chars[start..self.index].iter().collect()
    }

    /// 扫描一个运算符，优先匹配三字符、再二字符、最后一字符。
    fn read_operator(&mut self) -> Result<String, RuntimeError> {
        // 三级贪婪匹配：先试三字符（===/!==/>>>），命中即前进 3。
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

    /// 词法辅助：对输入串断言能成功 tokenize 并返回 token 向量。
    fn lex(input: &str) -> Vec<Token> {
        Tokenizer::new(input).tokenize().expect("tokenize failed")
    }

    /// 验证 "1 + 2" 被切成数字/运算符/数字三个 token。
    #[test]
    fn numbers_and_operators() {
        assert_eq!(
            lex("1 + 2"),
            vec![Token::Number(1.0), Token::Op("+".into()), Token::Number(2.0)]
        );
    }

    /// 验证标识符词法：字母开头与含 `_`/`$`/数字的混合名。
    #[test]
    fn identifiers() {
        assert_eq!(lex("foo"), vec![Token::Ident("foo".into())]);
        assert_eq!(lex("_bar$9"), vec![Token::Ident("_bar$9".into())]);
    }

    /// 验证单/双引号字符串都被去外层引号产出 [`Token::Str`]。
    #[test]
    fn strings_single_and_double_quotes() {
        assert_eq!(lex("'hello'"), vec![Token::Str("hello".into())]);
        assert_eq!(lex("\"world\""), vec![Token::Str("world".into())]);
    }

    /// 验证各种数字形态：小数、前导小数点、正/负指数、尾随小数点。
    #[test]
    fn number_forms() {
        assert_eq!(lex("1.5"), vec![Token::Number(1.5)]);
        assert_eq!(lex(".5"), vec![Token::Number(0.5)]);
        assert_eq!(lex("1e3"), vec![Token::Number(1000.0)]);
        assert_eq!(lex("1E-2"), vec![Token::Number(0.01)]);
        assert_eq!(lex("12."), vec![Token::Number(12.0)]);
    }

    /// 验证多字符运算符（===、!==、=~、!~、&&、>=）作为整体产出。
    #[test]
    fn multi_char_operators() {
        assert_eq!(lex("==="), vec![Token::Op("===".into())]);
        assert_eq!(lex("!=="), vec![Token::Op("!==".into())]);
        assert_eq!(lex("=~"), vec![Token::Op("=~".into())]);
        assert_eq!(lex("!~"), vec![Token::Op("!~".into())]);
        assert_eq!(lex("&&"), vec![Token::Op("&&".into())]);
        assert_eq!(lex(">="), vec![Token::Op(">=".into())]);
    }

    /// 验证成员访问（`.`）与索引访问（`[]`）拆成独立运算符 token。
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

    /// 验证三元条件运算符 `?` `:` 各自成独立 token。
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

    /// 验证空白被跳过，空输入产出空 token 列。
    #[test]
    fn whitespace_is_skipped() {
        assert_eq!(lex("  1   2  "), vec![Token::Number(1.0), Token::Number(2.0)]);
        assert_eq!(lex(""), vec![]);
    }

    /// 验证未闭合引号以 "Unclosed quote" 报错。
    #[test]
    fn unclosed_quote_errors() {
        let err = Tokenizer::new("'abc").tokenize().unwrap_err();
        assert!(err.message().contains("Unclosed quote"));
    }

    /// 验证非法字符以 "Unexpected" 报错。
    #[test]
    fn unexpected_char_errors() {
        let err = Tokenizer::new("#").tokenize().unwrap_err();
        assert!(err.message().contains("Unexpected"));
    }

    /// 验证三个字符谓词对字母/下划线/`$`/数字的真假判定。
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
