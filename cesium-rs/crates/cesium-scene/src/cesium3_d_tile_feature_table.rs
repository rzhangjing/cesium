//! Ported from `packages/engine/Source/Scene/Cesium3DTileFeatureTable.js`.
//!
//! Reads feature/batch table style properties from the feature table JSON
//! header plus its binary body (little-endian, mirroring the JS
//! `DataView`-based typed array creation).

use std::collections::HashMap;

use cesium_core::component_datatype::ComponentDatatype;
use serde_json::Value;

/// A value returned by [`Cesium3DTileFeatureTable::get_global_property`]
/// (JS returns either a typed array or the raw JSON value).
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureTablePropertyValue {
    /// A decoded typed array (JS `TypedArray` result).
    Array(Vec<f64>),
    /// The raw JSON value (JS non-`byteOffset` branch).
    Json(Value),
}

/// The result of [`Cesium3DTileFeatureTable::get_property`] (JS returns a
/// scalar for `componentLength === 1` or the filled `result` array).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FeatureTableProperty {
    /// A single component value (JS `typedArray[featureId]`).
    Scalar(f64),
    /// Components were written into the caller-provided `result` slice.
    Components,
}

/// A 3D Tiles feature table (JS `Cesium3DTileFeatureTable`).
///
/// DEVIATION: the JS caches `TypedArray` views over the binary buffer; the
/// Rust port caches decoded `Vec<f64>` values and returns owned copies
/// (same observable values, identical caching of decode work).
pub struct Cesium3DTileFeatureTable {
    json: Value,
    buffer: Vec<u8>,
    buffer_byte_offset: usize,
    cached_typed_arrays: HashMap<String, Vec<f64>>,
    /// The number of features (JS `featuresLength`, set by the consumer).
    pub features_length: usize,
}

impl Cesium3DTileFeatureTable {
    /// Creates a new feature table.
    ///
    /// JS `Cesium3DTileFeatureTable(featureTableJson, featureTableBinary)`;
    /// `buffer_byte_offset` mirrors `featureTableBinary.byteOffset` (the JS
    /// reads from `buffer.buffer` at `buffer.byteOffset + byteOffset`).
    pub fn new(json: Value, buffer: Vec<u8>, buffer_byte_offset: usize) -> Self {
        Self {
            json,
            buffer,
            buffer_byte_offset,
            cached_typed_arrays: HashMap::new(),
            features_length: 0,
        }
    }

    /// The feature table JSON header (JS `json`).
    pub fn json(&self) -> &Value {
        &self.json
    }

    fn get_typed_array_from_binary(
        &mut self,
        semantic: &str,
        component_type: ComponentDatatype,
        component_length: usize,
        count: usize,
        byte_offset: usize,
    ) -> Vec<f64> {
        if let Some(cached) = self.cached_typed_arrays.get(semantic) {
            return cached.clone();
        }
        let typed_array = decode_typed_array(
            &self.buffer,
            self.buffer_byte_offset + byte_offset,
            component_type,
            count * component_length,
        );
        self.cached_typed_arrays
            .insert(semantic.to_string(), typed_array.clone());
        typed_array
    }

    fn get_typed_array_from_array(
        &mut self,
        semantic: &str,
        component_type: ComponentDatatype,
        array: &Value,
    ) -> Vec<f64> {
        if let Some(cached) = self.cached_typed_arrays.get(semantic) {
            return cached.clone();
        }
        // JS `ComponentDatatype.createTypedArray(componentType, array)`
        // truncates each element to the target type; the values are
        // preserved as f64 here (callers consume JS-number semantics).
        let typed_array: Vec<f64> = match array {
            Value::Array(items) => items
                .iter()
                .map(|item| truncate_to_datatype(item.as_f64().unwrap_or(0.0), component_type))
                .collect(),
            other => vec![truncate_to_datatype(other.as_f64().unwrap_or(0.0), component_type)],
        };
        self.cached_typed_arrays
            .insert(semantic.to_string(), typed_array.clone());
        typed_array
    }

    /// Gets a global property.
    ///
    /// JS `getGlobalProperty(semantic, componentType, componentLength)`.
    /// Defaults (JS `??`): `componentType` to `UNSIGNED_INT` and
    /// `componentLength` to `1` when the value has a `byteOffset`.
    pub fn get_global_property(
        &mut self,
        semantic: &str,
        component_type: Option<ComponentDatatype>,
        component_length: Option<usize>,
    ) -> Option<FeatureTablePropertyValue> {
        let json_value = self.json.get(semantic).filter(|value| !value.is_null())?;

        if let Some(byte_offset_value) = json_value.get("byteOffset") {
            let byte_offset = byte_offset_value.as_u64()? as usize;
            let component_type = component_type.unwrap_or(ComponentDatatype::UnsignedInt);
            let component_length = component_length.unwrap_or(1);
            let typed_array = self.get_typed_array_from_binary(
                semantic,
                component_type,
                component_length,
                1,
                byte_offset,
            );
            return Some(FeatureTablePropertyValue::Array(typed_array));
        }

        Some(FeatureTablePropertyValue::Json(json_value.clone()))
    }

    /// Whether a property exists in the JSON header.
    ///
    /// JS `hasProperty(semantic)`.
    pub fn has_property(&self, semantic: &str) -> bool {
        self.json.get(semantic).map_or(false, |value| !value.is_null())
    }

    /// Gets a per-feature property array.
    ///
    /// JS `getPropertyArray(semantic, componentType, componentLength)`.
    pub fn get_property_array(
        &mut self,
        semantic: &str,
        component_type: Option<ComponentDatatype>,
        component_length: usize,
    ) -> Option<Vec<f64>> {
        let json_value = self.json.get(semantic).filter(|value| !value.is_null())?.clone();

        if let Some(byte_offset_value) = json_value.get("byteOffset") {
            let byte_offset = byte_offset_value.as_u64()? as usize;
            let mut resolved_component_type = component_type;
            if let Some(name) = json_value
                .get("componentType")
                .and_then(|value| value.as_str())
            {
                resolved_component_type = ComponentDatatype::from_name(name);
            }
            let resolved_component_type = resolved_component_type?;
            return Some(self.get_typed_array_from_binary(
                semantic,
                resolved_component_type,
                component_length,
                self.features_length,
                byte_offset,
            ));
        }

        let component_type = component_type?;
        Some(self.get_typed_array_from_array(semantic, component_type, &json_value))
    }

    /// Gets a single feature's property value.
    ///
    /// JS `getProperty(semantic, componentType, componentLength, featureId,
    /// result)`. When `component_length == 1` the scalar is returned
    /// directly; otherwise the components are written into `result`.
    ///
    /// DEVIATION: the JS writes into and returns the caller's `result`
    /// array; the Rust port writes into `result` and returns
    /// `FeatureTableProperty::Components` to indicate that (the JS returns
    /// the same array reference, which carries no extra information).
    pub fn get_property(
        &mut self,
        semantic: &str,
        component_type: Option<ComponentDatatype>,
        component_length: usize,
        feature_id: usize,
        result: &mut [f64],
    ) -> Option<FeatureTableProperty> {
        if self.json.get(semantic).filter(|value| !value.is_null()).is_none() {
            return None;
        }

        let typed_array =
            self.get_property_array(semantic, component_type, component_length)?;

        if component_length == 1 {
            // JS `typedArray[featureId]` is `undefined` past the end.
            return typed_array
                .get(feature_id)
                .copied()
                .map(FeatureTableProperty::Scalar);
        }

        for i in 0..component_length {
            result[i] = *typed_array.get(component_length * feature_id + i)?;
        }

        Some(FeatureTableProperty::Components)
    }
}

/// Decodes `length` components starting at `byte_offset` as `f64` values,
/// little-endian (the Rust analogue of
/// `ComponentDatatype.createArrayBufferView`).
///
/// # Panics
/// Panics when the read runs past the end of `buffer` (the JS `DataView`
/// throws `RangeError`).
fn decode_typed_array(
    buffer: &[u8],
    byte_offset: usize,
    component_type: ComponentDatatype,
    length: usize,
) -> Vec<f64> {
    let size = component_type.size_in_bytes();
    let mut values = Vec::with_capacity(length);
    for i in 0..length {
        let offset = byte_offset + i * size;
        let bytes = &buffer[offset..offset + size];
        let value = match component_type {
            ComponentDatatype::Byte => i8::from_le_bytes([bytes[0]]) as f64,
            ComponentDatatype::UnsignedByte => bytes[0] as f64,
            ComponentDatatype::Short => i16::from_le_bytes([bytes[0], bytes[1]]) as f64,
            ComponentDatatype::UnsignedShort => u16::from_le_bytes([bytes[0], bytes[1]]) as f64,
            ComponentDatatype::Int => {
                i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
            }
            ComponentDatatype::UnsignedInt => {
                u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
            }
            ComponentDatatype::Float => {
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
            }
            ComponentDatatype::Double => f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
        };
        values.push(value);
    }
    values
}

/// Mirrors typed-array construction truncation (`ToUint8`/`ToUint16`/...)
/// for the value round-trip used by `getTypedArrayFromArray`.
fn truncate_to_datatype(value: f64, component_type: ComponentDatatype) -> f64 {
    match component_type {
        ComponentDatatype::Byte => (value.trunc() as i8) as f64,
        ComponentDatatype::UnsignedByte => (value.trunc() as u8) as f64,
        ComponentDatatype::Short => (value.trunc() as i16) as f64,
        ComponentDatatype::UnsignedShort => (value.trunc() as u16) as f64,
        ComponentDatatype::Int => (value.trunc() as i32) as f64,
        ComponentDatatype::UnsignedInt => (value.trunc() as u32) as f64,
        ComponentDatatype::Float => value as f32 as f64,
        ComponentDatatype::Double => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn feature_table(json: Value, buffer: Vec<u8>) -> Cesium3DTileFeatureTable {
        Cesium3DTileFeatureTable::new(json, buffer, 0)
    }

    #[test]
    fn has_property_checks_json_presence() {
        let table = feature_table(json!({ "POINTS_LENGTH": 4 }), Vec::new());
        assert!(table.has_property("POINTS_LENGTH"));
        assert!(!table.has_property("POSITION"));
    }

    #[test]
    fn get_global_property_returns_json_value_when_inline() {
        let mut table = feature_table(json!({ "POINTS_LENGTH": 4 }), Vec::new());
        assert_eq!(
            table.get_global_property("POINTS_LENGTH", None, None),
            Some(FeatureTablePropertyValue::Json(json!(4)))
        );
        assert_eq!(table.get_global_property("MISSING", None, None), None);
    }

    #[test]
    fn get_global_property_reads_binary_with_default_unsigned_int() {
        let mut buffer = vec![0u8; 4];
        buffer[0..4].copy_from_slice(&42u32.to_le_bytes());
        let mut table = feature_table(
            json!({ "INSTANCES_LENGTH": { "byteOffset": 0 } }),
            buffer,
        );
        assert_eq!(
            table.get_global_property("INSTANCES_LENGTH", None, None),
            Some(FeatureTablePropertyValue::Array(vec![42.0]))
        );
    }

    #[test]
    fn get_property_array_from_binary_uses_component_type_override() {
        // Two FLOAT positions per feature, three features.
        let mut buffer = Vec::new();
        for value in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            buffer.extend_from_slice(&value.to_le_bytes());
        }
        let mut table = feature_table(
            json!({ "POSITION": { "byteOffset": 0 } }),
            buffer,
        );
        table.features_length = 3;
        let values = table
            .get_property_array("POSITION", Some(ComponentDatatype::Float), 2)
            .unwrap();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn get_property_array_from_binary_reads_component_type_name() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&(-2i16).to_le_bytes());
        buffer.extend_from_slice(&7i16.to_le_bytes());
        let mut table = feature_table(
            json!({ "SCALE": { "byteOffset": 0, "componentType": "SHORT" } }),
            buffer,
        );
        table.features_length = 2;
        // Pass a deliberately wrong datatype; the JSON `componentType`
        // name takes precedence (JS `ComponentDatatype.fromName`).
        let values = table
            .get_property_array("SCALE", Some(ComponentDatatype::UnsignedInt), 1)
            .unwrap();
        assert_eq!(values, vec![-2.0, 7.0]);
    }

    #[test]
    fn get_property_array_from_inline_json_array() {
        let mut table = feature_table(json!({ "SCALE": [1.5, 2.5] }), Vec::new());
        table.features_length = 2;
        let values = table
            .get_property_array("SCALE", Some(ComponentDatatype::Float), 1)
            .unwrap();
        assert_eq!(values, vec![1.5, 2.5]);
    }

    #[test]
    fn get_property_scalar_and_components() {
        let mut table = feature_table(json!({ "HEIGHT": [10.0, 20.0] }), Vec::new());
        table.features_length = 2;
        let mut result = [0.0; 1];
        assert_eq!(
            table.get_property("HEIGHT", Some(ComponentDatatype::Float), 1, 1, &mut result),
            Some(FeatureTableProperty::Scalar(20.0))
        );

        let mut table = feature_table(json!({ "POSITION": [1.0, 2.0, 3.0, 4.0] }), Vec::new());
        table.features_length = 2;
        let mut result = [0.0; 2];
        assert_eq!(
            table.get_property("POSITION", Some(ComponentDatatype::Float), 2, 1, &mut result),
            Some(FeatureTableProperty::Components)
        );
        assert_eq!(result, [3.0, 4.0]);

        let mut result = [0.0; 1];
        assert_eq!(
            table.get_property("MISSING", Some(ComponentDatatype::Float), 1, 0, &mut result),
            None
        );
    }

    #[test]
    fn decode_handles_signed_and_unsigned_types() {
        let mut buffer = Vec::new();
        buffer.push(0xFE); // -2 as i8
        buffer.push(0xFF); // 255 as u8
        let decoded = decode_typed_array(&buffer, 0, ComponentDatatype::Byte, 1);
        assert_eq!(decoded, vec![-2.0]);
        let decoded = decode_typed_array(&buffer, 1, ComponentDatatype::UnsignedByte, 1);
        assert_eq!(decoded, vec![255.0]);
    }
}
