//! Ported from `packages/engine/Source/Scene/MetadataTable.js`.
//!
//! A table containing binary metadata for a collection of entities.
//! Used for batch table binary properties and 3DTILES_metadata.

use std::collections::HashMap;

use crate::metadata_table_property::MetadataTableProperty;

/// A table containing binary metadata for a collection of entities.
///
/// Mirrors CesiumJS `MetadataTable` (372 lines):
/// - `count`: number of entities in the table
/// - `class_name`: the metadata class these entities conform to
/// - `properties`: map of property ID → MetadataTableProperty
/// - `buffer_views`: opaque buffer view storage (DEVIATION: JS uses Uint8Array map)
#[derive(Debug, Clone)]
pub struct MetadataTable {
    /// The number of entities in the table.
    pub count: usize,
    /// The metadata class name these entities conform to.
    pub class_name: String,
    /// Map of property ID → property definition.
    pub properties: HashMap<String, MetadataTableProperty>,
    /// Buffer view storage: index → raw bytes.
    /// DEVIATION: CesiumJS uses `Uint8Array` values; Rust uses `Vec<u8>`.
    pub buffer_views: HashMap<usize, Vec<u8>>,
}

impl MetadataTable {
    /// Creates a new `MetadataTable`.
    pub fn new(count: usize, class_name: &str) -> Self {
        Self {
            count,
            class_name: class_name.to_string(),
            properties: HashMap::new(),
            buffer_views: HashMap::new(),
        }
    }

    /// Returns the number of entities.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Gets a property definition by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&MetadataTableProperty> {
        self.properties.get(property_id)
    }

    /// Returns the list of property IDs.
    pub fn property_ids(&self) -> Vec<&String> {
        self.properties.keys().collect()
    }

    /// Whether the table has a specific property.
    pub fn has_property(&self, property_id: &str) -> bool {
        self.properties.contains_key(property_id)
    }

    /// Returns the number of properties.
    pub fn properties_length(&self) -> usize {
        self.properties.len()
    }
}

impl Default for MetadataTable {
    fn default() -> Self {
        Self::new(0, "")
    }
}
