//! Ported from `packages/engine/Source/Scene/ExpressionNodeType.js`.

/// Type of expression node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExpressionNodeType {
    /// Literal.
    Literal = 0,
    /// Property.
    Property = 1,
    /// Binary expression.
    BinaryExpression = 2,
    /// Unary expression.
    UnaryExpression = 3,
    /// Function call.
    FunctionCall = 4,
}
