//! Ported from `packages/engine/Source/Scene/Model/ModelLightingOptions.js`.

/// Lighting options for model rendering.
pub struct ModelLightingOptions {
    _private: (),
}

impl ModelLightingOptions {
    /// Creates a new ModelLightingOptions.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelLightingOptions {
    fn default() -> Self { Self::new() }
}
