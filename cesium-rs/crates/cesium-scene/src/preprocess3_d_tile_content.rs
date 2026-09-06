//! Ported from `packages/engine/Source/Scene/preprocess3DTileContent.js`.
//!
//! Preprocesses raw binary 3D Tiles content to determine its type and
//! extract either a binary payload or a parsed JSON payload.

use cesium_core::get_json_from_typed_array::get_json_from_typed_array;
use cesium_core::get_magic::get_magic;
use serde_json::Value;

use crate::cesium3_d_tile_content_type::Cesium3DTileContentType;

/// Result of preprocessing 3D tile content.
///
/// Mirrors CesiumJS `PreprocessedContent`:
/// - `content_type`: detected content type
/// - `binary_payload`: raw bytes for binary formats
/// - `json_payload`: parsed JSON for JSON formats
#[derive(Debug, Clone)]
pub struct PreprocessedContent {
    /// The detected content type.
    pub content_type: Cesium3DTileContentType,
    /// For binary formats, the raw payload (byte offset 0).
    pub binary_payload: Option<Vec<u8>>,
    /// For JSON formats, the parsed JSON object.
    pub json_payload: Option<Value>,
}

/// Preprocesses raw 3D Tiles content to determine its type.
///
/// Mirrors CesiumJS `preprocess3DTileContent(arrayBuffer)`:
/// 1. Reads the magic number from the first 4 bytes.
/// 2. For binary formats, returns the raw payload.
/// 3. For JSON formats, parses the JSON and inspects top-level keys
///    to distinguish tileset JSON / glTF / subtree / GeoJSON / voxel.
pub fn preprocess_3d_tile_content(array_buffer: &[u8]) -> Option<PreprocessedContent> {
    let magic = get_magic(array_buffer, Some(0));

    // JS: if magic === "glTF", treat as "glb" (binary glTF).
    let magic = if magic == "glTF" {
        "glb".to_string()
    } else {
        magic
    };

    // Try to match a known binary content type from the magic number.
    if let Some(content_type) = Cesium3DTileContentType::from_str(&magic) {
        if Cesium3DTileContentType::is_binary_format(content_type) {
            return Some(PreprocessedContent {
                content_type,
                binary_payload: Some(array_buffer.to_vec()),
                json_payload: None,
            });
        }
    }

    // Not a recognized binary format — try parsing as JSON.
    let json_str = get_json_from_typed_array(array_buffer, None, None);
    let json: Value = serde_json::from_str(&json_str).ok()?;

    // tileset.json: has "root" key.
    if json.get("root").is_some() {
        return Some(PreprocessedContent {
            content_type: Cesium3DTileContentType::ExternalTileset,
            binary_payload: None,
            json_payload: Some(json),
        });
    }

    // glTF JSON: has "asset" key (checked after tileset because both can have "asset").
    if json.get("asset").is_some() {
        return Some(PreprocessedContent {
            content_type: Cesium3DTileContentType::Gltf,
            binary_payload: None,
            json_payload: Some(json),
        });
    }

    // Subtree JSON: has "tileAvailability" key.
    if json.get("tileAvailability").is_some() {
        return Some(PreprocessedContent {
            content_type: Cesium3DTileContentType::ImplicitSubtreeJson,
            binary_payload: None,
            json_payload: Some(json),
        });
    }

    // GeoJSON: has "type" key.
    if json.get("type").is_some() {
        return Some(PreprocessedContent {
            content_type: Cesium3DTileContentType::GeoJson,
            binary_payload: None,
            json_payload: Some(json),
        });
    }

    // Voxel JSON: has "voxelTable" key.
    if json.get("voxelTable").is_some() {
        return Some(PreprocessedContent {
            content_type: Cesium3DTileContentType::VoxelJson,
            binary_payload: None,
            json_payload: Some(json),
        });
    }

    None
}
