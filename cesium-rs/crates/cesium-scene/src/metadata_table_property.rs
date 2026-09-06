//! Ported from `packages/engine/Source/Scene/MetadataTableProperty.js`.
//!
//! A single property within a `MetadataTable`, backed by a buffer view.

use serde_json::Value;

/// A single property within a [`MetadataTable`](super::metadata_table::MetadataTable).
///
/// Mirrors CesiumJS `MetadataTableProperty` — stores the property's type info,
/// buffer view reference, and per-element layout details needed to read
/// values out of the binary buffer.
#[derive(Debug, Clone)]
pub struct MetadataTableProperty {
    /// The property ID (key in the class properties dictionary).
    pub property_id: String,
    /// The property type name (e.g., "SCALAR", "VEC2", "MAT3", "STRING", "ENUM").
    pub property_type: String,
    /// The component type for numeric properties (e.g., "FLOAT32", "UINT16").
    pub component_type: Option<String>,
    /// The number of elements (rows) in this property.
    pub count: usize,
    /// Index into the buffer views dictionary.
    pub buffer_view_index: Option<usize>,
    /// Byte offset into the buffer view.
    pub byte_offset: usize,
    /// Byte stride between consecutive elements (0 = tightly packed).
    pub byte_stride: usize,
    /// Whether this property is an array type.
    pub is_array: bool,
    /// The number of components per element (e.g., 3 for VEC3).
    pub component_count: usize,
    /// Optional enum type name for ENUM properties.
    pub enum_type: Option<String>,
    /// Optional normalized flag (integer → [0,1] / [-1,1]).
    pub normalized: bool,
    /// Optional offset for quantized values.
    pub offset: Option<f64>,
    /// Optional scale for quantized values.
    pub scale: Option<f64>,
    /// Optional max value.
    pub max: Option<Value>,
    /// Optional min value.
    pub min: Option<Value>,
}

impl MetadataTableProperty {
    /// Creates a new `MetadataTableProperty` with the given ID and type.
    pub fn new(property_id: &str, property_type: &str, count: usize) -> Self {
        Self {
            property_id: property_id.to_string(),
            property_type: property_type.to_string(),
            component_type: None,
            count,
            buffer_view_index: None,
            byte_offset: 0,
            byte_stride: 0,
            is_array: false,
            component_count: 1,
            enum_type: None,
            normalized: false,
            offset: None,
            scale: None,
            max: None,
            min: None,
        }
    }

    /// Whether this property has binary data (a buffer view assigned).
    pub fn has_buffer_data(&self) -> bool {
        self.buffer_view_index.is_some()
    }
}

impl Default for MetadataTableProperty {
    fn default() -> Self {
        Self::new("", "SCALAR", 0)
    }
}
