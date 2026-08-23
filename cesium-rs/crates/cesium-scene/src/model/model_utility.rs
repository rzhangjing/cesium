//! Ported from `packages/engine/Source/Scene/Model/ModelUtility.js`.

/// Utility functions for model processing.
pub struct ModelUtility {
    _private: (),
}

impl ModelUtility {
    /// Creates a new ModelUtility.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelUtility {
    fn default() -> Self { Self::new() }
}
