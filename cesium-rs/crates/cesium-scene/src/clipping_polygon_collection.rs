//! Ported from `packages/engine/Source/Scene/ClippingPolygonCollection.js`.

/// A collection of clipping polygons.
///
/// Manages multiple clipping polygons for terrain/3D Tiles clipping.
pub struct ClippingPolygonCollection {
    /// The number of polygons.
    pub length: u32,
    /// Whether the collection is dirty.
    pub dirty: bool,
}

impl ClippingPolygonCollection {
    /// Creates a new ClippingPolygonCollection.
    pub fn new() -> Self { Self { length: 0, dirty: true } }
}

impl Default for ClippingPolygonCollection {
    fn default() -> Self { Self::new() }
}
