//! Ported from `packages/engine/Source/Scene/ClippingPolygon.js`.

/// A clipping polygon.
///
/// Defines a polygon region for clipping terrain and 3D Tiles.
pub struct ClippingPolygon {
    /// The polygon coordinates as (longitude, latitude) pairs.
    pub coordinates: Vec<(f64, f64)>,
    /// Whether the polygon is inverted.
    pub inverse: bool,
}

impl ClippingPolygon {
    /// Creates a new ClippingPolygon.
    pub fn new() -> Self { Self { coordinates: Vec::new(), inverse: false } }
}

impl Default for ClippingPolygon {
    fn default() -> Self { Self::new() }
}
