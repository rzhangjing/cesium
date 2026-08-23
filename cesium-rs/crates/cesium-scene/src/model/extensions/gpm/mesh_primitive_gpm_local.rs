//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Local GPM data for mesh primitives.
pub struct MeshPrimitiveGpmLocal {
    _private: (),
}

impl MeshPrimitiveGpmLocal {
    /// Creates a new MeshPrimitiveGpmLocal.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MeshPrimitiveGpmLocal {
    fn default() -> Self { Self::new() }
}
