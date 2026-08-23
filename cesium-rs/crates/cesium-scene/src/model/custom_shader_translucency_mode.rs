//! Ported from `packages/engine/Source/Scene/Model/CustomShaderTranslucencyMode.js`.

/// Translucency mode for custom shaders.
pub struct CustomShaderTranslucencyMode {
    _private: (),
}

impl CustomShaderTranslucencyMode {
    /// Creates a new CustomShaderTranslucencyMode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CustomShaderTranslucencyMode {
    fn default() -> Self { Self::new() }
}
