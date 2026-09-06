//! Ported from `packages/engine/Source/Scene/ShadowMapShader.js`.

/// Shadow map shader.
///
/// Shader utilities for shadow map rendering.
pub struct ShadowMapShader {
    /// Whether the shader is compiled.
    pub compiled: bool,
}

impl ShadowMapShader {
    /// Creates a new ShadowMapShader.
    pub fn new() -> Self { Self { compiled: false } }
}

impl Default for ShadowMapShader {
    fn default() -> Self { Self::new() }
}
