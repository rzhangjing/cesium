//! Pratt 解析器：针对 3D Tiles Styling 语言，把 token 流 -> jsep AST。
//!
//! 移植自 `cesium-rs/crates/cesium-scene/src/expression.rs` L672-897
//! （`binary_precedence` / `UNARY_PRECEDENCE` / `Parser`），它是 jsep 1.3.8
//! 运算符优先级解析器加上 Cesium 定制 `addBinaryOp("=~", 0)` 与
//! `addBinaryOp("!~", 0)` 的 Rust 复现。
//!
//! # M7-B 作用域说明
//!
//! * 本模块**只**把 token 变成 [`JsepNode`]。jsep-AST -> runtime-AST 的桥接
//!   （`parse_literal` / `parse_keywords_and_variables` /
//!   `parse_member_expression` / `parse_call` / `create_runtime_ast`）
//!   已存在于 `ast.rs`（M7-A），这里**复用**而非重新实现。
//! * 优先级表与一元处理遵循 jsep 1.3.8。三元 `?:` 只在顶层处理
//!   （`min_prec == 0`），在二元循环*之外*，因此它比所有二元运算符结合得都松。
//!   这有意修复了 blueprint 的一个 bug：`?:` 被嵌在二元循环内部，会把
//!   `a || b ? c : d` 误解析为 `a || (b ? c : d)`（正确：`(a || b) ? c : d`），
//!   把 `a ? b : c || d` 误解析为 `(a ? b : c) || d`（正确：`a ? b : (c || d)`）。
//!   负数字面量折叠（`-2` -> `Literal(Number(-2))`）同样只在顶层二元上下文中
//!   发生，所以 `--2` 保持为 `Unary("-", Unary("-", 2))`。
//! * 三元右结合性得以保留：两个分支都在 `min_prec == 0` 处递归，
//!   所以 `a ? b : c ? d : e` == `a ? b : (c ? d : e)`。

use crate::ast::{JsepLiteral, JsepNode};
use crate::tokenizer::{Token, Tokenizer};
use crate::value::{runtime_error, RuntimeError};

/// 镜像 `jsep.binary_ops`（jsep 1.x），带 Cesium 定制
/// `addBinaryOp("=~", 0)` 与 `addBinaryOp("!~", 0)`。对非二元运算符的
/// token 返回 `None`（于是 Pratt 循环停止）。
pub fn binary_precedence(operator: &str) -> Option<u8> {
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

/// 一元运算符比任何二元运算符结合得更紧。
pub const UNARY_PRECEDENCE: u8 = 15;

/// 作用于 token 流的递归下降 / Pratt 解析器。
pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    /// 解析一个完整表达式字符串。镜像 `jsep(expression)`：尾部多余的
    /// token（多个表达式 / 游离的 `;`）以 `"Provide exactly one expression."` 拒绝。
    pub fn parse(expression: &str) -> Result<JsepNode, RuntimeError> {
        let mut tokenizer = Tokenizer::new(expression);
        let tokens = tokenizer.tokenize()?;
        let mut parser = Parser { tokens, index: 0 };
        let node = parser.parse_expression(0)?;
        // 多个表达式（或尾部的 ";"）在 jsep 里产生一个 Compound 节点，
        // createRuntimeAst 会以 "Provide exactly one expression." 拒绝它。
        if parser.peek().is_some() {
            return Err(runtime_error("Provide exactly one expression."));
        }
        Ok(node)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    /// 返回下一个 token 运算符字符串的克隆，若下一个 token 不是运算符则返回
    /// `None`。克隆会释放对 `self` 的借用，从而下面的 Pratt 循环可以调用
    /// `advance`；这就是为什么该循环写成对拥有所有权的 `String` 的 `while let`，
    /// 而非直接匹配借用的 `peek()` 结果（后者会在 `advance` 期间一直持有借用）。
    fn peek_op(&self) -> Option<String> {
        match self.peek() {
            Some(Token::Op(op)) => Some(op.clone()),
            _ => None,
        }
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    /// 在 `min_prec` 下解析一条完整的二元运算符链，然后——仅在顶层
    /// （`min_prec == 0`）——处理任何尾随的 `?:` 条件式。
    ///
    /// 该条件式有意放在 [`Self::parse_binary`] *之外*且仅在 `min_prec == 0`
    /// 处理：三元的优先级低于每个二元运算符（包括 `||`/`&&`）。两个分支都在
    /// `min_prec == 0` 处递归，使三元保持右结合，并允许任一分支包含一条完整的
    /// 二元链或嵌套的 `?:`。
    fn parse_expression(&mut self, min_prec: u8) -> Result<JsepNode, RuntimeError> {
        let mut left = self.parse_binary(min_prec)?;
        if min_prec == 0 {
            while self.peek_op().as_deref() == Some("?") {
                self.advance();
                let consequent = self.parse_expression(0)?;
                match self.advance() {
                    Some(Token::Op(op)) if op == ":" => {}
                    _ => return Err(runtime_error("Expected :")),
                }
                let alternate = self.parse_expression(0)?;
                left = JsepNode::Conditional {
                    test: Box::new(left),
                    consequent: Box::new(consequent),
                    alternate: Box::new(alternate),
                };
            }
        }
        Ok(left)
    }

    /// Pratt 二元运算符循环：用 [`Self::parse_unary`] 解析左操作数，然后消费
    /// 优先级 `>= min_prec` 的二元运算符。三元 `?:` 不在此处理
    /// （见 [`Self::parse_expression`]）。
    fn parse_binary(&mut self, min_prec: u8) -> Result<JsepNode, RuntimeError> {
        let mut left = self.parse_unary()?;
        // jsep 会把前导的 `-<number>` 折叠为一个负的数值字面量，但仅在顶层
        // 二元上下文中（`min_prec == 0`）。折叠放在这里——而非 `parse_unary`
        // 内部——正是它让 `--2` 保持为 `Unary("-", Unary("-", 2))`，同时仍把
        // `-2`（以及 `-2 + 3` 的左操作数）折叠为 `Literal(Number(-2))`。
        if min_prec == 0 {
            let folded = match &left {
                JsepNode::Unary { operator, argument } if operator == "-" => {
                    match argument.as_ref() {
                        JsepNode::Literal(JsepLiteral::Number(value)) => Some(-*value),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(value) = folded {
                left = JsepNode::Literal(JsepLiteral::Number(value));
            }
        }
        while let Some(operator) = self.peek_op() {
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
                // 此处不做负字面量折叠：`-` 总是产生一个 `Unary` 节点。
                // 顶层折叠位于 `parse_binary` 中，从而嵌套一元（`--2`）
                // 保持其完整的 `Unary`/`Unary` 形态。
                let argument = self.parse_expression(UNARY_PRECEDENCE)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- 优先级表 ---

    #[test]
    fn binary_precedence_table() {
        assert_eq!(binary_precedence("=~"), Some(0));
        assert_eq!(binary_precedence("!~"), Some(0));
        assert_eq!(binary_precedence("||"), Some(1));
        assert_eq!(binary_precedence("&&"), Some(2));
        assert_eq!(binary_precedence("==="), Some(6));
        assert_eq!(binary_precedence("<"), Some(7));
        assert_eq!(binary_precedence("+"), Some(9));
        assert_eq!(binary_precedence("*"), Some(10));
        assert_eq!(binary_precedence("("), None);
        assert_eq!(binary_precedence("?"), None);
    }

    // --- 字面量 / 标识符 ---

    #[test]
    fn parses_number_and_string_literals() {
        assert!(matches!(
            Parser::parse("42").unwrap(),
            JsepNode::Literal(JsepLiteral::Number(n)) if n == 42.0
        ));
        assert!(matches!(
            Parser::parse("'hi'").unwrap(),
            JsepNode::Literal(JsepLiteral::Str(s)) if s == "hi"
        ));
        assert!(matches!(
            Parser::parse("true").unwrap(),
            JsepNode::Literal(JsepLiteral::Boolean(true))
        ));
        assert!(matches!(
            Parser::parse("null").unwrap(),
            JsepNode::Literal(JsepLiteral::Null)
        ));
    }

    #[test]
    fn negative_numeric_literal_is_folded() {
        // -2 折叠为 Literal(Number(-2))，而非 Unary("-", 2)。
        assert!(matches!(
            Parser::parse("-2").unwrap(),
            JsepNode::Literal(JsepLiteral::Number(n)) if n == -2.0
        ));
        // -x 保持为 Unary 节点。
        assert!(matches!(
            Parser::parse("-czm_x").unwrap(),
            JsepNode::Unary { operator, .. } if operator == "-"
        ));
    }

    // --- 树形态中的优先级 ---

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        // 1 + 2 * 3 == 1 + (2 * 3)
        let node = Parser::parse("1 + 2 * 3").unwrap();
        match node {
            JsepNode::Binary { operator, right, .. } => {
                assert_eq!(operator, "+");
                assert!(matches!(*right, JsepNode::Binary { operator, .. } if operator == "*"));
            }
            _ => panic!("expected Binary +"),
        }
    }

    #[test]
    fn comparison_binds_looser_than_arithmetic() {
        // 1 + 2 < 3 == (1 + 2) < 3
        let node = Parser::parse("1 + 2 < 3").unwrap();
        match node {
            JsepNode::Binary { operator, left, .. } => {
                assert_eq!(operator, "<");
                assert!(matches!(*left, JsepNode::Binary { operator, .. } if operator == "+"));
            }
            _ => panic!("expected Binary <"),
        }
    }

    #[test]
    fn regex_operators_have_lowest_precedence() {
        // a + b =~ c  ==  (a + b) =~ c  （=~ 优先级 0）
        let node = Parser::parse("czm_a + czm_b =~ czm_c").unwrap();
        match node {
            JsepNode::Binary { operator, left, .. } => {
                assert_eq!(operator, "=~");
                assert!(matches!(*left, JsepNode::Binary { operator, .. } if operator == "+"));
            }
            _ => panic!("expected Binary =~"),
        }
    }

    #[test]
    fn logical_precedence_and_over_or() {
        // a || b && c  ==  a || (b && c)
        let node = Parser::parse("czm_a || czm_b && czm_c").unwrap();
        match node {
            JsepNode::Binary { operator, right, .. } => {
                assert_eq!(operator, "||");
                assert!(matches!(*right, JsepNode::Binary { operator, .. } if operator == "&&"));
            }
            _ => panic!("expected Binary ||"),
        }
    }

    #[test]
    fn ternary_is_right_associative() {
        // a ? b : c ? d : e  ==  a ? b : (c ? d : e)
        let node = Parser::parse("czm_a ? czm_b : czm_c ? czm_d : czm_e").unwrap();
        match node {
            JsepNode::Conditional { alternate, .. } => {
                assert!(matches!(*alternate, JsepNode::Conditional { .. }));
            }
            _ => panic!("expected Conditional"),
        }
    }

    // --- 成员 / 调用 / 数组 ---

    #[test]
    fn member_and_index_access() {
        assert!(matches!(
            Parser::parse("czm_a.b").unwrap(),
            JsepNode::Member { computed: false, .. }
        ));
        assert!(matches!(
            Parser::parse("czm_a[0]").unwrap(),
            JsepNode::Member { computed: true, .. }
        ));
    }

    #[test]
    fn call_with_arguments() {
        let node = Parser::parse("vec3(1, 2, 3)").unwrap();
        match node {
            JsepNode::Call { arguments, .. } => assert_eq!(arguments.len(), 3),
            _ => panic!("expected Call"),
        }
        // 嵌套成员调用：regExp("a").test("b")
        let node = Parser::parse("regExp('a').test('b')").unwrap();
        assert!(matches!(node, JsepNode::Call { .. }));
    }

    #[test]
    fn array_literal() {
        let node = Parser::parse("[1, 2, 3]").unwrap();
        match node {
            JsepNode::Array(elements) => assert_eq!(elements.len(), 3),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn parenthesized_grouping_overrides_precedence() {
        // (1 + 2) * 3
        let node = Parser::parse("(1 + 2) * 3").unwrap();
        match node {
            JsepNode::Binary { operator, left, .. } => {
                assert_eq!(operator, "*");
                assert!(matches!(*left, JsepNode::Binary { operator, .. } if operator == "+"));
            }
            _ => panic!("expected Binary *"),
        }
    }

    // --- 错误路径 ---

    #[test]
    fn trailing_expression_errors() {
        let err = Parser::parse("1 2").unwrap_err();
        assert!(err.message().contains("Provide exactly one expression."));
    }

    #[test]
    fn unclosed_paren_errors() {
        assert!(Parser::parse("(1 + 2").is_err());
    }

    #[test]
    fn empty_expression_errors() {
        let err = Parser::parse("").unwrap_err();
        assert!(err.message().contains("Provide exactly one expression."));
    }

    #[test]
    fn missing_ternary_colon_errors() {
        let err = Parser::parse("czm_a ? czm_b").unwrap_err();
        assert!(err.message().contains("Expected :"));
    }

    // --- 三元相对于二元运算符的优先级（任务 #46） ---

    #[test]
    fn ternary_binds_looser_than_binary_or() {
        // czm_a || czm_b ? czm_c : czm_d  ==  (a || b) ? c : d
        // （不是 a || (b ? c : d)）
        let node = Parser::parse("czm_a || czm_b ? czm_c : czm_d").unwrap();
        match node {
            JsepNode::Conditional { test, consequent, alternate } => {
                assert!(matches!(*test, JsepNode::Binary { operator, .. } if operator == "||"));
                assert!(matches!(*consequent, JsepNode::Identifier(ref n) if n.as_str() == "czm_c"));
                assert!(matches!(*alternate, JsepNode::Identifier(ref n) if n.as_str() == "czm_d"));
            }
            _ => panic!("expected Conditional with a Binary(||) test"),
        }
    }

    #[test]
    fn ternary_alternate_absorbs_trailing_binary() {
        // czm_a ? czm_b : czm_c || czm_d  ==  a ? b : (c || d)
        // （不是 (a ? b : c) || d）
        let node = Parser::parse("czm_a ? czm_b : czm_c || czm_d").unwrap();
        match node {
            JsepNode::Conditional { test, consequent, alternate } => {
                assert!(matches!(*test, JsepNode::Identifier(ref n) if n.as_str() == "czm_a"));
                assert!(matches!(*consequent, JsepNode::Identifier(ref n) if n.as_str() == "czm_b"));
                assert!(matches!(*alternate, JsepNode::Binary { operator, .. } if operator == "||"));
            }
            _ => panic!("expected Conditional with a Binary(||) alternate"),
        }
    }

    // --- 负数值字面量折叠的形态（任务 #46） ---

    #[test]
    fn parenthesized_negative_literal_is_folded() {
        // (-2) 折叠为 Literal(Number(-2))。
        assert!(matches!(
            Parser::parse("(-2)").unwrap(),
            JsepNode::Literal(JsepLiteral::Number(n)) if n == -2.0
        ));
    }

    #[test]
    fn double_unary_minus_is_not_folded() {
        // --2 保持为 Unary("-", Unary("-", Literal(2)))：折叠只在顶层，
        // 所以内层的 `-2`（在 UNARY_PRECEDENCE 下解析）从不被折叠。
        match Parser::parse("--2").unwrap() {
            JsepNode::Unary { operator, argument } => {
                assert_eq!(operator, "-");
                match *argument {
                    JsepNode::Unary { operator: inner_op, argument: inner } => {
                        assert_eq!(inner_op, "-");
                        assert!(matches!(*inner, JsepNode::Literal(JsepLiteral::Number(n)) if n == 2.0));
                    }
                    _ => panic!("expected inner Unary(-, 2)"),
                }
            }
            _ => panic!("expected outer Unary(-, ...)"),
        }
    }

    #[test]
    fn call_arguments_fold_negative_literals() {
        // vec2(-1, -2)：每个参数都在 min_prec == 0 下解析，所以两者都折叠。
        match Parser::parse("vec2(-1, -2)").unwrap() {
            JsepNode::Call { arguments, .. } => {
                assert_eq!(arguments.len(), 2);
                assert!(matches!(arguments[0], JsepNode::Literal(JsepLiteral::Number(n)) if n == -1.0));
                assert!(matches!(arguments[1], JsepNode::Literal(JsepLiteral::Number(n)) if n == -2.0));
            }
            _ => panic!("expected Call"),
        }
    }

    #[test]
    fn negative_left_operand_folds_not_the_whole_sum() {
        // -2 + 3  ==  Literal(-2) + Literal(3)  （不是 Unary("-", 2 + 3)）。
        match Parser::parse("-2 + 3").unwrap() {
            JsepNode::Binary { operator, left, right } => {
                assert_eq!(operator, "+");
                assert!(matches!(*left, JsepNode::Literal(JsepLiteral::Number(n)) if n == -2.0));
                assert!(matches!(*right, JsepNode::Literal(JsepLiteral::Number(n)) if n == 3.0));
            }
            _ => panic!("expected Binary(+, Literal(-2), Literal(3))"),
        }
    }
}
