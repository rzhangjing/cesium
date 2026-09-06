//! Ported from `packages/engine/Source/Scene/I3SGeometry.js`.

/// I3S geometry data.
///
/// Contains geometry buffers for I3S node rendering.
pub struct I3SGeometry {
    /// The number of vertices.
    pub vertex_count: u32,
    /// The number of indices.
    pub index_count: u32,
    /// Whether the geometry is loaded.
    pub loaded: bool,
}

impl I3SGeometry {
    /// Creates a new I3SGeometry.
    pub fn new() -> Self { Self { vertex_count: 0, index_count: 0, loaded: false } }
}

impl Default for I3SGeometry {
    fn default() -> Self { Self::new() }
}
