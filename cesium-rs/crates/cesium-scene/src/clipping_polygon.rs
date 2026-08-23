//! Ported from `packages/engine/Source/Scene/ClippingPolygon.js`.

/// A clipping polygon.
pub struct ClippingPolygon {
    _private: (),
}

impl ClippingPolygon {
    /// Creates a new ClippingPolygon.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ClippingPolygon {
    fn default() -> Self { Self::new() }
}
