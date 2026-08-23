//! Ported from `packages/engine/Source/Renderer/createUniformArray.js`.

/// Creates a uniform array for a shader program.
pub struct UniformArray {
    /// The name of the uniform array.
    pub name: String,
    /// The number of elements in the array.
    pub count: usize,
}

impl UniformArray {
    pub fn new(name: &str, count: usize) -> Self {
        Self { name: name.to_string(), count }
    }
}
