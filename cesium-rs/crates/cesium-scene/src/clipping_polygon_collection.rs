//! Ported from `packages/engine/Source/Scene/ClippingPolygonCollection.js`.

/// A collection of clipping polygons.
pub struct ClippingPolygonCollection {
    _private: (),
}

impl ClippingPolygonCollection {
    /// Creates a new ClippingPolygonCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ClippingPolygonCollection {
    fn default() -> Self { Self::new() }
}
