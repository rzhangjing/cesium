//! Ported from `packages/engine/Source/Scene/MetadataClassProperty.js`.

use serde_json::Value;

use crate::metadata_component_type::MetadataComponentType;
use crate::metadata_type::MetadataType;

/// A metadata property, as part of a [`MetadataClass`](crate::metadata_class::MetadataClass).
#[derive(Debug, Clone)]
pub struct MetadataClassProperty {
    id: String,
    name: Option<String>,
    description: Option<String>,
    semantic: Option<String>,
    property_type: MetadataType,
    component_type: Option<MetadataComponentType>,
    is_array: bool,
    is_variable_length_array: bool,
    array_length: Option<usize>,
    normalized: bool,
    min: Option<Value>,
    max: Option<Value>,
    offset: Option<Value>,
    scale: Option<Value>,
    no_data: Option<Value>,
    default: Option<Value>,
    required: bool,
    extras: Option<Value>,
    extensions: Option<Value>,
}

impl MetadataClassProperty {
    /// Creates a `MetadataClassProperty` from a JSON object.
    pub fn from_json(
        id: &str,
        property: &Value,
        _enums: Option<&serde_json::Map<String, Value>>,
    ) -> Option<Self> {
        let obj = property.as_object()?;
        let type_str = obj.get("type")?.as_str()?;
        let property_type = MetadataType::from_str(type_str).unwrap_or(MetadataType::Scalar);

        let component_type = obj
            .get("componentType")
            .and_then(|v| v.as_str())
            .and_then(MetadataComponentType::from_str);

        let is_array = obj
            .get("array")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let array_length = obj.get("count").and_then(|v| v.as_u64()).map(|v| v as usize);
        let is_variable_length_array = is_array && array_length.is_none();

        let normalized = component_type
            .as_ref()
            .map(|ct| {
                ct.is_integer_type()
                    && obj.get("normalized")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
            })
            .unwrap_or(false);

        let required = obj
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Some(Self {
            id: id.to_string(),
            name: obj.get("name").and_then(|v| v.as_str()).map(String::from),
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            semantic: obj
                .get("semantic")
                .and_then(|v| v.as_str())
                .map(String::from),
            property_type,
            component_type,
            is_array,
            is_variable_length_array,
            array_length,
            normalized,
            min: obj.get("min").cloned(),
            max: obj.get("max").cloned(),
            offset: obj.get("offset").cloned(),
            scale: obj.get("scale").cloned(),
            no_data: obj.get("noData").cloned(),
            default: obj.get("default").cloned(),
            required,
            extras: obj.get("extras").cloned(),
            extensions: obj.get("extensions").cloned(),
        })
    }

    /// The ID of the property.
    pub fn id(&self) -> &str { &self.id }
    /// The name of the property.
    pub fn name(&self) -> Option<&str> { self.name.as_deref() }
    /// The description of the property.
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }
    /// An identifier that describes how this property should be interpreted.
    pub fn semantic(&self) -> Option<&str> { self.semantic.as_deref() }
    /// The type of the property (SCALAR, VEC2, etc.).
    pub fn property_type(&self) -> MetadataType { self.property_type }
    /// The component type (INT8, FLOAT32, etc.).
    pub fn component_type(&self) -> Option<MetadataComponentType> { self.component_type }
    /// Whether this property is an array.
    pub fn is_array(&self) -> bool { self.is_array }
    /// Whether this property is a variable-length array.
    pub fn is_variable_length_array(&self) -> bool { self.is_variable_length_array }
    /// The array length (for fixed-size arrays).
    pub fn array_length(&self) -> Option<usize> { self.array_length }
    /// Whether the property is normalized.
    pub fn normalized(&self) -> bool { self.normalized }
    /// Whether the property is required.
    pub fn required(&self) -> bool { self.required }
    /// Extra user-defined properties.
    pub fn extras(&self) -> Option<&Value> { self.extras.as_ref() }
    /// Extensions.
    pub fn extensions(&self) -> Option<&Value> { self.extensions.as_ref() }

    /// Returns the byte size of a single property element on the CPU.
    pub fn cpu_bytes_per_element(&self) -> usize {
        let component_count = self.property_type.component_count();
        let array_length = if self.is_array {
            self.array_length.unwrap_or(1)
        } else {
            1
        };
        let bytes_per_component = self
            .component_type
            .map(|ct| ct.size_in_bytes())
            .unwrap_or(4);
        component_count * array_length * bytes_per_component
    }
}
