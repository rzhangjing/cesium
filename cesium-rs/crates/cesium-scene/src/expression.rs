//! Ported from `packages/engine/Source/Scene/Expression.js`.
//!
//! An expression that evaluates a string expression against a feature.

use std::collections::HashMap;

/// An expression that evaluates a string expression against a feature.
///
/// Used for 3D Tiles styling and conditions expressions.
/// Mirrors CesiumJS `Expression` (752 lines).
pub struct Expression {
    /// The expression string.
    expression_string: String,
    /// Whether the expression has been compiled.
    compiled: bool,
}

impl Expression {
    /// Creates a new Expression from a string.
    pub fn new(expression_string: &str) -> Self {
        Self {
            expression_string: expression_string.to_string(),
            compiled: false,
        }
    }

    /// Evaluates the expression against a feature's properties.
    pub fn evaluate(&self, _properties: &HashMap<String, String>) -> Option<String> {
        // DEVIATION: Requires expression parser/evaluator (ConditionsExpression)
        None
    }

    /// Returns the expression string.
    pub fn expression_string(&self) -> &str {
        &self.expression_string
    }

    /// Returns whether the expression has been compiled.
    pub fn is_compiled(&self) -> bool {
        self.compiled
    }
}

impl Default for Expression {
    fn default() -> Self { Self::new("") }
}
