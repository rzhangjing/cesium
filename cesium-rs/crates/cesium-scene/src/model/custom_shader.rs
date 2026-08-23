//! Ported from `packages/engine/Source/Scene/CustomShader.js`.

/// A custom shader that can be applied to a model.
pub struct CustomShader {
    pub vertex_shader_text: Option<String>,
    pub fragment_shader_text: Option<String>,
}

impl CustomShader {
    pub fn new() -> Self {
        Self { vertex_shader_text: None, fragment_shader_text: None }
    }
}

impl Default for CustomShader {
    fn default() -> Self { Self::new() }
}
