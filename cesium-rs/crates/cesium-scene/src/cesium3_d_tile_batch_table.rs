//! Ported from `packages/engine/Source/Scene/Cesium3DTileBatchTable.js`.
//!
//! A batch table for per-feature properties in 3D Tiles.

use std::collections::HashMap;

/// A batch table for per-feature properties in 3D Tiles.
///
/// Stores per-feature data (batch table properties) for all features
/// across all loaded tiles in a tileset.
/// Mirrors CesiumJS `Cesium3DTileBatchTable` (830 lines).
pub struct Cesium3DTileBatchTable {
    /// The number of features in this batch table.
    features_count: i32,
    /// Property names available in the batch table.
    property_names: Vec<String>,
    /// Per-feature property values (feature_index -> property_name -> value).
    properties: HashMap<i32, HashMap<String, String>>,
}

impl Cesium3DTileBatchTable {
    /// Creates a new Cesium3DTileBatchTable.
    pub fn new() -> Self {
        Self {
            features_count: 0,
            property_names: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Returns the total number of features.
    pub fn features_count(&self) -> i32 {
        self.features_count
    }

    /// Returns the available property names.
    pub fn property_names(&self) -> &[String] {
        &self.property_names
    }

    /// Gets a property value for a feature.
    pub fn get_property(&self, feature_index: i32, name: &str) -> Option<&str> {
        self.properties
            .get(&feature_index)
            .and_then(|props| props.get(name))
            .map(|s| s.as_str())
    }

    /// Returns whether a feature has a property.
    pub fn has_property(&self, feature_index: i32, name: &str) -> bool {
        self.properties
            .get(&feature_index)
            .map_or(false, |props| props.contains_key(name))
    }
}

impl Default for Cesium3DTileBatchTable {
    fn default() -> Self { Self::new() }
}
