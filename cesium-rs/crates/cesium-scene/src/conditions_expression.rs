//! Ported from `packages/engine/Source/Scene/ConditionsExpression.js`.

/// Conditions expression.
///
/// Evaluates conditional style expressions for 3D Tiles styling.
pub struct ConditionsExpression {
    /// The expression conditions.
    pub conditions: Vec<(String, String)>,
}

impl ConditionsExpression {
    /// Creates a new ConditionsExpression.
    pub fn new() -> Self { Self { conditions: Vec::new() } }
}

impl Default for ConditionsExpression {
    fn default() -> Self { Self::new() }
}
