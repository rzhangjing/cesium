//! Ported from `packages/engine/Source/Scene/Model/CustomShaderMode.js`.

/// The mode for custom shader execution.
pub struct CustomShaderMode {
    _private: (),
}

impl CustomShaderMode {
    /// Creates a new CustomShaderMode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CustomShaderMode {
    fn default() -> Self { Self::new() }
}
