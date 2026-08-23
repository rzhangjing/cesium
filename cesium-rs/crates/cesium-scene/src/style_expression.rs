//! Ported from `packages/engine/Source/Scene/StyleExpression.js`.

/// An expression for 3D Tiles styling.
pub struct StyleExpression {
    _private: (),
}

impl StyleExpression {
    /// Creates a new StyleExpression.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for StyleExpression {
    fn default() -> Self { Self::new() }
}
