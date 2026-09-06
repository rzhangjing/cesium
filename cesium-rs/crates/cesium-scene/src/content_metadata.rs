//! Ported from `packages/engine/Source/Scene/ContentMetadata.js`.

use serde_json::Value;

/// Metadata about the content of a 3D Tile. This represents the content
/// metadata JSON (3D Tiles 1.1) or the `3DTILES_metadata` extension on a
/// single `Cesium3DTileContent`.
#[derive(Debug, Clone)]
pub struct ContentMetadata {
    /// The class that properties conform to.
    class: Value,
    /// The content properties.
    properties: Value,
    /// Extra user-defined properties.
    extras: Option<Value>,
    /// An object containing extensions.
    extensions: Option<Value>,
}

impl ContentMetadata {
    /// Creates a new `ContentMetadata`.
    pub fn new(content: &Value, class: Value) -> Self {
        Self {
            class,
            properties: content.get("properties").cloned().unwrap_or(Value::Object(Default::default())),
            extras: content.get("extras").cloned(),
            extensions: content.get("extensions").cloned(),
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

    /// Returns whether the content has this property.
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
