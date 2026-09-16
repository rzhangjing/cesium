//! Pratt parser: token stream -> jsep AST for the 3D Tiles Styling language.
//!
//! Ported from `cesium-rs/crates/cesium-scene/src/expression.rs` L672-897
//! (`binary_precedence` / `UNARY_PRECEDENCE` / `Parser`), the Rust reproduction
//! of jsep 1.3.8's operator-precedence parser plus the Cesium customizations
//! `addBinaryOp("=~", 0)` and `addBinaryOp("!~", 0)`.
//!
//! # M7-B scope notes
//!
//! * This module ONLY turns tokens into a [`JsepNode`]. The jsep-AST ->
//!   runtime-AST bridge (`parse_literal` / `parse_keywords_and_variables` /
//!   `parse_member_expression` / `parse_call` / `create_runtime_ast`) already
//!   lives in `ast.rs` (M7-A) and is **reused**, not re-implemented here.
//! * The precedence table and unary handling follow jsep 1.3.8. The ternary
//!   `?:` is handled at the top level only (`min_prec == 0`), *outside* the
//!   binary loop, so it binds looser than every binary operator. This
//!   deliberately fixes a blueprint bug where `?:` was nested inside the binary
//!   loop and mis-parsed `a || b ? c : d` as `a || (b ? c : d)` (correct:
//!   `(a || b) ? c : d`) and `a ? b : c || d` as `(a ? b : c) || d` (correct:
//!   `a ? b : (c || d)`). The negative-numeric-literal fold (`-2` ->
//!   `Literal(Number(-2))`) likewise happens only in a top-level binary
//!   context, so `--2` stays `Unary("-", Unary("-", 2))`.
//! * Ternary right-associativity is preserved: both branches recurse at
//!   `min_prec == 0`, so `a ? b : c ? d : e` == `a ? b : (c ? d : e)`.

use crate::ast::{JsepLiteral, JsepNode};
use crate::tokenizer::{Token, Tokenizer};
use crate::value::{runtime_error, RuntimeError};

/// Mirrors `jsep.binary_ops` (jsep 1.x) with the Cesium customizations
/// `addBinaryOp("=~", 0)` and `addBinaryOp("!~", 0)`. Returns `None` for a
/// token that is not a binary operator (so the Pratt loop stops).
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

/// Unary operators bind tighter than any binary operator.
pub const UNARY_PRECEDENCE: u8 = 15;

/// A recursive-descent / Pratt parser over the token stream.
pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    /// Parses a full expression string. Mirrors `jsep(expression)`: a trailing
    /// token (multiple expressions / a stray `;`) is rejected with
    /// `"Provide exactly one expression."`.
    pub fn parse(expression: &str) -> Result<JsepNode, RuntimeError> {
        let mut tokenizer = Tokenizer::new(expression);
        let tokens = tokenizer.tokenize()?;
        let mut parser = Parser { tokens, index: 0 };
        let node = parser.parse_expression(0)?;
        // Multiple expressions (or a trailing ";") yield a Compound node in
        // jsep, which createRuntimeAst rejects with "Provide exactly one
        // expression."
        if parser.peek().is_some() {
            return Err(runtime_error("Provide exactly one expression."));
        }
        Ok(node)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    /// Returns a clone of the next token's operator string, or `None` when the
    /// next token is not an operator. Cloning releases the borrow on `self` so
    /// the Pratt loop below can call `advance`; that is why the loop is written
    /// as a `while let` over an owned `String` rather than matching the borrowed
    /// `peek()` result directly (which would hold the borrow across `advance`).
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

    /// Parses a full binary-operator chain at `min_prec`, then — only at the top
    /// level (`min_prec == 0`) — any trailing `?:` conditional.
    ///
    /// The conditional is deliberately handled *outside* [`Self::parse_binary`]
    /// and only at `min_prec == 0`: the ternary has lower precedence than every
    /// binary operator (including `||`/`&&`). Recursing at `min_prec == 0` for
    /// both branches keeps the ternary right-associative and lets either branch
    /// contain a full binary chain or a nested `?:`.
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

    /// The Pratt binary-operator loop: parses the left operand with
    /// [`Self::parse_unary`], then consumes binary operators whose precedence is
    /// `>= min_prec`. Ternary `?:` is *not* handled here (see
    /// [`Self::parse_expression`]).
    fn parse_binary(&mut self, min_prec: u8) -> Result<JsepNode, RuntimeError> {
        let mut left = self.parse_unary()?;
        // jsep folds a leading `-<number>` into a negative numeric literal, but
        // only in a top-level binary context (`min_prec == 0`). Folding here —
        // rather than inside `parse_unary` — is what keeps `--2` as
        // `Unary("-", Unary("-", 2))` while still folding `-2` (and the left
        // operand of `-2 + 3`) to `Literal(Number(-2))`.
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
                // No negative-literal folding here: `-` always yields a `Unary`
                // node. The top-level fold lives in `parse_binary` so that
                // nested unary (`--2`) keeps its full `Unary`/`Unary` shape.
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

    // --- precedence table ---

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

    // --- literals / identifiers ---

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
        // -2 folds to Literal(Number(-2)), not Unary("-", 2).
        assert!(matches!(
            Parser::parse("-2").unwrap(),
            JsepNode::Literal(JsepLiteral::Number(n)) if n == -2.0
        ));
        // -x stays a Unary node.
        assert!(matches!(
            Parser::parse("-czm_x").unwrap(),
            JsepNode::Unary { operator, .. } if operator == "-"
        ));
    }

    // --- precedence in the tree shape ---

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
        // a + b =~ c  ==  (a + b) =~ c   (=~ precedence 0)
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

    // --- member / call / array ---

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
        // Nested member call: regExp("a").test("b")
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

    // --- error paths ---

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

    // --- ternary precedence relative to binary operators (task #46) ---

    #[test]
    fn ternary_binds_looser_than_binary_or() {
        // czm_a || czm_b ? czm_c : czm_d  ==  (a || b) ? c : d
        // (NOT a || (b ? c : d))
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
        // (NOT (a ? b : c) || d)
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

    // --- negative-numeric-literal folding shapes (task #46) ---

    #[test]
    fn parenthesized_negative_literal_is_folded() {
        // (-2) folds to Literal(Number(-2)).
        assert!(matches!(
            Parser::parse("(-2)").unwrap(),
            JsepNode::Literal(JsepLiteral::Number(n)) if n == -2.0
        ));
    }

    #[test]
    fn double_unary_minus_is_not_folded() {
        // --2 stays Unary("-", Unary("-", Literal(2))): the fold is top-level
        // only, so the inner `-2` (parsed at UNARY_PRECEDENCE) is never folded.
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
        // vec2(-1, -2): each argument is parsed at min_prec == 0, so both fold.
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
        // -2 + 3  ==  Literal(-2) + Literal(3)  (NOT Unary("-", 2 + 3)).
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
