//! Ported from `packages/engine/Source/Scene/MetadataEnum.js`.

use std::collections::HashMap;

use crate::metadata_component_type::MetadataComponentType;
use crate::metadata_enum_value::MetadataEnumValue;

/// A metadata enum.
///
/// See the [3D Metadata Specification](https://github.com/CesiumGS/3d-tiles/tree/main/specification/Metadata)
/// for 3D Tiles.
#[derive(Debug, Clone)]
pub struct MetadataEnum {
    id: String,
    values: Vec<MetadataEnumValue>,
    names_by_value: HashMap<i32, String>,
    values_by_name: HashMap<String, i32>,
    value_type: MetadataComponentType,
    name: Option<String>,
    description: Option<String>,
    extras: Option<serde_json::Value>,
    extensions: Option<serde_json::Value>,
}

impl MetadataEnum {
    /// Creates a new `MetadataEnum`.
    pub fn new(
        id: String,
        values: Vec<MetadataEnumValue>,
        value_type: Option<MetadataComponentType>,
        name: Option<String>,
        description: Option<String>,
        extras: Option<serde_json::Value>,
        extensions: Option<serde_json::Value>,
    ) -> Self {
        let mut names_by_value = HashMap::new();
        let mut values_by_name = HashMap::new();
        for v in &values {
            names_by_value.insert(v.value(), v.name().to_string());
            values_by_name.insert(v.name().to_string(), v.value());
        }

        Self {
            id,
            values,
            names_by_value,
            values_by_name,
            value_type: value_type.unwrap_or(MetadataComponentType::Uint16),
            name,
            description,
            extras,
            extensions,
        }
    }

    /// Creates a `MetadataEnum` from a JSON object.
    ///
    /// Corresponds to `MetadataEnum.fromJson` in the JS API.
    pub fn from_json(id: &str, json: &serde_json::Value) -> Option<Self> {
        let obj = json.as_object()?;
        let values_arr = obj.get("values")?.as_array()?;

        let values: Vec<MetadataEnumValue> = values_arr
            .iter()
            .filter_map(|v| MetadataEnumValue::from_json(v))
            .collect();

        let value_type = obj
            .get("valueType")
            .and_then(|v| v.as_str())
            .and_then(MetadataComponentType::from_str);

        Some(Self::new(
            id.to_string(),
            values,
            value_type,
            obj.get("name").and_then(|v| v.as_str()).map(String::from),
            obj.get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            obj.get("extras").cloned(),
            obj.get("extensions").cloned(),
        ))
    }

    /// The enum values.
    pub fn values(&self) -> &[MetadataEnumValue] {
        &self.values
    }

    /// A dictionary mapping enum integer values to names.
    pub fn names_by_value(&self) -> &HashMap<i32, String> {
        &self.names_by_value
    }

    /// A dictionary mapping enum names to integer values.
    pub fn values_by_name(&self) -> &HashMap<String, i32> {
        &self.values_by_name
    }

    /// The enum value type.
    pub fn value_type(&self) -> MetadataComponentType {
        self.value_type
    }

    /// The ID of the enum.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The name of the enum.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The description of the enum.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Extra user-defined properties.
    pub fn extras(&self) -> Option<&serde_json::Value> {
        self.extras.as_ref()
    }

    /// An object containing extensions.
    pub fn extensions(&self) -> Option<&serde_json::Value> {
        self.extensions.as_ref()
    }
}
