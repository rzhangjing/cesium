//! Ported from `packages/engine/Source/Scene/ExpressionNodeType.js`.
//!
//! The discriminants mirror the numeric values of the original JS object;
//! `is_literal_type` depends on the `>= LITERAL_NULL` ordering, exactly like
//! the original `isLiteralType` helper in Expression.js.

/// @private
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ExpressionNodeType {
    /// A `${name}` variable reference.
    Variable = 0,
    /// Unary operator or single-argument function.
    Unary = 1,
    /// Binary operator or two-argument function.
    Binary = 2,
    /// Three-argument function.
    Ternary = 3,
    /// Conditional `? :` expression.
    Conditional = 4,
    /// Member access (`.` or `[]`).
    Member = 5,
    /// Function call on an object (e.g. `regExp(...).test(...)`).
    FunctionCall = 6,
    /// Array literal.
    Array = 7,
    /// Regular expression constructed at evaluation time.
    Regex = 8,
    /// String literal containing `${name}` placeholders.
    VariableInString = 9,
    /// `null` literal.
    LiteralNull = 10,
    /// Boolean literal.
    LiteralBoolean = 11,
    /// Number literal.
    LiteralNumber = 12,
    /// String literal.
    LiteralString = 13,
    /// Color literal (`color`/`rgb`/`rgba`/`hsl`/`hsla`).
    LiteralColor = 14,
    /// Vector literal (`vec2`/`vec3`/`vec4`).
    LiteralVector = 15,
    /// Regular expression literal (pre-compiled).
    LiteralRegex = 16,
    /// `undefined` literal.
    LiteralUndefined = 17,
    /// Built-in variable (e.g. `tiles3d_tileset_time`).
    BuiltinVariable = 18,
}

impl ExpressionNodeType {
    /// Returns `true` when the node holds a literal value, mirroring
    /// `isLiteralType`: `node._type >= ExpressionNodeType.LITERAL_NULL`.
    pub fn is_literal_type(self) -> bool {
        self >= ExpressionNodeType::LiteralNull
    }
}
