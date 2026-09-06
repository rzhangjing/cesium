//! Ported from `packages/engine/Source/Scene/findGroupMetadata.js`.

use serde_json::Value;

/// Check if a content has metadata, either defined in its `metadata` field
/// (3D Tiles 1.1) or in the `3DTILES_metadata` extension. If so, look up
/// the group with the corresponding ID.
///
/// Mirrors `findGroupMetadata(tileset, contentHeader)`.
///
/// # Arguments
/// * `metadata_extension` - The tileset's metadata extension object (must
///   contain `groups` and `groupIds`).
/// * `content_header` - The JSON header for a `Cesium3DTileContent`.
pub fn find_group_metadata(
    metadata_extension: Option<&Value>,
    content_header: &Value,
) -> Option<Value> {
    let metadata_ext = metadata_extension?;
    let groups = metadata_ext.get("groups")?;

    // Get the group reference from extension or direct field
    let group = content_header
        .get("extensions")
        .and_then(|ext| ext.get("3DTILES_metadata"))
        .and_then(|meta| meta.get("group"))
        .or_else(|| content_header.get("group"));

    let group = group?;

    // If group is a numeric index, look up directly
    if let Some(index) = group.as_u64() {
        return groups.get(index as usize).cloned();
    }

    // If group is a string ID, find the matching index
    if let Some(group_id) = group.as_str() {
        let group_ids = metadata_ext.get("groupIds")?.as_array()?;
        let index = group_ids
            .iter()
            .position(|id| id.as_str() == Some(group_id))?;
        return groups.get(index).cloned();
    }

    None
}
