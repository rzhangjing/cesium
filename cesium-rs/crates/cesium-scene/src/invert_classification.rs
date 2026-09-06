//! Ported from `packages/engine/Source/Scene/InvertClassification.js`.

/// Invert classification.
///
/// Inverts the classification rendering effect.
pub struct InvertClassification {
    /// Whether inversion is enabled.
    pub enabled: bool,
}

impl InvertClassification {
    /// Creates a new InvertClassification.
    pub fn new() -> Self { Self { enabled: false } }
}

impl Default for InvertClassification {
    fn default() -> Self { Self::new() }
}
