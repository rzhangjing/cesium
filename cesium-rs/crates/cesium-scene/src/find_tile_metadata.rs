//! Ported from `packages/engine/Source/Scene/findTileMetadata.js`.

use serde_json::Value;

use crate::tile_metadata::TileMetadata;

/// Check if a tile has metadata, either defined in its `metadata` field
/// (3D Tiles 1.1) or in the `3DTILES_metadata` extension. If defined, get
/// the tile metadata with the corresponding class.
///
/// Mirrors `findTileMetadata(tileset, tileHeader)`.
///
/// # Arguments
/// * `schema` - The tileset schema (must contain `classes`).
/// * `tile_header` - The JSON header for a `Cesium3DTile`.
pub fn find_tile_metadata(
    schema: Option<&Value>,
    tile_header: &Value,
) -> Option<TileMetadata> {
    // Check for 3DTILES_metadata extension or direct metadata field
    let metadata_json = tile_header
        .get("extensions")
        .and_then(|ext| ext.get("3DTILES_metadata"))
        .or_else(|| tile_header.get("metadata"));

    let metadata_json = metadata_json?;

    let schema = schema?;
    let classes = schema.get("classes")?;

    let class_name = metadata_json.get("class")?.as_str()?;
    let tile_class = classes.get(class_name)?;

    Some(TileMetadata::new(metadata_json, tile_class.clone()))
}
