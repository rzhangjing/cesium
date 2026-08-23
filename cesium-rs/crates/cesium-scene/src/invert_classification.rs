//! Ported from `packages/engine/Source/Scene/InvertClassification.js`.

/// Invert classification effect.
pub struct InvertClassification {
    _private: (),
}

impl InvertClassification {
    /// Creates a new InvertClassification.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for InvertClassification {
    fn default() -> Self { Self::new() }
}
