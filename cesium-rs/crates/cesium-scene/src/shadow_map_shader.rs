//! Ported from `packages/engine/Source/Scene/ShadowMapShader.js`.

/// Shader for shadow map rendering.
pub struct ShadowMapShader {
    _private: (),
}

impl ShadowMapShader {
    /// Creates a new ShadowMapShader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ShadowMapShader {
    fn default() -> Self { Self::new() }
}
