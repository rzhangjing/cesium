//! Ported from `packages/engine/Source/Renderer/ShaderFunction.js`.
//!
//! Describes a function within a shader program.

use crate::shader_destination::ShaderDestination;

/// Describes a function to be injected into a shader.
pub struct ShaderFunction {
    /// The GLSL function signature.
    pub signature: String,
    /// The function body.
    pub body: String,
    /// Which shader stages this function should be added to.
    pub shader_destination: ShaderDestination,
}

impl ShaderFunction {
    /// Creates a new shader function.
    pub fn new(signature: String, body: String, shader_destination: ShaderDestination) -> Self {
        Self { signature, body, shader_destination }
    }

    /// Generates the GLSL function source.
    pub fn to_glsl(&self) -> String {
        format!("{} {{\n{}\n}}\n", self.signature, self.body)
    }
}
