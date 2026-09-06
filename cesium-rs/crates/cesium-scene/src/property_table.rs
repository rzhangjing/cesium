//! Ported from `packages/engine/Source/Scene/PropertyTable.js`.
//!
//! A property table for EXT_structural_metadata / EXT_feature_metadata,
//! with compatibility for the legacy 3D Tiles 1.0 batch table.

use serde_json::Value;

use crate::batch_table_hierarchy::BatchTableHierarchy;
use crate::json_metadata_table::JsonMetadataTable;
use crate::metadata_table::MetadataTable;

/// A property table for structured metadata.
///
/// Mirrors CesiumJS `PropertyTable` (580 lines):
/// - `name`: human-readable name
/// - `id`: unique identifier (index or key)
/// - `count`: number of features
/// - `metadata_table`: binary metadata (EXT_structural_metadata)
/// - `json_metadata_table`: JSON metadata (legacy batch table)
/// - `batch_table_hierarchy`: hierarchy (3DTILES_batch_table_hierarchy)
/// - `extras` / `extensions`: user-defined / extension data
///
/// Property resolution order (mirrors CesiumJS):
/// 1. Binary properties from `metadata_table`
/// 2. JSON properties from `json_metadata_table`
/// 3. Hierarchy properties from `batch_table_hierarchy`
#[derive(Debug, Clone)]
pub struct PropertyTable {
    /// Human-readable name.
    pub name: String,
    /// Unique identifier (array index for EXT_structural_metadata,
    /// dictionary key for EXT_feature_metadata).
    pub id: Value,
    /// The number of features in the table.
    pub count: usize,
    /// Binary metadata table.
    pub metadata_table: Option<MetadataTable>,
    /// JSON metadata table (legacy compatibility).
    pub json_metadata_table: Option<JsonMetadataTable>,
    /// Batch table hierarchy.
    pub batch_table_hierarchy: Option<BatchTableHierarchy>,
    /// Extra user-defined properties.
    pub extras: Option<Value>,
    /// Extension data.
    pub extensions: Option<Value>,
}

impl PropertyTable {
    /// Creates a new `PropertyTable`.
    pub fn new(count: usize) -> Self {
        Self {
            name: String::new(),
            id: Value::Null,
            count,
            metadata_table: None,
            json_metadata_table: None,
            batch_table_hierarchy: None,
            extras: None,
            extensions: None,
        }
    }

    /// Returns the number of features.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Gets a property value for a feature at the given index.
    pub fn get_property(&self, index: usize, property_id: &str) -> Option<Value> {
        // 1. Binary metadata table.
        if let Some(ref mt) = self.metadata_table {
            if let Some(prop) = mt.get_property(property_id) {
                if prop.has_buffer_data() {
                    // DEVIATION: full binary decoding not yet implemented.
                    return None;
                }
            }
        }

        // 2. JSON metadata table.
        if let Some(ref jmt) = self.json_metadata_table {
            if let Some(val) = jmt.get_property(property_id, index) {
                return Some(val.clone());
            }
        }

        // 3. Batch table hierarchy.
        // DEVIATION: hierarchy property lookup requires full class resolution.
        None
    }

    /// Whether the table has a specific property.
    pub fn has_property(&self, property_id: &str) -> bool {
        if let Some(ref mt) = self.metadata_table {
            if mt.has_property(property_id) {
                return true;
            }
        }
        if let Some(ref jmt) = self.json_metadata_table {
            if jmt.has_property(property_id) {
                return true;
            }
        }
        false
    }

    /// Returns the list of available property IDs across all sources.
    pub fn property_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        if let Some(ref mt) = self.metadata_table {
            for id in mt.property_ids() {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
        }
        if let Some(ref jmt) = self.json_metadata_table {
            for id in jmt.property_ids() {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
        }
        ids
    }
}

impl Default for PropertyTable {
    fn default() -> Self {
        Self::new(0)
    }
}
