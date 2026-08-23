//! Ported from `packages/engine/Source/Scene/VertexAttributeSemantic.js`.

/// The semantic meaning of a vertex attribute.
pub struct VertexAttributeSemantic {
    _private: (),
}

impl VertexAttributeSemantic {
    /// Creates a new VertexAttributeSemantic.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VertexAttributeSemantic {
    fn default() -> Self { Self::new() }
}
