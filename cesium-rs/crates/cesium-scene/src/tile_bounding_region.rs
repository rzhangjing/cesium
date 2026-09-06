//! Ported from `packages/engine/Source/Scene/TileBoundingRegion.js`.

/// Tile bounding region.
///
/// Geographic bounding region for a 3D tile.
pub struct TileBoundingRegion {
    /// The south latitude in radians.
    pub south: f64,
    /// The west longitude in radians.
    pub west: f64,
    /// The north latitude in radians.
    pub north: f64,
    /// The east longitude in radians.
    pub east: f64,
    /// The minimum height.
    pub minimum_height: f64,
    /// The maximum height.
    pub maximum_height: f64,
}

impl TileBoundingRegion {
    /// Creates a new TileBoundingRegion.
    pub fn new() -> Self {
        Self { south: 0.0, west: 0.0, north: 0.0, east: 0.0, minimum_height: 0.0, maximum_height: 0.0 }
    }
}

impl Default for TileBoundingRegion {
    fn default() -> Self { Self::new() }
}
