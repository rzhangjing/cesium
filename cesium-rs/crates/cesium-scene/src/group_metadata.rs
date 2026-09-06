//! Ported from `packages/engine/Source/Scene/GroupMetadata.js`.

use serde_json::Value;

/// Metadata about a group of `Cesium3DTileContent`.
#[derive(Debug, Clone)]
pub struct GroupMetadata {
    /// The ID of the group.
    id: String,
    /// The class that properties conform to.
    class: Value,
    /// The group properties.
    properties: Value,
    /// Extra user-defined properties.
    extras: Option<Value>,
    /// An object containing extensions.
    extensions: Option<Value>,
}

impl GroupMetadata {
    /// Creates a new `GroupMetadata`.
    pub fn new(id: String, group: &Value, class: Value) -> Self {
        let properties = group
            .get("properties")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));
        Self {
            id,
            class,
            properties,
            extras: group.get("extras").cloned(),
            extensions: group.get("extensions").cloned(),
        }
    }

    /// The class that properties conform to.
    pub fn class(&self) -> &Value {
        &self.class
    }

    /// The ID of the group.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Extra user-defined properties.
    pub fn extras(&self) -> Option<&Value> {
        self.extras.as_ref()
    }

    /// An object containing extensions.
    pub fn extensions(&self) -> Option<&Value> {
        self.extensions.as_ref()
    }

    /// Returns whether the group has this property.
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
