//! Ported from `packages/engine/Source/Scene/MetadataEnumValue.js`.

/// A metadata enum value.
///
/// See the [3D Metadata Specification](https://github.com/CesiumGS/3d-tiles/tree/main/specification/Metadata)
/// for 3D Tiles.
#[derive(Debug, Clone)]
pub struct MetadataEnumValue {
    value: i32,
    name: String,
    description: Option<String>,
    extras: Option<serde_json::Value>,
    extensions: Option<serde_json::Value>,
}

impl MetadataEnumValue {
    /// Creates a new `MetadataEnumValue`.
    pub fn new(
        value: i32,
        name: String,
        description: Option<String>,
        extras: Option<serde_json::Value>,
        extensions: Option<serde_json::Value>,
    ) -> Self {
        Self {
            value,
            name,
            description,
            extras,
            extensions,
        }
    }

    /// Creates a `MetadataEnumValue` from a JSON object.
    pub fn from_json(json: &serde_json::Value) -> Option<Self> {
        let obj = json.as_object()?;
        Some(Self {
            value: obj.get("value")?.as_i64()? as i32,
            name: obj.get("name")?.as_str()?.to_string(),
            description: obj.get("description").and_then(|v| v.as_str()).map(String::from),
            extras: obj.get("extras").cloned(),
            extensions: obj.get("extensions").cloned(),
        })
    }

    /// The integer value.
    pub fn value(&self) -> i32 {
        self.value
    }

    /// The name of the enum value.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The description of the enum value.
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
