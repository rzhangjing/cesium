//! Ported from `packages/engine/Source/Scene/ConditionsExpression.js`.

/// A conditions expression.
pub struct ConditionsExpression {
    _private: (),
}

impl ConditionsExpression {
    /// Creates a new ConditionsExpression.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ConditionsExpression {
    fn default() -> Self { Self::new() }
}
