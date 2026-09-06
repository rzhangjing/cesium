//! Ported from `packages/engine/Source/Scene/Cesium3DTilePointFeature.js`.

/// A point feature within a 3D tile.
///
/// Represents a single 3D point feature with position and properties.
pub struct Cesium3DTilePointFeature {
    /// The batch table hierarchical index.
    pub batch_id: u32,
    /// The position x component.
    pub position_x: f64,
    /// The position y component.
    pub position_y: f64,
    /// The position z component.
    pub position_z: f64,
    /// Whether this feature is visible.
    pub show: bool,
}

impl Cesium3DTilePointFeature {
    /// Creates a new Cesium3DTilePointFeature.
    pub fn new() -> Self {
        Self { batch_id: 0, position_x: 0.0, position_y: 0.0, position_z: 0.0, show: true }
    }
}

impl Default for Cesium3DTilePointFeature {
    fn default() -> Self { Self::new() }
}
