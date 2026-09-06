//! Scene fidelity specs for the fifth batch of substantiated Scene types:
//! - find_meshopt_extension
//! - get_clip_and_style_code
//! - find_content_metadata / find_tile_metadata / find_group_metadata
//! - ContentMetadata / TileMetadata / GroupMetadata
//! - get_binary_accessor
//! - get_mesh_primitives
//! - ModelType (expanded)

use cesium_scene::content_metadata::ContentMetadata;
use cesium_scene::find_content_metadata::find_content_metadata;
use cesium_scene::find_group_metadata::find_group_metadata;
use cesium_scene::find_meshopt_extension::find_meshopt_extension;
use cesium_scene::find_tile_metadata::find_tile_metadata;
use cesium_scene::get_binary_accessor::{components_per_attribute, get_binary_accessor};
use cesium_scene::get_clip_and_style_code::get_clip_and_style_code;
use cesium_scene::get_mesh_primitives::get_mesh_primitives;
use cesium_scene::group_metadata::GroupMetadata;
use cesium_scene::model::cartesian_rectangle::CartesianRectangle;
use cesium_scene::model::imagery_input::ImageryInput;
use cesium_scene::model::mapped_positions::MappedPositions;
use cesium_scene::model::model_type::ModelType;
use cesium_scene::tile_metadata::TileMetadata;

// ─── find_meshopt_extension ────────────────────────────────────

#[test]
fn find_meshopt_extension_prefers_khr() {
    let obj = serde_json::json!({
        "extensions": {
            "KHR_meshopt_compression": {"mode": 1},
            "EXT_meshopt_compression": {"mode": 2}
        }
    });
    let result = find_meshopt_extension(&obj);
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().get("mode").unwrap().as_i64(),
        Some(1)
    );
}

#[test]
fn find_meshopt_extension_falls_back_to_ext() {
    let obj = serde_json::json!({
        "extensions": {
            "EXT_meshopt_compression": {"mode": 2}
        }
    });
    let result = find_meshopt_extension(&obj);
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().get("mode").unwrap().as_i64(),
        Some(2)
    );
}

#[test]
fn find_meshopt_extension_returns_none_when_missing() {
    let obj = serde_json::json!({"attributes": []});
    assert!(find_meshopt_extension(&obj).is_none());
}

// ─── get_clip_and_style_code ───────────────────────────────────

#[test]
fn get_clip_and_style_code_contains_uniform_names() {
    let code = get_clip_and_style_code("uSampler", "uMatrix", "uStyle");
    assert!(code.contains("uSampler"));
    assert!(code.contains("uMatrix"));
    assert!(code.contains("uStyle"));
    assert!(code.contains("clipDistance"));
    assert!(code.contains("clippingPlanesEdgeColor"));
    assert!(code.contains("out_FragColor"));
}

// ─── ContentMetadata ───────────────────────────────────────────

#[test]
fn content_metadata_new_stores_properties() {
    let content = serde_json::json!({
        "properties": {"height": 10.0, "name": "Building"},
        "extras": {"note": "test"},
        "extensions": {"ext1": {}}
    });
    let class = serde_json::json!({"properties": {}});
    let meta = ContentMetadata::new(&content, class);

    assert!(meta.has_property("height"));
    assert!(meta.has_property("name"));
    assert!(!meta.has_property("missing"));
    assert_eq!(
        meta.get_property("height").unwrap().as_f64(),
        Some(10.0)
    );
    assert!(meta.extras().is_some());
    assert!(meta.extensions().is_some());
}

#[test]
fn content_metadata_get_property_ids() {
    let content = serde_json::json!({
        "properties": {"a": 1, "b": 2}
    });
    let meta = ContentMetadata::new(&content, serde_json::json!({}));
    let ids = meta.get_property_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"a".to_string()));
    assert!(ids.contains(&"b".to_string()));
}

// ─── TileMetadata ──────────────────────────────────────────────

#[test]
fn tile_metadata_new_stores_properties() {
    let tile = serde_json::json!({
        "properties": {"area": 100.0}
    });
    let class = serde_json::json!({"id": "TileClass"});
    let meta = TileMetadata::new(&tile, class);

    assert!(meta.has_property("area"));
    assert!(!meta.has_property("missing"));
    assert_eq!(meta.get_property("area").unwrap().as_f64(), Some(100.0));
}

// ─── GroupMetadata ─────────────────────────────────────────────

#[test]
fn group_metadata_new_stores_id_and_properties() {
    let group = serde_json::json!({
        "properties": {"category": "residential"},
        "extras": {"note": "group"}
    });
    let class = serde_json::json!({"id": "GroupClass"});
    let meta = GroupMetadata::new("group1".to_string(), &group, class);

    assert_eq!(meta.id(), "group1");
    assert!(meta.has_property("category"));
    assert_eq!(
        meta.get_property("category").unwrap().as_str(),
        Some("residential")
    );
    assert!(meta.extras().is_some());
}

// ─── find_content_metadata ─────────────────────────────────────

#[test]
fn find_content_metadata_from_extension() {
    let schema = serde_json::json!({
        "classes": {
            "Building": {"properties": {"height": {"type": "SCALAR"}}}
        }
    });
    let content = serde_json::json!({
        "extensions": {
            "3DTILES_metadata": {
                "class": "Building",
                "properties": {"height": 20.0}
            }
        }
    });

    let result = find_content_metadata(Some(&schema), &content);
    assert!(result.is_some());
    let meta = result.unwrap();
    assert!(meta.has_property("height"));
}

#[test]
fn find_content_metadata_from_direct_field() {
    let schema = serde_json::json!({
        "classes": {
            "Wall": {"properties": {}}
        }
    });
    let content = serde_json::json!({
        "metadata": {
            "class": "Wall",
            "properties": {"material": "brick"}
        }
    });

    let result = find_content_metadata(Some(&schema), &content);
    assert!(result.is_some());
    assert!(result.unwrap().has_property("material"));
}

#[test]
fn find_content_metadata_returns_none_for_missing_schema() {
    let content = serde_json::json!({"metadata": {"class": "X", "properties": {}}});
    assert!(find_content_metadata(None, &content).is_none());
}

#[test]
fn find_content_metadata_returns_none_for_missing_metadata() {
    let schema = serde_json::json!({"classes": {}});
    let content = serde_json::json!({"other": "data"});
    assert!(find_content_metadata(Some(&schema), &content).is_none());
}

// ─── find_tile_metadata ────────────────────────────────────────

#[test]
fn find_tile_metadata_from_extension() {
    let schema = serde_json::json!({
        "classes": {
            "Tile": {"properties": {"lod": {"type": "SCALAR"}}}
        }
    });
    let tile = serde_json::json!({
        "extensions": {
            "3DTILES_metadata": {
                "class": "Tile",
                "properties": {"lod": 3}
            }
        }
    });

    let result = find_tile_metadata(Some(&schema), &tile);
    assert!(result.is_some());
    assert!(result.unwrap().has_property("lod"));
}

#[test]
fn find_tile_metadata_returns_none_for_missing_class() {
    let schema = serde_json::json!({"classes": {}});
    let tile = serde_json::json!({
        "metadata": {"class": "Missing", "properties": {}}
    });
    assert!(find_tile_metadata(Some(&schema), &tile).is_none());
}

// ─── find_group_metadata ───────────────────────────────────────

#[test]
fn find_group_metadata_by_numeric_index() {
    let meta_ext = serde_json::json!({
        "groups": [
            {"properties": {"name": "group0"}},
            {"properties": {"name": "group1"}}
        ],
        "groupIds": ["g0", "g1"]
    });
    let content = serde_json::json!({
        "extensions": {
            "3DTILES_metadata": {"group": 1}
        }
    });

    let result = find_group_metadata(Some(&meta_ext), &content);
    assert!(result.is_some());
    let group = result.unwrap();
    assert_eq!(
        group.get("properties").unwrap().get("name").unwrap().as_str(),
        Some("group1")
    );
}

#[test]
fn find_group_metadata_by_string_id() {
    let meta_ext = serde_json::json!({
        "groups": [
            {"properties": {"name": "A"}},
            {"properties": {"name": "B"}}
        ],
        "groupIds": ["idA", "idB"]
    });
    let content = serde_json::json!({
        "extensions": {
            "3DTILES_metadata": {"group": "idB"}
        }
    });

    let result = find_group_metadata(Some(&meta_ext), &content);
    assert!(result.is_some());
    assert_eq!(
        result
            .unwrap()
            .get("properties")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str(),
        Some("B")
    );
}

#[test]
fn find_group_metadata_returns_none_when_missing() {
    let content = serde_json::json!({"other": "data"});
    assert!(find_group_metadata(None, &content).is_none());
}

// ─── get_binary_accessor ───────────────────────────────────────

#[test]
fn components_per_attribute_lookup() {
    assert_eq!(components_per_attribute("SCALAR"), Some(1));
    assert_eq!(components_per_attribute("VEC2"), Some(2));
    assert_eq!(components_per_attribute("VEC3"), Some(3));
    assert_eq!(components_per_attribute("VEC4"), Some(4));
    assert_eq!(components_per_attribute("MAT2"), Some(4));
    assert_eq!(components_per_attribute("MAT3"), Some(9));
    assert_eq!(components_per_attribute("MAT4"), Some(16));
    assert_eq!(components_per_attribute("INVALID"), None);
}

#[test]
fn get_binary_accessor_parses_vec3() {
    let accessor = serde_json::json!({"type": "VEC3", "componentType": 5126});
    let result = get_binary_accessor(&accessor).unwrap();
    assert_eq!(result.components_per_attribute, 3);
    assert_eq!(result.attribute_type, "VEC3");
}

#[test]
fn get_binary_accessor_returns_none_for_invalid() {
    let accessor = serde_json::json!({"type": "INVALID"});
    assert!(get_binary_accessor(&accessor).is_none());
}

// ─── get_mesh_primitives ───────────────────────────────────────

#[test]
fn get_mesh_primitives_returns_original_without_extension() {
    let mesh = serde_json::json!({
        "primitives": [
            {"attributes": {"POSITION": 0}, "mode": 4}
        ]
    });
    let result = get_mesh_primitives(&mesh);
    assert_eq!(result.len(), 1);
}

#[test]
fn get_mesh_primitives_returns_empty_for_no_primitives() {
    let mesh = serde_json::json!({"other": "data"});
    let result = get_mesh_primitives(&mesh);
    assert!(result.is_empty());
}

// ─── ModelType ─────────────────────────────────────────────────

#[test]
fn model_type_as_str_all_variants() {
    assert_eq!(ModelType::Gltf.as_str(), "GLTF");
    assert_eq!(ModelType::TileGltf.as_str(), "TILE_GLTF");
    assert_eq!(ModelType::TileB3dm.as_str(), "B3DM");
    assert_eq!(ModelType::TileI3dm.as_str(), "I3DM");
    assert_eq!(ModelType::TilePnts.as_str(), "PNTS");
    assert_eq!(ModelType::TileGeojson.as_str(), "TILE_GEOJSON");
}

#[test]
fn model_type_from_str_round_trips() {
    assert_eq!(ModelType::from_str("GLTF"), Some(ModelType::Gltf));
    assert_eq!(ModelType::from_str("B3DM"), Some(ModelType::TileB3dm));
    assert_eq!(ModelType::from_str("I3DM"), Some(ModelType::TileI3dm));
    assert_eq!(ModelType::from_str("PNTS"), Some(ModelType::TilePnts));
    assert_eq!(
        ModelType::from_str("TILE_GEOJSON"),
        Some(ModelType::TileGeojson)
    );
    assert_eq!(ModelType::from_str("INVALID"), None);
}

#[test]
fn model_type_is_3d_tiles() {
    assert!(!ModelType::Gltf.is_3d_tiles());
    assert!(ModelType::TileGltf.is_3d_tiles());
    assert!(ModelType::TileB3dm.is_3d_tiles());
    assert!(ModelType::TileI3dm.is_3d_tiles());
    assert!(ModelType::TilePnts.is_3d_tiles());
    assert!(ModelType::TileGeojson.is_3d_tiles());
}

// ─── CartesianRectangle ────────────────────────────────────────

#[test]
fn cartesian_rectangle_new_and_default() {
    let r = CartesianRectangle::new(1.0, 2.0, 3.0, 4.0);
    assert!((r.min_x - 1.0).abs() < f64::EPSILON);
    assert!((r.min_y - 2.0).abs() < f64::EPSILON);
    assert!((r.max_x - 3.0).abs() < f64::EPSILON);
    assert!((r.max_y - 4.0).abs() < f64::EPSILON);

    let d = CartesianRectangle::default();
    assert!((d.min_x - 0.0).abs() < f64::EPSILON);
}

#[test]
fn cartesian_rectangle_contains() {
    let r = CartesianRectangle::new(0.0, 0.0, 10.0, 10.0);
    // Default: includes min, excludes max
    assert!(r.contains(0.0, 0.0));
    assert!(r.contains(5.0, 5.0));
    assert!(!r.contains(10.0, 10.0)); // max excluded
    assert!(!r.contains(-1.0, 5.0));
}

#[test]
fn cartesian_rectangle_contains_exclusive() {
    let r = CartesianRectangle::new(0.0, 0.0, 10.0, 10.0);
    assert!(!r.contains_exclusive(0.0, 0.0)); // border excluded
    assert!(r.contains_exclusive(5.0, 5.0));
    assert!(!r.contains_exclusive(10.0, 10.0));
}

#[test]
fn cartesian_rectangle_contains_inclusive() {
    let r = CartesianRectangle::new(0.0, 0.0, 10.0, 10.0);
    assert!(r.contains_inclusive(0.0, 0.0));
    assert!(r.contains_inclusive(10.0, 10.0)); // border included
    assert!(!r.contains_inclusive(11.0, 5.0));
}

// ─── ImageryInput ──────────────────────────────────────────────

#[test]
fn imagery_input_stores_fields() {
    let input = ImageryInput::new(
        serde_json::json!("layer"),
        serde_json::json!("texture"),
        serde_json::json!([1.0, 2.0, 3.0, 4.0]),
        serde_json::json!([0.0, 0.0, 1.0, 1.0]),
        2,
    );
    assert_eq!(input.imagery_tex_coord_attribute_set_index, 2);
}

// ─── MappedPositions ───────────────────────────────────────────

#[test]
fn mapped_positions_stores_fields() {
    let mp = MappedPositions::new(
        serde_json::json!([[0.1, 0.2], [0.3, 0.4]]),
        2,
        serde_json::json!({"west": 0.1, "south": 0.2, "east": 0.3, "north": 0.4}),
        serde_json::json!("WGS84"),
    );
    assert_eq!(mp.num_positions, 2);
}
