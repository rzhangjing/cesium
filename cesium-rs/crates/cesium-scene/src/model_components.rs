//! Ported from `packages/engine/Source/Scene/ModelComponents.js`.

/// Components of a loaded model.
pub struct ModelComponents {
    _private: (),
}

impl ModelComponents {
    /// Creates a new ModelComponents.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelComponents {
    fn default() -> Self { Self::new() }
}
