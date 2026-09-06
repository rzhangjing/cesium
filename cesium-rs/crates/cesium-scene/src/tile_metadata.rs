//! Ported from `packages/engine/Source/Scene/TileMetadata.js`.

use serde_json::Value;

/// Metadata about a 3D Tile. This represents the tile metadata JSON
/// (3D Tiles 1.1) or the `3DTILES_metadata` extension on a single
/// `Cesium3DTile`.
#[derive(Debug, Clone)]
pub struct TileMetadata {
    /// The class that properties conform to.
    class: Value,
    /// The tile properties.
    properties: Value,
    /// Extra user-defined properties.
    extras: Option<Value>,
    /// An object containing extensions.
    extensions: Option<Value>,
}

impl TileMetadata {
    /// Creates a new `TileMetadata`.
    pub fn new(tile: &Value, class: Value) -> Self {
        Self {
            class,
            properties: tile.get("properties").cloned().unwrap_or(Value::Object(Default::default())),
            extras: tile.get("extras").cloned(),
            extensions: tile.get("extensions").cloned(),
        }
    }

    /// The class that properties conform to.
    pub fn class(&self) -> &Value {
        &self.class
    }

    /// Extra user-defined properties.
    pub fn extras(&self) -> Option<&Value> {
        self.extras.as_ref()
    }

    /// An object containing extensions.
    pub fn extensions(&self) -> Option<&Value> {
        self.extensions.as_ref()
    }

    /// Returns whether the tile has this property.
    pub fn has_property(&self, property_id: &str) -> bool {
        self.properties.get(property_id).is_some()
    }

    /// Returns a copy of the value of the property with the given ID.
    pub fn get_property(&self, property_id: &str) -> Option<&Value> {
        self.properties.get(property_id)
    }

    /// Returns an array of property IDs.
    pub fn get_property_ids(&self) -> Vec<String> {
        match &self.properties {
            Value::Object(map) => map.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }
}
