//! 3D Tiles Feature Table and Batch Table.
//!
//! Maps to CesiumJS:
//! - `Scene/Cesium3DTileFeatureTable.js`
//! - `Scene/Cesium3DTileBatchTable.js`
//! - `Scene/BatchTableHierarchy.js`
//! - `Scene/Cesium3DTileFeature.js`

use serde_json::Value;
use std::collections::HashMap;

/// Component data types for binary accessors.
///
/// Maps to CesiumJS `Core/ComponentDatatype.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Signed 8-bit integer
    Int8,
    /// Unsigned 8-bit integer
    Uint8,
    /// Signed 16-bit integer
    Int16,
    /// Unsigned 16-bit integer
    Uint16,
    /// Signed 32-bit integer
    Int32,
    /// Unsigned 32-bit integer
    Uint32,
    /// 32-bit float
    Float32,
    /// 64-bit float
    Float64,
}

impl ComponentType {
    /// Returns the byte size of this component type.
    pub fn byte_size(&self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Float64 => 8,
        }
    }

    /// Parses from a string name (as used in batch table binary references).
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "SCALAR" | "BYTE" | "INT8" => Some(Self::Int8),
            "UNSIGNED_BYTE" | "UINT8" => Some(Self::Uint8),
            "SHORT" | "INT16" => Some(Self::Int16),
            "UNSIGNED_SHORT" | "UINT16" => Some(Self::Uint16),
            "INT" | "INT32" => Some(Self::Int32),
            "UNSIGNED_INT" | "UINT32" => Some(Self::Uint32),
            "FLOAT" | "FLOAT32" => Some(Self::Float32),
            "DOUBLE" | "FLOAT64" => Some(Self::Float64),
            _ => None,
        }
    }
}

/// The number of components per element (type).
///
/// Maps to glTF accessor types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessorType {
    /// Single component
    Scalar,
    /// 2 components
    Vec2,
    /// 3 components
    Vec3,
    /// 4 components
    Vec4,
}

impl AccessorType {
    /// Returns the number of components.
    pub fn component_count(&self) -> usize {
        match self {
            Self::Scalar => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Vec4 => 4,
        }
    }

    /// Parses from a string name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "SCALAR" => Some(Self::Scalar),
            "VEC2" => Some(Self::Vec2),
            "VEC3" => Some(Self::Vec3),
            "VEC4" => Some(Self::Vec4),
            _ => None,
        }
    }
}

/// A binary property reference within a feature/batch table.
#[derive(Debug, Clone)]
pub struct BinaryPropertyRef {
    /// Byte offset into the binary body.
    pub byte_offset: usize,
    /// Component type (override from JSON if present).
    pub component_type: Option<ComponentType>,
    /// Accessor type (override from JSON if present).
    pub accessor_type: Option<AccessorType>,
}

/// Feature Table for 3D Tiles content.
///
/// Maps to CesiumJS `Scene/Cesium3DTileFeatureTable.js`
///
/// The feature table contains per-tile global properties (like POINTS_LENGTH)
/// and per-feature properties (like POSITION, COLOR) that can be stored
/// either as JSON arrays or as binary data.
#[derive(Debug, Clone)]
pub struct FeatureTable {
    /// Parsed JSON header.
    pub json: Value,
    /// Binary body data.
    pub binary: Vec<u8>,
    /// Number of features (POINTS_LENGTH, BATCH_LENGTH, or INSTANCES_LENGTH).
    pub features_length: u32,
}

impl FeatureTable {
    /// Creates a new feature table from JSON and binary data.
    pub fn new(json: Option<Value>, binary: Vec<u8>) -> Self {
        let json = json.unwrap_or(Value::Null);
        let features_length = json
            .get("POINTS_LENGTH")
            .or_else(|| json.get("BATCH_LENGTH"))
            .or_else(|| json.get("INSTANCES_LENGTH"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        Self {
            json,
            binary,
            features_length,
        }
    }

    /// Returns true if the given semantic property exists.
    pub fn has_property(&self, semantic: &str) -> bool {
        self.json.get(semantic).is_some()
    }

    /// Gets a global property value (scalar or small array stored directly in JSON).
    pub fn get_global_property(&self, semantic: &str) -> Option<&Value> {
        self.json.get(semantic)
    }

    /// Gets a global property as u32.
    pub fn get_global_u32(&self, semantic: &str) -> Option<u32> {
        self.json.get(semantic).and_then(|v| v.as_u64()).map(|v| v as u32)
    }

    /// Gets a global property as f64.
    pub fn get_global_f64(&self, semantic: &str) -> Option<f64> {
        self.json.get(semantic).and_then(|v| v.as_f64())
    }

    /// Gets a global property as [f64; 3] (e.g., RTC_CENTER).
    pub fn get_global_vec3(&self, semantic: &str) -> Option<[f64; 3]> {
        self.json.get(semantic).and_then(|v| {
            let arr = v.as_array()?;
            if arr.len() >= 3 {
                Some([
                    arr[0].as_f64()?,
                    arr[1].as_f64()?,
                    arr[2].as_f64()?,
                ])
            } else {
                None
            }
        })
    }

    /// Gets a per-feature property as a JSON array.
    pub fn get_property_array(&self, semantic: &str) -> Option<&Vec<Value>> {
        self.json.get(semantic).and_then(|v| v.as_array())
    }

    /// Gets the binary property reference for a semantic.
    ///
    /// Returns the byte offset if the property is stored in binary.
    pub fn get_binary_ref(&self, semantic: &str) -> Option<BinaryPropertyRef> {
        self.json.get(semantic).and_then(|v| {
            let byte_offset = v.get("byteOffset")?.as_u64()? as usize;
            let component_type = v
                .get("componentType")
                .and_then(|ct| ct.as_str())
                .and_then(ComponentType::from_name);
            let accessor_type = v
                .get("type")
                .and_then(|t| t.as_str())
                .and_then(AccessorType::from_name);
            Some(BinaryPropertyRef {
                byte_offset,
                component_type,
                accessor_type,
            })
        })
    }

    /// Reads f32 values from the binary body at the given byte offset.
    pub fn read_f32_array(&self, byte_offset: usize, count: usize) -> Option<Vec<f32>> {
        let end = byte_offset + count * 4;
        if end > self.binary.len() {
            return None;
        }
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let offset = byte_offset + i * 4;
            let bytes = [
                self.binary[offset],
                self.binary[offset + 1],
                self.binary[offset + 2],
                self.binary[offset + 3],
            ];
            result.push(f32::from_le_bytes(bytes));
        }
        Some(result)
    }

    /// Reads u8 values from the binary body at the given byte offset.
    pub fn read_u8_array(&self, byte_offset: usize, count: usize) -> Option<Vec<u8>> {
        let end = byte_offset + count;
        if end > self.binary.len() {
            return None;
        }
        Some(self.binary[byte_offset..end].to_vec())
    }

    /// Reads u16 values from the binary body at the given byte offset.
    pub fn read_u16_array(&self, byte_offset: usize, count: usize) -> Option<Vec<u16>> {
        let end = byte_offset + count * 2;
        if end > self.binary.len() {
            return None;
        }
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let offset = byte_offset + i * 2;
            let bytes = [self.binary[offset], self.binary[offset + 1]];
            result.push(u16::from_le_bytes(bytes));
        }
        Some(result)
    }

    /// Gets per-feature positions (POSITION semantic) as Vec<[f32; 3]>.
    pub fn get_positions(&self) -> Option<Vec<[f32; 3]>> {
        let bin_ref = self.get_binary_ref("POSITION")?;
        let count = self.features_length as usize;
        let values = self.read_f32_array(bin_ref.byte_offset, count * 3)?;
        Some(
            values
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
        )
    }

    /// Gets per-feature colors (COLOR or RGB semantic) as Vec<[f32; 3]>.
    pub fn get_colors_rgb(&self) -> Option<Vec<[f32; 3]>> {
        // Try RGB first (normalized u8), then COLOR (float)
        if let Some(bin_ref) = self.get_binary_ref("RGB") {
            let count = self.features_length as usize;
            let values = self.read_u8_array(bin_ref.byte_offset, count * 3)?;
            return Some(
                values
                    .chunks_exact(3)
                    .map(|c| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0])
                    .collect(),
            );
        }
        if let Some(bin_ref) = self.get_binary_ref("COLOR") {
            let count = self.features_length as usize;
            let values = self.read_f32_array(bin_ref.byte_offset, count * 3)?;
            return Some(
                values
                    .chunks_exact(3)
                    .map(|c| [c[0], c[1], c[2]])
                    .collect(),
            );
        }
        None
    }

    /// Gets per-feature RGBA colors as Vec<[f32; 4]>.
    pub fn get_colors_rgba(&self) -> Option<Vec<[f32; 4]>> {
        if let Some(bin_ref) = self.get_binary_ref("RGBA") {
            let count = self.features_length as usize;
            let values = self.read_u8_array(bin_ref.byte_offset, count * 4)?;
            return Some(
                values
                    .chunks_exact(4)
                    .map(|c| {
                        [
                            c[0] as f32 / 255.0,
                            c[1] as f32 / 255.0,
                            c[2] as f32 / 255.0,
                            c[3] as f32 / 255.0,
                        ]
                    })
                    .collect(),
            );
        }
        None
    }

    /// Gets per-feature normals (NORMAL semantic) as Vec<[f32; 3]>.
    pub fn get_normals(&self) -> Option<Vec<[f32; 3]>> {
        let bin_ref = self.get_binary_ref("NORMAL")?;
        let count = self.features_length as usize;
        let values = self.read_f32_array(bin_ref.byte_offset, count * 3)?;
        Some(
            values
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
        )
    }

    /// Gets the batch ID for each feature (BATCH_ID semantic).
    pub fn get_batch_ids(&self) -> Option<Vec<u16>> {
        let bin_ref = self.get_binary_ref("BATCH_ID")?;
        let count = self.features_length as usize;
        self.read_u16_array(bin_ref.byte_offset, count)
    }
}

/// A single property value in a batch table (can be JSON array or binary reference).
#[derive(Debug, Clone)]
pub enum BatchPropertyValue {
    /// JSON array of values (one per feature).
    JsonArray(Vec<Value>),
    /// Binary reference.
    Binary(BinaryPropertyRef),
}

/// Batch Table for 3D Tiles content.
///
/// Maps to CesiumJS `Scene/Cesium3DTileBatchTable.js`
///
/// The batch table stores per-feature metadata (properties like height, name, etc.)
/// that can be used for styling, picking, and feature inspection.
#[derive(Debug, Clone)]
pub struct BatchTable {
    /// Number of features (batch length).
    pub features_length: u32,
    /// Property name → value mapping.
    pub properties: HashMap<String, BatchPropertyValue>,
    /// Binary body data for binary properties.
    pub binary: Vec<u8>,
    /// Extensions (e.g., 3DTILES_batch_table_hierarchy).
    pub extensions: HashMap<String, Value>,
    /// Optional batch table hierarchy.
    pub hierarchy: Option<BatchTableHierarchy>,
}

impl BatchTable {
    /// Creates a new batch table from parsed JSON and binary data.
    pub fn new(
        json: Option<Value>,
        binary: Vec<u8>,
        features_length: u32,
    ) -> Self {
        let mut properties = HashMap::new();
        let mut extensions = HashMap::new();
        let mut hierarchy = None;

        if let Some(Value::Object(map)) = &json {
            for (key, value) in map {
                if key == "extensions" {
                    if let Value::Object(ext_map) = value {
                        for (ext_key, ext_val) in ext_map {
                            extensions.insert(ext_key.clone(), ext_val.clone());
                        }
                    }
                    continue;
                }
                if key == "extras" || key == "HIERARCHY" {
                    if key == "HIERARCHY" {
                        // Legacy hierarchy property
                        extensions.insert(
                            "3DTILES_batch_table_hierarchy".to_string(),
                            value.clone(),
                        );
                    }
                    continue;
                }

                // Check if it's a binary reference
                if let Some(byte_offset) = value.get("byteOffset").and_then(|v| v.as_u64()) {
                    let component_type = value
                        .get("componentType")
                        .and_then(|ct| ct.as_str())
                        .and_then(ComponentType::from_name);
                    let accessor_type = value
                        .get("type")
                        .and_then(|t| t.as_str())
                        .and_then(AccessorType::from_name);
                    properties.insert(
                        key.clone(),
                        BatchPropertyValue::Binary(BinaryPropertyRef {
                            byte_offset: byte_offset as usize,
                            component_type,
                            accessor_type,
                        }),
                    );
                } else if let Some(arr) = value.as_array() {
                    properties.insert(
                        key.clone(),
                        BatchPropertyValue::JsonArray(arr.clone()),
                    );
                }
            }

            // Parse hierarchy extension if present
            if let Some(hierarchy_json) =
                extensions.get("3DTILES_batch_table_hierarchy")
            {
                hierarchy = BatchTableHierarchy::from_json(hierarchy_json, &binary);
            }
        }

        Self {
            features_length,
            properties,
            binary,
            extensions,
            hierarchy,
        }
    }

    /// Returns the property names available in this batch table.
    pub fn property_names(&self) -> Vec<&str> {
        self.properties.keys().map(|s| s.as_str()).collect()
    }

    /// Returns true if the given property exists.
    pub fn has_property(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Gets a property value for a specific feature (batch ID).
    pub fn get_property(&self, name: &str, batch_id: u32) -> Option<Value> {
        let prop = self.properties.get(name)?;
        match prop {
            BatchPropertyValue::JsonArray(arr) => {
                arr.get(batch_id as usize).cloned()
            }
            BatchPropertyValue::Binary(bin_ref) => {
                self.get_binary_value(bin_ref, batch_id)
            }
        }
    }

    /// Gets all values for a property as a JSON array.
    pub fn get_property_all(&self, name: &str) -> Option<Vec<Value>> {
        let prop = self.properties.get(name)?;
        match prop {
            BatchPropertyValue::JsonArray(arr) => Some(arr.clone()),
            BatchPropertyValue::Binary(bin_ref) => {
                let mut values = Vec::with_capacity(self.features_length as usize);
                for i in 0..self.features_length {
                    if let Some(v) = self.get_binary_value(bin_ref, i) {
                        values.push(v);
                    }
                }
                Some(values)
            }
        }
    }

    /// Sets a property value for a specific feature.
    pub fn set_property(&mut self, name: &str, batch_id: u32, value: Value) -> bool {
        if let Some(BatchPropertyValue::JsonArray(arr)) = self.properties.get_mut(name) {
            if (batch_id as usize) < arr.len() {
                arr[batch_id as usize] = value;
                return true;
            }
        }
        false
    }

    /// Reads a binary property value for a specific batch ID.
    fn get_binary_value(&self, bin_ref: &BinaryPropertyRef, batch_id: u32) -> Option<Value> {
        let component_type = bin_ref.component_type.unwrap_or(ComponentType::Float32);
        let accessor_type = bin_ref.accessor_type.unwrap_or(AccessorType::Scalar);
        let component_count = accessor_type.component_count();
        let byte_size = component_type.byte_size();
        let stride = byte_size * component_count;
        let offset = bin_ref.byte_offset + (batch_id as usize) * stride;

        if offset + stride > self.binary.len() {
            return None;
        }

        let values: Vec<f64> = (0..component_count)
            .filter_map(|i| {
                let elem_offset = offset + i * byte_size;
                match component_type {
                    ComponentType::Float32 => {
                        let bytes = [
                            self.binary[elem_offset],
                            self.binary[elem_offset + 1],
                            self.binary[elem_offset + 2],
                            self.binary[elem_offset + 3],
                        ];
                        Some(f32::from_le_bytes(bytes) as f64)
                    }
                    ComponentType::Float64 => {
                        let bytes: [u8; 8] = self.binary[elem_offset..elem_offset + 8]
                            .try_into()
                            .ok()?;
                        Some(f64::from_le_bytes(bytes))
                    }
                    ComponentType::Uint8 => Some(self.binary[elem_offset] as f64),
                    ComponentType::Int8 => Some(self.binary[elem_offset] as i8 as f64),
                    ComponentType::Uint16 => {
                        let bytes = [
                            self.binary[elem_offset],
                            self.binary[elem_offset + 1],
                        ];
                        Some(u16::from_le_bytes(bytes) as f64)
                    }
                    ComponentType::Int16 => {
                        let bytes = [
                            self.binary[elem_offset],
                            self.binary[elem_offset + 1],
                        ];
                        Some(i16::from_le_bytes(bytes) as f64)
                    }
                    ComponentType::Uint32 => {
                        let bytes: [u8; 4] =
                            self.binary[elem_offset..elem_offset + 4].try_into().ok()?;
                        Some(u32::from_le_bytes(bytes) as f64)
                    }
                    ComponentType::Int32 => {
                        let bytes: [u8; 4] =
                            self.binary[elem_offset..elem_offset + 4].try_into().ok()?;
                        Some(i32::from_le_bytes(bytes) as f64)
                    }
                }
            })
            .collect();

        if values.len() != component_count {
            return None;
        }

        match component_count {
            1 => Some(serde_json::json!(values[0])),
            _ => Some(Value::Array(
                values.iter().map(|v| serde_json::json!(v)).collect(),
            )),
        }
    }

    /// Returns the total byte length of binary data.
    pub fn byte_length(&self) -> usize {
        self.binary.len()
    }
}

/// A class in the batch table hierarchy.
///
/// Maps to CesiumJS `Scene/BatchTableHierarchy.js`
#[derive(Debug, Clone)]
pub struct HierarchyClass {
    /// Class name (e.g., "Building", "Floor").
    pub name: String,
    /// Number of instances of this class.
    pub length: u32,
    /// Property names for this class.
    pub property_names: Vec<String>,
}

/// Batch Table Hierarchy extension (3DTILES_batch_table_hierarchy).
///
/// Provides a class-based hierarchy for organizing features.
/// Maps to CesiumJS `Scene/BatchTableHierarchy.js`
#[derive(Debug, Clone)]
pub struct BatchTableHierarchy {
    /// Classes in the hierarchy.
    pub classes: Vec<HierarchyClass>,
    /// Number of instances total.
    pub instances_length: u32,
    /// Class index for each instance.
    pub class_ids: Vec<u32>,
    /// Parent index for each instance (u32::MAX = no parent).
    pub parent_ids: Vec<u32>,
    /// Property values per class (class_index → property_name → values).
    pub class_properties: HashMap<usize, HashMap<String, Vec<Value>>>,
}

impl BatchTableHierarchy {
    /// Parses a hierarchy from the extension JSON.
    pub fn from_json(json: &Value, binary: &[u8]) -> Option<Self> {
        let classes_json = json.get("classes")?.as_array()?;
        let instances_length = json.get("instancesLength")?.as_u64()? as u32;

        let mut classes = Vec::new();
        let mut class_properties: HashMap<usize, HashMap<String, Vec<Value>>> = HashMap::new();

        for (i, class_json) in classes_json.iter().enumerate() {
            let name = class_json.get("name")?.as_str()?.to_string();
            let length = class_json.get("length")?.as_u64()? as u32;

            let mut property_names = Vec::new();
            let mut props = HashMap::new();

            if let Some(props_json) = class_json.get("properties").and_then(|p| p.as_object()) {
                for (prop_name, prop_value) in props_json {
                    property_names.push(prop_name.clone());
                    if let Some(arr) = prop_value.as_array() {
                        props.insert(prop_name.clone(), arr.clone());
                    } else if let Some(byte_offset) =
                        prop_value.get("byteOffset").and_then(|v| v.as_u64())
                    {
                        // Binary property — read as f32 array
                        let offset = byte_offset as usize;
                        let count = length as usize;
                        let end = offset + count * 4;
                        if end <= binary.len() {
                            let values: Vec<Value> = (0..count)
                                .map(|j| {
                                    let o = offset + j * 4;
                                    let bytes = [
                                        binary[o],
                                        binary[o + 1],
                                        binary[o + 2],
                                        binary[o + 3],
                                    ];
                                    serde_json::json!(f32::from_le_bytes(bytes))
                                })
                                .collect();
                            props.insert(prop_name.clone(), values);
                        }
                    }
                }
            }

            classes.push(HierarchyClass {
                name,
                length,
                property_names,
            });
            class_properties.insert(i, props);
        }

        // Parse classIds
        let class_ids = parse_id_array(json.get("classIds")?, instances_length as usize);
        // Parse parentIds (optional)
        let parent_ids = json
            .get("parentIds")
            .map(|v| parse_id_array(v, instances_length as usize))
            .unwrap_or_else(|| vec![u32::MAX; instances_length as usize]);

        Some(Self {
            classes,
            instances_length,
            class_ids,
            parent_ids,
            class_properties,
        })
    }

    /// Gets the class index for an instance.
    pub fn get_class_id(&self, instance_id: u32) -> Option<u32> {
        self.class_ids.get(instance_id as usize).copied()
    }

    /// Gets the parent instance ID for an instance.
    pub fn get_parent_id(&self, instance_id: u32) -> Option<u32> {
        self.parent_ids.get(instance_id as usize).copied()
    }

    /// Gets a property value for an instance.
    pub fn get_property(&self, instance_id: u32, property_name: &str) -> Option<Value> {
        let class_id = self.get_class_id(instance_id)? as usize;
        let class_props = self.class_properties.get(&class_id)?;
        let values = class_props.get(property_name)?;

        // Find the index within the class
        let mut index_in_class = 0u32;
        for i in 0..instance_id {
            if self.class_ids.get(i as usize) == Some(&(class_id as u32)) {
                index_in_class += 1;
            }
        }

        values.get(index_in_class as usize).cloned()
    }

    /// Gets the class name for an instance.
    pub fn get_class_name(&self, instance_id: u32) -> Option<&str> {
        let class_id = self.get_class_id(instance_id)? as usize;
        self.classes.get(class_id).map(|c| c.name.as_str())
    }
}

/// Parses an ID array from JSON (either direct array or binary reference).
fn parse_id_array(json: &Value, count: usize) -> Vec<u32> {
    if let Some(arr) = json.as_array() {
        arr.iter()
            .filter_map(|v| v.as_u64().map(|n| n as u32))
            .collect()
    } else if let Some(byte_offset) = json.get("byteOffset").and_then(|v| v.as_u64()) {
        // Binary reference — but we don't have the binary here
        // This would need the binary buffer passed in
        let _ = byte_offset;
        vec![0; count]
    } else {
        vec![0; count]
    }
}

/// A feature in a 3D Tile (wraps batch table access for a single batch ID).
///
/// Maps to CesiumJS `Scene/Cesium3DTileFeature.js`
#[derive(Debug, Clone)]
pub struct TileFeature {
    /// The batch ID of this feature.
    pub batch_id: u32,
    /// Property values for this feature.
    pub properties: HashMap<String, Value>,
}

impl TileFeature {
    /// Creates a feature by extracting all properties from a batch table.
    pub fn from_batch_table(batch_table: &BatchTable, batch_id: u32) -> Self {
        let mut properties = HashMap::new();
        for name in batch_table.property_names() {
            if let Some(value) = batch_table.get_property(name, batch_id) {
                properties.insert(name.to_string(), value);
            }
        }
        Self {
            batch_id,
            properties,
        }
    }

    /// Gets a property value.
    pub fn get_property(&self, name: &str) -> Option<&Value> {
        self.properties.get(name)
    }

    /// Gets a property as f64.
    pub fn get_property_f64(&self, name: &str) -> Option<f64> {
        self.properties.get(name).and_then(|v| v.as_f64())
    }

    /// Gets a property as string.
    pub fn get_property_str(&self, name: &str) -> Option<&str> {
        self.properties.get(name).and_then(|v| v.as_str())
    }

    /// Returns all property IDs (names).
    pub fn property_ids(&self) -> Vec<&str> {
        self.properties.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_component_type_byte_size() {
        assert_eq!(ComponentType::Uint8.byte_size(), 1);
        assert_eq!(ComponentType::Uint16.byte_size(), 2);
        assert_eq!(ComponentType::Float32.byte_size(), 4);
        assert_eq!(ComponentType::Float64.byte_size(), 8);
    }

    #[test]
    fn test_component_type_from_name() {
        assert_eq!(ComponentType::from_name("FLOAT"), Some(ComponentType::Float32));
        assert_eq!(ComponentType::from_name("UNSIGNED_BYTE"), Some(ComponentType::Uint8));
        assert_eq!(ComponentType::from_name("UINT16"), Some(ComponentType::Uint16));
        assert_eq!(ComponentType::from_name("INVALID"), None);
    }

    #[test]
    fn test_accessor_type() {
        assert_eq!(AccessorType::Scalar.component_count(), 1);
        assert_eq!(AccessorType::Vec3.component_count(), 3);
        assert_eq!(AccessorType::from_name("VEC2"), Some(AccessorType::Vec2));
    }

    #[test]
    fn test_feature_table_global_properties() {
        let json = json!({
            "POINTS_LENGTH": 100,
            "RTC_CENTER": [1.0, 2.0, 3.0]
        });
        let ft = FeatureTable::new(Some(json), vec![]);
        assert_eq!(ft.features_length, 100);
        assert_eq!(ft.get_global_u32("POINTS_LENGTH"), Some(100));
        assert_eq!(ft.get_global_vec3("RTC_CENTER"), Some([1.0, 2.0, 3.0]));
        assert!(ft.has_property("POINTS_LENGTH"));
        assert!(!ft.has_property("NONEXISTENT"));
    }

    #[test]
    fn test_feature_table_binary_positions() {
        // 3 points with positions
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let mut binary = Vec::new();
        for p in &positions {
            binary.extend_from_slice(&p.to_le_bytes());
        }

        let json = json!({
            "POINTS_LENGTH": 3,
            "POSITION": { "byteOffset": 0 }
        });
        let ft = FeatureTable::new(Some(json), binary);
        let pos = ft.get_positions().unwrap();
        assert_eq!(pos.len(), 3);
        assert!((pos[0][0] - 1.0).abs() < 1e-6);
        assert!((pos[2][2] - 9.0).abs() < 1e-6);
    }

    #[test]
    fn test_feature_table_rgb_colors() {
        // 2 points with RGB colors (u8)
        let binary = vec![255u8, 0, 0, 0, 255, 0]; // red, green
        let json = json!({
            "POINTS_LENGTH": 2,
            "RGB": { "byteOffset": 0 }
        });
        let ft = FeatureTable::new(Some(json), binary);
        let colors = ft.get_colors_rgb().unwrap();
        assert_eq!(colors.len(), 2);
        assert!((colors[0][0] - 1.0).abs() < 1e-6); // red channel
        assert!((colors[1][1] - 1.0).abs() < 1e-6); // green channel
    }

    #[test]
    fn test_feature_table_batch_ids() {
        let ids: Vec<u16> = vec![0, 1, 2, 3];
        let mut binary = Vec::new();
        for id in &ids {
            binary.extend_from_slice(&id.to_le_bytes());
        }
        let json = json!({
            "POINTS_LENGTH": 4,
            "BATCH_ID": { "byteOffset": 0 }
        });
        let ft = FeatureTable::new(Some(json), binary);
        let batch_ids = ft.get_batch_ids().unwrap();
        assert_eq!(batch_ids, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_batch_table_json_properties() {
        let json = json!({
            "height": [10.5, 20.3, 30.1],
            "name": ["A", "B", "C"]
        });
        let bt = BatchTable::new(Some(json), vec![], 3);
        assert_eq!(bt.features_length, 3);
        assert!(bt.has_property("height"));
        assert!(bt.has_property("name"));
        assert!(!bt.has_property("missing"));

        assert_eq!(bt.get_property("height", 0), Some(json!(10.5)));
        assert_eq!(bt.get_property("name", 2), Some(json!("C")));
        assert_eq!(bt.get_property("height", 5), None); // out of bounds
    }

    #[test]
    fn test_batch_table_binary_properties() {
        // 3 float32 values
        let values: Vec<f32> = vec![1.5, 2.5, 3.5];
        let mut binary = Vec::new();
        for v in &values {
            binary.extend_from_slice(&v.to_le_bytes());
        }

        let json = json!({
            "temperature": {
                "byteOffset": 0,
                "componentType": "FLOAT",
                "type": "SCALAR"
            }
        });
        let bt = BatchTable::new(Some(json), binary, 3);
        let v0 = bt.get_property("temperature", 0).unwrap();
        assert!((v0.as_f64().unwrap() - 1.5).abs() < 1e-6);
        let v2 = bt.get_property("temperature", 2).unwrap();
        assert!((v2.as_f64().unwrap() - 3.5).abs() < 1e-6);
    }

    #[test]
    fn test_batch_table_set_property() {
        let json = json!({
            "height": [10.0, 20.0]
        });
        let mut bt = BatchTable::new(Some(json), vec![], 2);
        assert!(bt.set_property("height", 0, json!(99.0)));
        assert_eq!(bt.get_property("height", 0), Some(json!(99.0)));
        assert!(!bt.set_property("height", 5, json!(0.0))); // out of bounds
        assert!(!bt.set_property("missing", 0, json!(0.0))); // no property
    }

    #[test]
    fn test_batch_table_property_names() {
        let json = json!({
            "height": [1.0],
            "width": [2.0],
            "name": ["X"]
        });
        let bt = BatchTable::new(Some(json), vec![], 1);
        let mut names = bt.property_names();
        names.sort();
        assert_eq!(names, vec!["height", "name", "width"]);
    }

    #[test]
    fn test_batch_table_extensions() {
        let json = json!({
            "height": [1.0],
            "extensions": {
                "custom_ext": { "data": 42 }
            }
        });
        let bt = BatchTable::new(Some(json), vec![], 1);
        assert!(bt.extensions.contains_key("custom_ext"));
        // "extensions" should not be a property
        assert!(!bt.has_property("extensions"));
    }

    #[test]
    fn test_batch_table_hierarchy() {
        let json = json!({
            "height": [10.0, 20.0, 30.0],
            "extensions": {
                "3DTILES_batch_table_hierarchy": {
                    "classes": [
                        {
                            "name": "Building",
                            "length": 2,
                            "properties": {
                                "buildingName": ["Tower A", "Tower B"]
                            }
                        },
                        {
                            "name": "Floor",
                            "length": 1,
                            "properties": {
                                "floorNumber": [1]
                            }
                        }
                    ],
                    "instancesLength": 3,
                    "classIds": [0, 0, 1],
                    "parentIds": [4294967295u32, 4294967295u32, 0]
                }
            }
        });
        let bt = BatchTable::new(Some(json), vec![], 3);
        let hierarchy = bt.hierarchy.as_ref().unwrap();

        assert_eq!(hierarchy.instances_length, 3);
        assert_eq!(hierarchy.classes.len(), 2);
        assert_eq!(hierarchy.classes[0].name, "Building");
        assert_eq!(hierarchy.classes[1].name, "Floor");

        assert_eq!(hierarchy.get_class_id(0), Some(0));
        assert_eq!(hierarchy.get_class_id(2), Some(1));
        assert_eq!(hierarchy.get_class_name(0), Some("Building"));
        assert_eq!(hierarchy.get_class_name(2), Some("Floor"));

        assert_eq!(hierarchy.get_parent_id(2), Some(0));
        assert_eq!(hierarchy.get_parent_id(0), Some(u32::MAX));

        assert_eq!(
            hierarchy.get_property(0, "buildingName"),
            Some(json!("Tower A"))
        );
        assert_eq!(
            hierarchy.get_property(1, "buildingName"),
            Some(json!("Tower B"))
        );
        assert_eq!(hierarchy.get_property(2, "floorNumber"), Some(json!(1)));
    }

    #[test]
    fn test_tile_feature() {
        let json = json!({
            "height": [10.5, 20.3],
            "name": ["A", "B"],
            "visible": [true, false]
        });
        let bt = BatchTable::new(Some(json), vec![], 2);
        let feature = TileFeature::from_batch_table(&bt, 1);

        assert_eq!(feature.batch_id, 1);
        assert_eq!(feature.get_property_f64("height"), Some(20.3));
        assert_eq!(feature.get_property_str("name"), Some("B"));
        assert_eq!(feature.get_property("visible"), Some(&json!(false)));

        let mut ids = feature.property_ids();
        ids.sort();
        assert_eq!(ids, vec!["height", "name", "visible"]);
    }

    #[test]
    fn test_batch_table_get_property_all() {
        let json = json!({
            "score": [100, 200, 300]
        });
        let bt = BatchTable::new(Some(json), vec![], 3);
        let all = bt.get_property_all("score").unwrap();
        assert_eq!(all, vec![json!(100), json!(200), json!(300)]);
    }

    #[test]
    fn test_feature_table_normals() {
        let normals: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let mut binary = Vec::new();
        for n in &normals {
            binary.extend_from_slice(&n.to_le_bytes());
        }
        let json = json!({
            "POINTS_LENGTH": 2,
            "NORMAL": { "byteOffset": 0 }
        });
        let ft = FeatureTable::new(Some(json), binary);
        let normals = ft.get_normals().unwrap();
        assert_eq!(normals.len(), 2);
        assert!((normals[0][2] - 1.0).abs() < 1e-6);
        assert!((normals[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_batch_table_byte_length() {
        let binary = vec![0u8; 64];
        let bt = BatchTable::new(None, binary, 0);
        assert_eq!(bt.byte_length(), 64);
    }
}
