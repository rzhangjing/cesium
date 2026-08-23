//! Ported from `packages/engine/Source/Scene/Model/ModelAlphaOptions.js`.

/// Alpha options for model rendering.
pub struct ModelAlphaOptions {
    _private: (),
}

impl ModelAlphaOptions {
    /// Creates a new ModelAlphaOptions.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelAlphaOptions {
    fn default() -> Self { Self::new() }
}
