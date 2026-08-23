//! Ported from `packages/engine/Source/Scene/Model/ModelImagery.js`.

/// Imagery data for a model.
pub struct ModelImagery {
    _private: (),
}

impl ModelImagery {
    /// Creates a new ModelImagery.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelImagery {
    fn default() -> Self { Self::new() }
}
