//! Ported from `packages/engine/Source/Scene/I3dmParser.js`.

use serde_json::Value;

use cesium_core::get_json_from_typed_array::get_json_from_typed_array;
use cesium_core::runtime_error::RuntimeError;

const SIZE_OF_UINT32: usize = 4;

/// The result of parsing an Instanced 3D Model (i3dm) tile.
#[derive(Debug, Clone)]
pub struct I3dmParseResult {
    /// The glTF format: 0 = URI, 1 = embedded.
    pub gltf_format: u32,
    /// The feature table JSON.
    pub feature_table_json: Value,
    /// The feature table binary.
    pub feature_table_binary: Vec<u8>,
    /// The batch table JSON (if present).
    pub batch_table_json: Option<Value>,
    /// The batch table binary (if present).
    pub batch_table_binary: Option<Vec<u8>>,
    /// The embedded glTF data.
    pub gltf: Vec<u8>,
}

/// Handles parsing of an Instanced 3D Model.
pub struct I3dmParser;

impl I3dmParser {
    /// Parses the contents of an [Instanced 3D Model](https://github.com/CesiumGS/3d-tiles/tree/main/specification/TileFormats/Instanced3DModel).
    ///
    /// # Arguments
    /// * `data` - The byte buffer containing the i3dm.
    /// * `byte_offset` - The byte offset of the beginning of the i3dm in the buffer.
    ///
    /// # Returns
    /// A [`I3dmParseResult`] containing the glTF format, feature table (binary and JSON),
    /// batch table (binary and JSON), and glTF parts of the i3dm.
    pub fn parse(data: &[u8], byte_offset: Option<usize>) -> Result<I3dmParseResult, RuntimeError> {
        let byte_start = byte_offset.unwrap_or(0);
        let mut offset = byte_start;

        // Skip magic (4 bytes)
        offset += SIZE_OF_UINT32;

        // Version
        let version = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        if version != 1 {
            return Err(RuntimeError::new(Some(&format!(
                "Only Instanced 3D Model version 1 is supported. Version {version} is not."
            ))));
        }
        offset += SIZE_OF_UINT32;

        // Byte length
        let byte_length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += SIZE_OF_UINT32;

        // Feature table JSON byte length
        let feature_table_json_byte_length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        if feature_table_json_byte_length == 0 {
            return Err(RuntimeError::new(Some(
                "featureTableJsonByteLength is zero, the feature table must be defined.",
            )));
        }
        offset += SIZE_OF_UINT32;

        // Feature table binary byte length
        let feature_table_binary_byte_length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += SIZE_OF_UINT32;

        // Batch table JSON byte length
        let batch_table_json_byte_length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += SIZE_OF_UINT32;

        // Batch table binary byte length
        let batch_table_binary_byte_length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += SIZE_OF_UINT32;

        // glTF format
        let gltf_format = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        if gltf_format != 0 && gltf_format != 1 {
            return Err(RuntimeError::new(Some(&format!(
                "Only glTF format 0 (uri) or 1 (embedded) are supported. Format {gltf_format} is not."
            ))));
        }
        offset += SIZE_OF_UINT32;

        // Feature table JSON
        let feature_table_json_str =
            get_json_from_typed_array(data, Some(offset), Some(feature_table_json_byte_length));
        let feature_table_json: Value = serde_json::from_str(&feature_table_json_str)
            .unwrap_or(Value::Object(serde_json::Map::new()));
        offset += feature_table_json_byte_length;

        // Feature table binary
        let feature_table_binary = data[offset..offset + feature_table_binary_byte_length].to_vec();
        offset += feature_table_binary_byte_length;

        // Batch table (optional)
        let mut batch_table_json = None;
        let mut batch_table_binary = None;

        if batch_table_json_byte_length > 0 {
            let batch_table_json_str = get_json_from_typed_array(
                data,
                Some(offset),
                Some(batch_table_json_byte_length),
            );
            batch_table_json = serde_json::from_str(&batch_table_json_str).ok();
            offset += batch_table_json_byte_length;

            if batch_table_binary_byte_length > 0 {
                batch_table_binary =
                    Some(data[offset..offset + batch_table_binary_byte_length].to_vec());
                offset += batch_table_binary_byte_length;
            }
        }

        // glTF data
        let gltf_byte_length = byte_start + byte_length - offset;
        if gltf_byte_length == 0 {
            return Err(RuntimeError::new(Some(
                "glTF byte length must be greater than 0.",
            )));
        }

        let gltf = data[offset..offset + gltf_byte_length].to_vec();

        Ok(I3dmParseResult {
            gltf_format,
            feature_table_json,
            feature_table_binary,
            batch_table_json,
            batch_table_binary,
            gltf,
        })
    }
}
