//! Ported from `packages/engine/Source/Scene/StyleExpression.js`.

/// Style expression.
///
/// Evaluates 3D Tiles style expressions.
pub struct StyleExpression {
    /// The expression string.
    pub expression: String,
}

impl StyleExpression {
    /// Creates a new StyleExpression.
    pub fn new() -> Self { Self { expression: String::new() } }
}

impl Default for StyleExpression {
    fn default() -> Self { Self::new() }
}
