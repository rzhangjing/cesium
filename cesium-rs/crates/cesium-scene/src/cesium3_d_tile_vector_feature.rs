//! Ported from `packages/engine/Source/Scene/Cesium3DTileVectorFeature.js`.

/// A vector feature within a 3D tile.
///
/// Represents a single vector feature with geometry and properties.
pub struct Cesium3DTileVectorFeature {
    /// The batch table hierarchical index.
    pub batch_id: u32,
    /// Whether this feature is visible.
    pub show: bool,
    /// The feature's property table index.
    pub property_table_index: u32,
}

impl Cesium3DTileVectorFeature {
    /// Creates a new Cesium3DTileVectorFeature.
    pub fn new() -> Self {
        Self { batch_id: 0, show: true, property_table_index: 0 }
    }
}

impl Default for Cesium3DTileVectorFeature {
    fn default() -> Self { Self::new() }
}
