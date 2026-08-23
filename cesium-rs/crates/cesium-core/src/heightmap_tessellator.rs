//! Ported from `packages/engine/Source/Core/HeightmapTessellator.js`.

/// Tessellates heightmap data into triangles.
pub struct HeightmapTessellator {
    _private: (),
}

impl HeightmapTessellator {
    /// Creates a new HeightmapTessellator.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for HeightmapTessellator {
    fn default() -> Self { Self::new() }
}
