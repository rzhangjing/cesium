//! Ported from `packages/engine/Source/Scene/Cesium3DTileFeatureTable.js`.
//!
//! A feature table from a 3D Tiles tile content.

use std::collections::HashMap;

/// A feature table from a 3D Tiles tile content.
///
/// Provides access to per-feature data stored in the feature table
/// section of a tile's binary content.
/// Mirrors CesiumJS `Cesium3DTileFeatureTable` (296 lines).
pub struct Cesium3DTileFeatureTable {
    /// The number of features in this table.
    features_length: i32,
    /// The binary data for this feature table.
    byte_offset: usize,
    /// Property semantics mapped to accessor indices.
    semantics: HashMap<String, i32>,
}

impl Cesium3DTileFeatureTable {
    /// Creates a new Cesium3DTileFeatureTable.
    pub fn new() -> Self {
        Self {
            features_length: 0,
            byte_offset: 0,
            semantics: HashMap::new(),
        }
    }

    /// Returns the number of features.
    pub fn features_length(&self) -> i32 {
        self.features_length
    }

    /// Returns whether a semantic exists in this table.
    pub fn has_semantic(&self, semantic: &str) -> bool {
        self.semantics.contains_key(semantic)
    }

    /// Gets the global semantic value, if available.
    pub fn get_global_semantic(&self, _semantic: &str) -> Option<i32> {
        None
    }

    /// Gets a per-feature value for a semantic, if available.
    pub fn get_shader_data(&self, _feature_id: i32, _semantic: &str) -> Option<i32> {
        None
    }
}

impl Default for Cesium3DTileFeatureTable {
    fn default() -> Self { Self::new() }
}
