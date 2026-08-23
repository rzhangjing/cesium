//! Ported from `packages/engine/Source/Scene/Model/UniformType.js`.

/// The type of a shader uniform.
pub struct UniformType {
    _private: (),
}

impl UniformType {
    /// Creates a new UniformType.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for UniformType {
    fn default() -> Self { Self::new() }
}
