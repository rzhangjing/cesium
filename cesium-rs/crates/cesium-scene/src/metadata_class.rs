//! Ported from `packages/engine/Source/Scene/MetadataClass.js`.

use std::collections::HashMap;

use serde_json::Value;

use crate::metadata_class_property::MetadataClassProperty;

/// A metadata class.
///
/// See the [3D Metadata Specification](https://github.com/CesiumGS/3d-tiles/tree/main/specification/Metadata)
/// for 3D Tiles.
#[derive(Debug, Clone)]
pub struct MetadataClass {
    id: String,
    name: Option<String>,
    description: Option<String>,
    properties: HashMap<String, MetadataClassProperty>,
    properties_by_semantic: HashMap<String, MetadataClassProperty>,
    extras: Option<Value>,
    extensions: Option<Value>,
}

/// The class name given to the metadata class when a batch table is loaded
/// from 3D Tiles 1.0 formats.
pub const BATCH_TABLE_CLASS_NAME: &str = "_batchTable";

impl MetadataClass {
    /// Creates a new `MetadataClass`.
    pub fn new(
        id: String,
        name: Option<String>,
        description: Option<String>,
        properties: HashMap<String, MetadataClassProperty>,
        extras: Option<Value>,
        extensions: Option<Value>,
    ) -> Self {
        // Build semantic index
        let mut properties_by_semantic = HashMap::new();
        for (pid, prop) in &properties {
            if let Some(semantic) = prop.semantic() {
                properties_by_semantic.insert(semantic.to_string(), prop.clone());
            }
            let _ = pid; // suppress unused warning
        }

        Self {
            id,
            name,
            description,
            properties,
            properties_by_semantic,
            extras,
            extensions,
        }
    }

    /// Creates a `MetadataClass` from a JSON object.
    pub fn from_json(
        id: &str,
        class: &Value,
        enums: Option<&serde_json::Map<String, Value>>,
    ) -> Option<Self> {
        let obj = class.as_object()?;
        let properties_json = obj.get("properties")?.as_object()?;

        let mut properties = HashMap::new();
        for (pid, pval) in properties_json {
            let prop = MetadataClassProperty::from_json(pid, pval, enums)?;
            properties.insert(pid.clone(), prop);
        }

        Some(Self::new(
            id.to_string(),
            obj.get("name").and_then(|v| v.as_str()).map(String::from),
            obj.get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            properties,
            obj.get("extras").cloned(),
            obj.get("extensions").cloned(),
        ))
    }

    /// The ID of the class.
    pub fn id(&self) -> &str { &self.id }
    /// The name of the class.
    pub fn name(&self) -> Option<&str> { self.name.as_deref() }
    /// The description of the class.
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }
    /// The class properties.
    pub fn properties(&self) -> &HashMap<String, MetadataClassProperty> { &self.properties }
    /// Properties indexed by semantic.
    pub fn properties_by_semantic(&self) -> &HashMap<String, MetadataClassProperty> {
        &self.properties_by_semantic
    }
    /// Extra user-defined properties.
    pub fn extras(&self) -> Option<&Value> { self.extras.as_ref() }
    /// Extensions.
    pub fn extensions(&self) -> Option<&Value> { self.extensions.as_ref() }
}
