//! Ported from `packages/engine/Source/Scene/TilesetMetadata.js`.
//!
//! Tileset-level metadata conforming to EXT_structural_metadata.

use serde_json::Value;

/// Metadata for a tileset.
///
/// Stores the metadata class, property dictionary, and optional extras/extensions.
/// Property access follows the EXT_structural_metadata convention:
/// - `has_property` / `get_property` by property ID
/// - `has_property_by_semantic` / `get_property_by_semantic` by semantic name
///
/// Mirrors CesiumJS `TilesetMetadata` (~120 lines).
pub struct TilesetMetadata {
    /// The metadata class name this metadata conforms to.
    pub class_name: Option<String>,
    /// The property dictionary (property_id → value).
    pub properties: std::collections::HashMap<String, Value>,
    /// User-defined extra data.
    pub extras: Option<Value>,
    /// Extension data.
    pub extensions: Option<Value>,
}

impl TilesetMetadata {
    /// Creates a new `TilesetMetadata`.
    pub fn new() -> Self {
        Self {
            class_name: None,
            properties: std::collections::HashMap::new(),
            extras: None,
            extensions: None,
        }
    }

    /// Returns whether a property with the given ID exists.
    pub fn has_property(&self, property_id: &str) -> bool {
        self.properties.contains_key(property_id)
    }

    /// Gets a property value by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&Value> {
        self.properties.get(property_id)
    }

    /// Sets a property value by ID.
    pub fn set_property(&mut self, property_id: &str, value: Value) {
        self.properties.insert(property_id.to_string(), value);
    }

    /// Returns all property IDs.
    pub fn get_property_ids(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// Returns whether a property with the given semantic exists.
    ///
    /// Searches property values for a matching `"semantic"` field.
    pub fn has_property_by_semantic(&self, semantic: &str) -> bool {
        self.properties.values().any(|v| {
            v.get("semantic")
                .and_then(|s| s.as_str())
                .map_or(false, |s| s == semantic)
        })
    }

    /// Gets a property value by semantic name.
    pub fn get_property_by_semantic(&self, semantic: &str) -> Option<&Value> {
        self.properties.values().find(|v| {
            v.get("semantic")
                .and_then(|s| s.as_str())
                .map_or(false, |s| s == semantic)
        })
    }
}

impl Default for TilesetMetadata {
    fn default() -> Self { Self::new() }
}
