//! Ported from `packages/engine/Source/Scene/JsonMetadataTable.js`.
//!
//! A metadata table backed by free-form JSON properties, used for
//! compatibility with the legacy 3D Tiles 1.0 batch table.

use std::collections::HashMap;

use serde_json::Value;

/// A metadata table backed by free-form JSON properties.
///
/// Mirrors CesiumJS `JsonMetadataTable`:
/// - `count`: number of entities
/// - `properties`: map of property name → JSON array (one value per entity)
///
/// Used for compatibility with the old batch table where properties are
/// stored as JSON arrays rather than binary buffer views.
#[derive(Debug, Clone)]
pub struct JsonMetadataTable {
    /// The number of entities in the table.
    pub count: usize,
    /// Map of property name → JSON array of values (one per entity).
    pub properties: HashMap<String, Value>,
}

impl JsonMetadataTable {
    /// Creates a new `JsonMetadataTable`.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            properties: HashMap::new(),
        }
    }

    /// Returns the number of entities.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Gets the JSON value for a property at the given entity index.
    pub fn get_property(&self, property_id: &str, index: usize) -> Option<&Value> {
        self.properties.get(property_id).and_then(|arr| arr.get(index))
    }

    /// Gets the full JSON array for a property.
    pub fn get_property_array(&self, property_id: &str) -> Option<&Value> {
        self.properties.get(property_id)
    }

    /// Whether the table has a specific property.
    pub fn has_property(&self, property_id: &str) -> bool {
        self.properties.contains_key(property_id)
    }

    /// Returns the list of property names.
    pub fn property_ids(&self) -> Vec<&String> {
        self.properties.keys().collect()
    }

    /// Returns the number of properties.
    pub fn properties_length(&self) -> usize {
        self.properties.len()
    }
}

impl Default for JsonMetadataTable {
    fn default() -> Self {
        Self::new(0)
    }
}
