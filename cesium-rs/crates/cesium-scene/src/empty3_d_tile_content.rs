//! Ported from `packages/engine/Source/Scene/Empty3DTileContent.js`.
//!
//! Represents empty content for tiles in a 3D Tiles tileset that do not
//! have content, e.g., because they are used to optimize hierarchical
//! culling. Implements the `Cesium3DTileContent` interface.

/// Empty 3D tile content (JS `Empty3DTileContent`).
///
/// DEVIATION: the JS constructor stores live `tileset` / `tile` references
/// exposed through getters; the Rust port cannot hold owning references
/// inside the content value, so the two getters are omitted and callers
/// keep the tile/tileset association (both are otherwise unused by the
/// interface methods, which all return constants).
#[derive(Debug, Default)]
pub struct Empty3DTileContent {
    /// JS `featurePropertiesDirty`, initialized to `false`.
    pub feature_properties_dirty: bool,
    destroyed: bool,
}

impl Empty3DTileContent {
    /// Creates a new empty content (JS constructor).
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of features (JS `featuresLength`, always `0`).
    pub fn features_length(&self) -> usize {
        0
    }

    /// Number of points (JS `pointsLength`, always `0`).
    pub fn points_length(&self) -> usize {
        0
    }

    /// Number of triangles (JS `trianglesLength`, always `0`).
    pub fn triangles_length(&self) -> usize {
        0
    }

    /// Geometry byte length (JS `geometryByteLength`, always `0`).
    pub fn geometry_byte_length(&self) -> usize {
        0
    }

    /// Textures byte length (JS `texturesByteLength`, always `0`).
    pub fn textures_byte_length(&self) -> usize {
        0
    }

    /// Batch table byte length (JS `batchTableByteLength`, always `0`).
    pub fn batch_table_byte_length(&self) -> usize {
        0
    }

    /// Inner contents (JS `innerContents`, always `undefined`).
    pub fn inner_contents(&self) -> Option<()> {
        None
    }

    /// Whether the content is ready to render (JS `ready`, always `true`).
    pub fn ready(&self) -> bool {
        true
    }

    /// The content url (JS `url`, always `undefined`).
    pub fn url(&self) -> Option<String> {
        None
    }

    /// Whether a feature property exists (JS `hasProperty`, always `false`).
    pub fn has_property(&self, _batch_id: usize, _name: &str) -> bool {
        false
    }

    /// Gets a feature (JS `getFeature`, always `undefined`).
    pub fn get_feature(&self, _batch_id: usize) -> Option<()> {
        None
    }

    /// JS `isDestroyed`.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// JS `destroy` (`destroyObject` semantics).
    pub fn destroy(&mut self) {
        self.destroyed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_content_reports_zeroes_and_ready() {
        let content = Empty3DTileContent::new();
        assert_eq!(content.features_length(), 0);
        assert_eq!(content.points_length(), 0);
        assert_eq!(content.triangles_length(), 0);
        assert_eq!(content.geometry_byte_length(), 0);
        assert_eq!(content.textures_byte_length(), 0);
        assert_eq!(content.batch_table_byte_length(), 0);
        assert!(content.inner_contents().is_none());
        assert!(content.ready());
        assert!(content.url().is_none());
        assert!(!content.feature_properties_dirty);
    }

    #[test]
    fn empty_content_has_no_features_and_lifecycle() {
        let mut content = Empty3DTileContent::new();
        assert!(!content.has_property(0, "Height"));
        assert!(content.get_feature(0).is_none());
        assert!(!content.is_destroyed());
        content.destroy();
        assert!(content.is_destroyed());
    }
}
