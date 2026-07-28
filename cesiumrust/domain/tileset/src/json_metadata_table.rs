//! JsonMetadataTable - JSON-based metadata table for 3D Tiles.
//!
//! Maps to CesiumJS `Scene/JsonMetadataTable.js`

use serde_json::Value;
use std::collections::HashMap;

/// A metadata table backed by JSON values.
/// Maps to CesiumJS `Scene/JsonMetadataTable.js`
#[derive(Debug, Clone)]
pub struct JsonMetadataTable {
    count: usize,
    properties: HashMap<String, Vec<Value>>,
}

impl JsonMetadataTable {
    /// Creates a new JsonMetadataTable.
    ///
    /// # Arguments
    /// * `count` - The number of features in the table.
    /// * `properties` - A map of property IDs to arrays of values.
    pub fn new(count: usize, properties: HashMap<String, Vec<Value>>) -> Self {
        Self { count, properties }
    }

    /// Returns the number of features in the table.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns true if the table has the given property.
    pub fn has_property(&self, property_id: &str) -> bool {
        self.properties.contains_key(property_id)
    }

    /// Returns a sorted list of property IDs.
    pub fn get_property_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.properties.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Gets the value of a property at the given index.
    /// Returns None if the property doesn't exist or index is out of bounds.
    pub fn get_property(&self, index: usize, property_id: &str) -> Option<Value> {
        if index >= self.count {
            return None;
        }
        let values = self.properties.get(property_id)?;
        values.get(index).cloned()
    }

    /// Sets the value of a property at the given index.
    /// Creates the property if it doesn't exist.
    pub fn set_property(&mut self, index: usize, property_id: &str, value: Value) {
        if index >= self.count {
            return;
        }
        let values = self
            .properties
            .entry(property_id.to_string())
            .or_insert_with(|| vec![Value::Null; self.count]);
        if index < values.len() {
            values[index] = value;
        }
    }
}
