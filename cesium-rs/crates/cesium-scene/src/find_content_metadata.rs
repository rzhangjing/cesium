//! Ported from `packages/engine/Source/Scene/findContentMetadata.js`.

use serde_json::Value;

use crate::content_metadata::ContentMetadata;

/// Check if a content has metadata, either defined in its `metadata` field
/// (3D Tiles 1.1) or in the `3DTILES_metadata` extension. If defined, get
/// the content metadata with the corresponding class.
///
/// Mirrors `findContentMetadata(tileset, contentHeader)`.
///
/// # Arguments
/// * `schema` - The tileset schema (must contain `classes`).
/// * `content_header` - The JSON header for a `Cesium3DTileContent`.
pub fn find_content_metadata(
    schema: Option<&Value>,
    content_header: &Value,
) -> Option<ContentMetadata> {
    // Check for 3DTILES_metadata extension or direct metadata field
    let metadata_json = content_header
        .get("extensions")
        .and_then(|ext| ext.get("3DTILES_metadata"))
        .or_else(|| content_header.get("metadata"));

    let metadata_json = metadata_json?;

    let schema = schema?;
    let classes = schema.get("classes")?;

    let class_name = metadata_json.get("class")?.as_str()?;
    let content_class = classes.get(class_name)?;

    Some(ContentMetadata::new(metadata_json, content_class.clone()))
}
