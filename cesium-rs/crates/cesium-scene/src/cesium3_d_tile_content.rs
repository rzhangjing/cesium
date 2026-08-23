//! Ported from `packages/engine/Source/Scene/Cesium3DTileContent.js`.
//!
//! The content of a tile in a 3D Tiles tileset.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;

use crate::cesium3_d_tile_content_state::Cesium3DTileContentState;

/// The content of a tile in a [`Cesium3DTileset`](crate::cesium3_d_tileset::Cesium3DTileset).
///
/// Derived types provide access to individual features in the tile.
/// This mirrors the CesiumJS `Cesium3DTileContent` interface (357 lines).
pub struct Cesium3DTileContent {
    /// The number of features in the tile.
    pub features_length: i32,
    /// The number of points in the tile (point cloud content only).
    pub points_length: i32,
    /// The number of triangles in the tile.
    pub triangles: i32,
    /// The byte length of the tile content.
    pub byte_length: i32,
    /// The current content state.
    pub state: Cesium3DTileContentState,
    /// The bounding volume of the content.
    pub bounding_volume: Option<BoundingSphere>,
    /// The content URI.
    pub uri: Option<String>,
    /// Whether any feature's property changed.
    pub feature_properties_dirty: bool,
    /// The tileset URL for resolving relative resources.
    pub tileset_url: Option<String>,
}

impl Cesium3DTileContent {
    /// Creates a new Cesium3DTileContent.
    pub fn new() -> Self {
        Self {
            features_length: 0,
            points_length: 0,
            triangles: 0,
            byte_length: 0,
            state: Cesium3DTileContentState::Unloaded,
            bounding_volume: None,
            uri: None,
            feature_properties_dirty: false,
            tileset_url: None,
        }
    }

    /// Returns whether the content is ready.
    pub fn is_ready(&self) -> bool {
        self.state == Cesium3DTileContentState::Ready
    }

    /// Returns whether the content has a feature at the given index.
    pub fn has_feature(&self, feature_index: i32) -> bool {
        feature_index >= 0 && feature_index < self.features_length
    }

    /// Gets the position of a feature, if available.
    pub fn feature_position(&self, _feature_index: i32) -> Option<Cartesian3> {
        // DEVIATION: Requires batch table / feature table data
        None
    }
}

impl Default for Cesium3DTileContent {
    fn default() -> Self { Self::new() }
}
