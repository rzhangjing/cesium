//! Ported from `packages/engine/Source/Scene/GltfPipeline/parseGlb.js`.
//!
//! Convert a binary glTF (GLB) to glTF. The embedded binary data is stored
//! in `buffers[0].data` (version 2) or the `binary_glTF` /
//! `KHR_binary_glTF` buffer (version 1), mirroring CesiumJS
//! `buffer.extras._pipeline.source`.

use cesium_core::get_magic::get_magic;
use cesium_core::runtime_error::RuntimeError;

use crate::gltf_loader::GltfJson;

const SIZE_OF_UINT32: usize = 4;

/// Converts a binary glTF to glTF.
///
/// The returned glTF has the embedded binary chunk attached to its first
/// buffer (`GltfBuffer::data`), the Rust analogue of CesiumJS pipeline
/// extras (`buffer.extras._pipeline.source`).
///
/// # Errors
/// Returns a [`RuntimeError`] when the container is not a valid GLB
/// (bad magic, unsupported version, or non-JSON content format).
pub fn parse_glb(glb: &[u8]) -> Result<GltfJson, RuntimeError> {
    // Check that the magic string is present
    let magic = get_magic(glb, None);
    if magic != "glTF" {
        return Err(RuntimeError::new(Some("File is not valid binary glTF")));
    }

    let header = read_header(glb, 0, 5)?;
    let version = header[1];
    if version != 1 && version != 2 {
        return Err(RuntimeError::new(Some(
            "Binary glTF version is not 1 or 2",
        )));
    }

    if version == 1 {
        return parse_glb_version1(glb, &header);
    }

    parse_glb_version2(glb, &header)
}

fn read_header(glb: &[u8], byte_offset: usize, count: usize) -> Result<Vec<u32>, RuntimeError> {
    let mut header = Vec::with_capacity(count);
    for i in 0..count {
        let start = byte_offset + i * SIZE_OF_UINT32;
        let end = start + SIZE_OF_UINT32;
        if end > glb.len() {
            return Err(RuntimeError::new(Some(
                "Invalid binary glTF: header exceeds buffer length",
            )));
        }
        header.push(u32::from_le_bytes([
            glb[start],
            glb[start + 1],
            glb[start + 2],
            glb[start + 3],
        ]));
    }
    Ok(header)
}

fn parse_glb_version1(glb: &[u8], header: &[u32]) -> Result<GltfJson, RuntimeError> {
    let length = header[2] as usize;
    let content_length = header[3] as usize;
    let content_format = header[4];

    // Check that the content format is 0, indicating that it is JSON
    if content_format != 0 {
        return Err(RuntimeError::new(Some(
            "Binary glTF scene format is not JSON",
        )));
    }

    let json_start = 20;
    let binary_start = json_start + content_length;

    let content_string = get_string_from_typed_array(glb, json_start, content_length)?;
    let mut gltf_value: serde_json::Value = serde_json::from_str(content_string)
        .map_err(|e| RuntimeError::new(Some(&format!("Failed to parse binary glTF JSON: {e}"))))?;

    let binary_buffer = slice_or_err(glb, binary_start, length.min(glb.len()))?;

    // In some older models, the binary glTF buffer is named KHR_binary_glTF.
    // glTF 1.0 stores `buffers` as an object keyed by buffer name; normalize
    // it to the glTF 2.0 array form while attaching the binary chunk.
    // DEVIATION: CesiumJS keeps the original object and relies on
    // updateVersion() for the 1.0 -> 2.0 upgrade; the Rust port performs the
    // buffer normalization here and defers the full updateVersion pipeline.
    if let Some(object) = gltf_value.get_mut("buffers").and_then(|b| b.as_object_mut()) {
        let binary_key = if object.contains_key("binary_glTF") {
            Some("binary_glTF")
        } else if object.contains_key("KHR_binary_glTF") {
            Some("KHR_binary_glTF")
        } else {
            None
        };
        if let Some(key) = binary_key {
            if let Some(buffer) = object.get_mut(key).and_then(|b| b.as_object_mut()) {
                buffer["extras"] = serde_json::json!({ "_pipeline": { "source": [] } });
                buffer.remove("uri");
                // Remember which normalized array slot receives the binary data.
                buffer["_binarySource"] = serde_json::Value::Bool(true);
            }
        }

        // Convert { name: buffer } object into an array of buffers.
        let mut binary_index: Option<usize> = None;
        let mut array: Vec<serde_json::Value> = Vec::with_capacity(object.len());
        for (_, value) in object.iter_mut() {
            if let Some(map) = value.as_object_mut() {
                if map.remove("_binarySource").is_some() {
                    binary_index = Some(array.len());
                }
                // Strip the placeholder extras; the bytes are attached after
                // deserialization since `data` is not part of the JSON schema.
                map.remove("extras");
            }
            array.push(value.clone());
        }
        gltf_value["buffers"] = serde_json::Value::Array(array);

        let mut gltf: GltfJson = serde_json::from_value(gltf_value).map_err(|e| {
            RuntimeError::new(Some(&format!(
                "Failed to convert binary glTF JSON to glTF 2.0 structure: {e}"
            )))
        })?;
        if let Some(index) = binary_index {
            if let Some(buffer) = gltf.buffers.get_mut(index) {
                buffer.data = Some(binary_buffer.to_vec());
                buffer.uri = None;
            }
        }
        // Remove the KHR_binary_glTF extension (removeExtensionsUsed)
        gltf.extensions_used
            .retain(|name| name != "KHR_binary_glTF");
        return Ok(gltf);
    }

    // The JSON already uses array-form buffers (unlikely for v1 but tolerated).
    let mut gltf: GltfJson = serde_json::from_value(gltf_value).map_err(|e| {
        RuntimeError::new(Some(&format!(
            "Failed to convert binary glTF JSON to glTF 2.0 structure: {e}"
        )))
    })?;
    if let Some(buffer) = gltf.buffers.first_mut() {
        buffer.data = Some(binary_buffer.to_vec());
    }
    Ok(gltf)
}

fn parse_glb_version2(glb: &[u8], header: &[u32]) -> Result<GltfJson, RuntimeError> {
    let length = header[2] as usize;
    let mut byte_offset = 12usize;
    let mut gltf: Option<GltfJson> = None;
    let mut binary_buffer: Option<&[u8]> = None;

    while byte_offset < length {
        let chunk_header = read_header(glb, byte_offset, 2)?;
        let chunk_length = chunk_header[0] as usize;
        let chunk_type = chunk_header[1];
        byte_offset += 8;
        let chunk_buffer = slice_or_err(glb, byte_offset, byte_offset + chunk_length)?;
        byte_offset += chunk_length;

        // Load JSON chunk
        if chunk_type == 0x4e4f534a {
            let json_string = get_string_from_typed_array(chunk_buffer, 0, chunk_buffer.len())?;
            let parsed: GltfJson = serde_json::from_str(json_string).map_err(|e| {
                RuntimeError::new(Some(&format!("Failed to parse binary glTF JSON: {e}")))
            })?;
            gltf = Some(parsed);
        }
        // Load Binary chunk
        else if chunk_type == 0x004e4942 {
            binary_buffer = Some(chunk_buffer);
        }
    }

    let mut gltf = gltf.ok_or_else(|| {
        RuntimeError::new(Some("Binary glTF JSON chunk is missing"))
    })?;

    if let Some(binary_buffer) = binary_buffer {
        if !gltf.buffers.is_empty() {
            gltf.buffers[0].data = Some(binary_buffer.to_vec());
        }
    }

    Ok(gltf)
}

fn slice_or_err<'a>(glb: &'a [u8], start: usize, end: usize) -> Result<&'a [u8], RuntimeError> {
    if end > glb.len() || start > end {
        return Err(RuntimeError::new(Some(
            "Invalid binary glTF: chunk exceeds buffer length",
        )));
    }
    Ok(&glb[start..end])
}

/// Rust analogue of `Core/getStringFromTypedArray.js` (UTF-8 decode).
fn get_string_from_typed_array<'a>(
    uint8_array: &'a [u8],
    byte_offset: usize,
    byte_length: usize,
) -> Result<&'a str, RuntimeError> {
    let end = byte_offset + byte_length;
    if end > uint8_array.len() {
        return Err(RuntimeError::new(Some(
            "Invalid binary glTF: string exceeds buffer length",
        )));
    }
    std::str::from_utf8(&uint8_array[byte_offset..end]).map_err(|e| {
        RuntimeError::new(Some(&format!("Failed to decode binary glTF string: {e}")))
    })
}
