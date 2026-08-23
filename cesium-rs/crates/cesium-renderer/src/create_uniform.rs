//! Ported from `packages/engine/Source/Renderer/createUniform.js`.
//!
//! Creates a uniform object for a shader program.

/// Creates a uniform object for a shader program.
///
/// DEVIATION: In wgpu, uniforms are managed via bind groups and
/// uniform buffers rather than individual GL uniform calls.
pub struct Uniform {
    /// The name of the uniform.
    pub name: String,
    /// The location of the uniform (if applicable).
    pub location: Option<u32>,
}

impl Uniform {
    /// Creates a new uniform.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), location: None }
    }
}
