//! Ported from `packages/engine/Source/Scene/BatchTable.js`.
//!
//! A batch table for per-feature metadata in 3D Tiles.

use serde_json::Value;

use crate::batch_table_hierarchy::BatchTableHierarchy;
use crate::json_metadata_table::JsonMetadataTable;
use crate::metadata_table::MetadataTable;

/// A batch table for per-feature metadata in 3D Tiles.
///
/// Mirrors CesiumJS `BatchTable` (~500 lines):
/// - `features_length`: number of features
/// - `metadata_table`: binary metadata (EXT_structural_metadata)
/// - `json_metadata_table`: JSON metadata (legacy batch table)
/// - `batch_table_hierarchy`: hierarchy (3DTILES_batch_table_hierarchy)
/// - `extras` / `extensions`: user-defined / extension data
#[derive(Debug, Clone)]
pub struct BatchTable {
    /// The number of features in the batch.
    pub features_length: usize,
    /// Binary metadata table (EXT_structural_metadata).
    pub metadata_table: Option<MetadataTable>,
    /// JSON metadata table (legacy batch table compatibility).
    pub json_metadata_table: Option<JsonMetadataTable>,
    /// Batch table hierarchy (3DTILES_batch_table_hierarchy extension).
    pub batch_table_hierarchy: Option<BatchTableHierarchy>,
    /// Extra user-defined properties.
    pub extras: Option<Value>,
    /// Extension data.
    pub extensions: Option<Value>,
}

impl BatchTable {
    /// Creates a new `BatchTable` with the given feature count.
    pub fn new(features_length: usize) -> Self {
        Self {
            features_length,
            metadata_table: None,
            json_metadata_table: None,
            batch_table_hierarchy: None,
            extras: None,
            extensions: None,
        }
    }

    /// Returns the number of features.
    pub fn features_length(&self) -> usize {
        self.features_length
    }

    /// Gets a property value for a feature at the given batch ID.
    ///
    /// Resolution order mirrors CesiumJS:
    /// 1. Binary properties from `metadata_table`
    /// 2. JSON properties from `json_metadata_table`
    /// 3. Hierarchy properties from `batch_table_hierarchy`
    pub fn get_property(&self, batch_id: usize, name: &str) -> Option<Value> {
        // 1. Binary metadata table.
        if let Some(ref mt) = self.metadata_table {
            if let Some(prop) = mt.get_property(name) {
                if prop.has_buffer_data() {
                    // DEVIATION: full binary decoding not yet implemented.
                    return None;
                }
            }
        }

        // 2. JSON metadata table.
        if let Some(ref jmt) = self.json_metadata_table {
            if let Some(val) = jmt.get_property(name, batch_id) {
                return Some(val.clone());
            }
        }

        // 3. Batch table hierarchy.
        // DEVIATION: hierarchy property lookup requires full class resolution.
        None
    }

    /// Whether the batch table has a specific property for any feature.
    pub fn has_property(&self, name: &str) -> bool {
        if let Some(ref mt) = self.metadata_table {
            if mt.has_property(name) {
                return true;
            }
        }
        if let Some(ref jmt) = self.json_metadata_table {
            if jmt.has_property(name) {
                return true;
            }
        }
        false
    }
}

impl Default for BatchTable {
    fn default() -> Self {
        Self::new(0)
    }
}
