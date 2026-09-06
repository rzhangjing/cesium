//! Ported from `packages/engine/Source/Scene/parseBatchTable.js`.
//!
//! Parses a 3D Tiles 1.0 batch table into structured metadata,
//! transcoding binary properties to EXT_structural_metadata format.

use serde_json::Value;

/// Options for parsing a batch table.
pub struct ParseBatchTableOptions {
    /// The number of features.
    pub count: u32,
    /// The batch table JSON object.
    pub batch_table: Value,
    /// Optional binary body (for binary property data).
    pub binary_body: Option<Vec<u8>>,
    /// Whether to parse as property attributes (for .pnts).
    pub parse_as_property_attributes: bool,
}

/// Result of parsing a batch table.
pub struct ParsedBatchTable {
    /// The parsed JSON properties (property_id → values array).
    pub json_properties: std::collections::HashMap<String, Value>,
    /// The number of features.
    pub features_length: u32,
    /// Optional hierarchy data.
    pub hierarchy: Option<Value>,
    /// Optional extras.
    pub extras: Option<Value>,
    /// Optional extensions.
    pub extensions: Option<Value>,
}

/// Partitions a batch table JSON into binary, JSON, hierarchy, extras, and extensions.
pub fn partition_properties(batch_table: &Value) -> PartitionedProperties {
    let mut binary = std::collections::HashMap::new();
    let mut json = std::collections::HashMap::new();
    let mut hierarchy = None;
    let mut extras = None;
    let mut extensions = None;

    if let Some(obj) = batch_table.as_object() {
        for (key, value) in obj {
            match key.as_str() {
                "HIERARCHY" => hierarchy = Some(value.clone()),
                "extras" => extras = Some(value.clone()),
                "extensions" => extensions = Some(value.clone()),
                _ => {
                    if value.is_object() && value.get("byteOffset").is_some() {
                        binary.insert(key.clone(), value.clone());
                    } else {
                        json.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    PartitionedProperties {
        binary,
        json,
        hierarchy,
        extras,
        extensions,
    }
}

/// Partitioned batch table properties.
pub struct PartitionedProperties {
    /// Binary properties (property_id → accessor with byteOffset).
    pub binary: std::collections::HashMap<String, Value>,
    /// JSON properties (property_id → values array).
    pub json: std::collections::HashMap<String, Value>,
    /// Hierarchy data, if present.
    pub hierarchy: Option<Value>,
    /// Extras, if present.
    pub extras: Option<Value>,
    /// Extensions, if present.
    pub extensions: Option<Value>,
}

/// Transcodes a legacy component type string to the EXT_structural_metadata equivalent.
pub fn transcode_component_type(component_type: &str) -> &'static str {
    match component_type.to_uppercase().as_str() {
        "BYTE" => "INT8",
        "UNSIGNED_BYTE" => "UINT8",
        "SHORT" => "INT16",
        "UNSIGNED_SHORT" => "UINT16",
        "INT" => "INT32",
        "UNSIGNED_INT" => "UINT32",
        "FLOAT" => "FLOAT32",
        "DOUBLE" => "FLOAT64",
        _ => "UINT8",
    }
}

/// Parses a batch table from JSON and optional binary data.
///
/// Returns a [`ParsedBatchTable`] with the extracted properties.
pub fn parse_batch_table(options: &ParseBatchTableOptions) -> ParsedBatchTable {
    let partitioned = partition_properties(&options.batch_table);

    ParsedBatchTable {
        json_properties: partitioned.json,
        features_length: options.count,
        hierarchy: partitioned.hierarchy,
        extras: partitioned.extras,
        extensions: partitioned.extensions,
    }
}
