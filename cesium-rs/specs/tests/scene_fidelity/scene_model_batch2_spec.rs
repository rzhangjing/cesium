//! Scene Model batch 2 spec tests.
//!
//! Mirrors CesiumJS Jasmine specs for Scene-level types:
//! - has_extension (extension checking utility)
//! - KeyframeNode / KeyframeLoadState
//! - preprocess_3d_tile_content (content type detection)
//! - MetadataTableProperty / MetadataTable / JsonMetadataTable
//! - BatchTable / BatchTableHierarchy / PropertyTable
//! - ModelRuntimeNode
//! - ModelComponents
//! - PrimitiveLoadPlan / AttributeLoadPlan / IndicesLoadPlan

use serde_json::json;

use cesium_scene::has_extension::has_extension;
use cesium_scene::keyframe_node::{KeyframeLoadState, KeyframeNode};
use cesium_scene::preprocess3_d_tile_content::preprocess_3d_tile_content;
use cesium_scene::cesium3_d_tile_content_type::Cesium3DTileContentType;
use cesium_scene::metadata_table_property::MetadataTableProperty;
use cesium_scene::metadata_table::MetadataTable;
use cesium_scene::json_metadata_table::JsonMetadataTable;
use cesium_scene::batch_table::BatchTable;
use cesium_scene::batch_table_hierarchy::BatchTableHierarchy;
use cesium_scene::property_table::PropertyTable;
use cesium_scene::model::model_runtime_node::ModelRuntimeNode;
use cesium_scene::model_components::ModelComponents;
use cesium_scene::primitive_load_plan::{AttributeLoadPlan, IndicesLoadPlan, PrimitiveLoadPlan};

use cesium_core::matrix4::Matrix4;

// ==================== has_extension ====================

#[test]
fn has_extension_true_when_present() {
    let j = json!({"extensions": {"3DTILES_metadata": {}}});
    assert!(has_extension(&j, "3DTILES_metadata"));
}

#[test]
fn has_extension_false_when_absent() {
    let j = json!({"extensions": {}});
    assert!(!has_extension(&j, "3DTILES_metadata"));
}

#[test]
fn has_extension_false_no_extensions_key() {
    let j = json!({"name": "test"});
    assert!(!has_extension(&j, "anything"));
}

#[test]
fn has_extension_false_for_null() {
    assert!(!has_extension(&serde_json::Value::Null, "x"));
}

// ==================== KeyframeNode ====================

#[test]
fn keyframe_node_new_defaults() {
    let node = KeyframeNode::new(5, 10);
    assert_eq!(node.spatial_node_index, 5);
    assert_eq!(node.keyframe, 10);
    assert_eq!(node.state, KeyframeLoadState::Unloaded);
    assert!(node.content.is_none());
    assert_eq!(node.megatexture_index, -1);
    assert_eq!(node.priority, f64::MIN);
    assert_eq!(node.high_priority_frame_number, -1);
}

#[test]
fn keyframe_node_unload_resets() {
    let mut node = KeyframeNode::new(0, 0);
    node.state = KeyframeLoadState::Loaded;
    node.content = Some(42);
    node.megatexture_index = 5;
    node.priority = 100.0;
    node.high_priority_frame_number = 10;

    node.unload();
    assert_eq!(node.state, KeyframeLoadState::Unloaded);
    assert!(node.content.is_none());
    assert_eq!(node.megatexture_index, -1);
    assert_eq!(node.priority, f64::MIN);
    assert_eq!(node.high_priority_frame_number, -1);
}

#[test]
fn keyframe_load_state_from_i32() {
    assert_eq!(KeyframeLoadState::from_i32(0), Some(KeyframeLoadState::Unloaded));
    assert_eq!(KeyframeLoadState::from_i32(3), Some(KeyframeLoadState::Loaded));
    assert_eq!(KeyframeLoadState::from_i32(5), Some(KeyframeLoadState::Unavailable));
    assert_eq!(KeyframeLoadState::from_i32(99), None);
}

#[test]
fn keyframe_node_priority_compare() {
    let mut a = KeyframeNode::new(0, 0);
    let mut b = KeyframeNode::new(0, 1);
    a.priority = 1.0;
    b.priority = 5.0;
    assert_eq!(
        KeyframeNode::priority_compare(&a, &b),
        std::cmp::Ordering::Less
    );
}

#[test]
fn keyframe_node_search_compare() {
    let a = KeyframeNode::new(0, 3);
    let b = KeyframeNode::new(0, 7);
    assert_eq!(
        KeyframeNode::search_compare(&a, &b),
        std::cmp::Ordering::Less
    );
}

// ==================== preprocess_3d_tile_content ====================

#[test]
fn preprocess_detects_b3dm_binary() {
    // b3dm magic number
    let mut data = vec![0u8; 32];
    data[0] = b'b'; data[1] = b'3'; data[2] = b'd'; data[3] = b'm';
    let result = preprocess_3d_tile_content(&data).unwrap();
    assert_eq!(result.content_type, Cesium3DTileContentType::Batched3DModel);
    assert!(result.binary_payload.is_some());
    assert!(result.json_payload.is_none());
}

#[test]
fn preprocess_detects_gltf_binary() {
    // glTF magic → treated as glb
    let mut data = vec![0u8; 32];
    data[0] = b'g'; data[1] = b'l'; data[2] = b'T'; data[3] = b'F';
    let result = preprocess_3d_tile_content(&data).unwrap();
    assert_eq!(result.content_type, Cesium3DTileContentType::GltfBinary);
    assert!(result.binary_payload.is_some());
}

#[test]
fn preprocess_detects_json_tileset() {
    let json_str = r#"{"root": {"content": {"uri": "tile.b3dm"}}}"#;
    let data = json_str.as_bytes();
    let result = preprocess_3d_tile_content(data).unwrap();
    assert_eq!(result.content_type, Cesium3DTileContentType::ExternalTileset);
    assert!(result.json_payload.is_some());
}

#[test]
fn preprocess_detects_json_gltf() {
    let json_str = r#"{"asset": {"version": "2.0"}}"#;
    let data = json_str.as_bytes();
    let result = preprocess_3d_tile_content(data).unwrap();
    assert_eq!(result.content_type, Cesium3DTileContentType::Gltf);
    assert!(result.json_payload.is_some());
}

#[test]
fn preprocess_detects_geojson() {
    let json_str = r#"{"type": "FeatureCollection", "features": []}"#;
    let data = json_str.as_bytes();
    let result = preprocess_3d_tile_content(data).unwrap();
    assert_eq!(result.content_type, Cesium3DTileContentType::GeoJson);
}

// ==================== MetadataTableProperty ====================

#[test]
fn metadata_table_property_new() {
    let prop = MetadataTableProperty::new("height", "SCALAR", 100);
    assert_eq!(prop.property_id, "height");
    assert_eq!(prop.property_type, "SCALAR");
    assert_eq!(prop.count, 100);
    assert!(!prop.has_buffer_data());
}

#[test]
fn metadata_table_property_with_buffer() {
    let mut prop = MetadataTableProperty::new("position", "VEC3", 50);
    prop.buffer_view_index = Some(0);
    prop.byte_stride = 12;
    prop.component_count = 3;
    assert!(prop.has_buffer_data());
    assert_eq!(prop.component_count, 3);
}

// ==================== MetadataTable ====================

#[test]
fn metadata_table_new() {
    let table = MetadataTable::new(100, "building");
    assert_eq!(table.count(), 100);
    assert_eq!(table.class_name, "building");
    assert_eq!(table.properties_length(), 0);
}

#[test]
fn metadata_table_add_and_get_property() {
    let mut table = MetadataTable::new(10, "test");
    let prop = MetadataTableProperty::new("name", "STRING", 10);
    table.properties.insert("name".to_string(), prop);
    assert!(table.has_property("name"));
    assert!(!table.has_property("missing"));
    assert_eq!(table.get_property("name").unwrap().property_type, "STRING");
}

// ==================== JsonMetadataTable ====================

#[test]
fn json_metadata_table_new() {
    let table = JsonMetadataTable::new(5);
    assert_eq!(table.count(), 5);
    assert_eq!(table.properties_length(), 0);
}

#[test]
fn json_metadata_table_get_property() {
    let mut table = JsonMetadataTable::new(3);
    table.properties.insert("name".to_string(), json!(["a", "b", "c"]));
    assert_eq!(table.get_property("name", 1).unwrap(), &json!("b"));
    assert!(table.get_property("name", 5).is_none());
    assert!(table.get_property("missing", 0).is_none());
}

// ==================== BatchTable ====================

#[test]
fn batch_table_new() {
    let bt = BatchTable::new(50);
    assert_eq!(bt.features_length(), 50);
    assert!(bt.metadata_table.is_none());
    assert!(bt.json_metadata_table.is_none());
}

#[test]
fn batch_table_get_property_from_json() {
    let mut bt = BatchTable::new(2);
    let mut jmt = JsonMetadataTable::new(2);
    jmt.properties.insert("id".to_string(), json!([10, 20]));
    bt.json_metadata_table = Some(jmt);

    assert_eq!(bt.get_property(0, "id"), Some(json!(10)));
    assert_eq!(bt.get_property(1, "id"), Some(json!(20)));
    assert!(bt.has_property("id"));
    assert!(!bt.has_property("missing"));
}

// ==================== BatchTableHierarchy ====================

#[test]
fn batch_table_hierarchy_add_class() {
    let mut bth = BatchTableHierarchy::new();
    let idx = bth.add_class("building", 10, vec!["name".to_string(), "height".to_string()]);
    assert_eq!(idx, 0);
    assert_eq!(bth.classes_length(), 1);
    let class = bth.get_class(0).unwrap();
    assert_eq!(class.name, "building");
    assert_eq!(class.length, 10);
    assert_eq!(class.property_ids.len(), 2);
}

#[test]
fn batch_table_hierarchy_get_property() {
    let mut bth = BatchTableHierarchy::new();
    bth.class_ids = vec![0, 0];
    let mut inst0 = std::collections::HashMap::new();
    inst0.insert("name".to_string(), json!("Tower"));
    bth.instances.push(inst0);
    bth.instances.push(std::collections::HashMap::new());

    assert_eq!(bth.get_property(0, "name"), Some(&json!("Tower")));
    assert!(bth.get_property(1, "name").is_none());
    assert_eq!(bth.class_id(0), Some(0));
}

// ==================== PropertyTable ====================

#[test]
fn property_table_new() {
    let pt = PropertyTable::new(100);
    assert_eq!(pt.count(), 100);
    assert_eq!(pt.name, "");
    assert!(pt.metadata_table.is_none());
}

#[test]
fn property_table_get_property_from_json() {
    let mut pt = PropertyTable::new(2);
    let mut jmt = JsonMetadataTable::new(2);
    jmt.properties.insert("color".to_string(), json!(["red", "blue"]));
    pt.json_metadata_table = Some(jmt);

    assert_eq!(pt.get_property(0, "color"), Some(json!("red")));
    assert!(pt.has_property("color"));
    assert!(!pt.has_property("missing"));
}

#[test]
fn property_table_property_ids_merged() {
    let mut pt = PropertyTable::new(1);
    let mut jmt = JsonMetadataTable::new(1);
    jmt.properties.insert("a".to_string(), json!([1]));
    jmt.properties.insert("b".to_string(), json!([2]));
    pt.json_metadata_table = Some(jmt);

    let ids = pt.property_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"a".to_string()));
    assert!(ids.contains(&"b".to_string()));
}

// ==================== ModelRuntimeNode ====================

#[test]
fn model_runtime_node_new() {
    let node = ModelRuntimeNode::new(5);
    assert_eq!(node.node_index, 5);
    assert_eq!(node.local_matrix, Matrix4::IDENTITY);
    assert_eq!(node.world_matrix, Matrix4::IDENTITY);
    assert_eq!(node.skin_index, -1);
    assert!(node.show);
    assert!(!node.computed);
    assert!(!node.is_skinned());
}

#[test]
fn model_runtime_node_skinned() {
    let mut node = ModelRuntimeNode::new(0);
    node.skin_index = 3;
    assert!(node.is_skinned());
}

#[test]
fn model_runtime_node_mark_dirty() {
    let mut node = ModelRuntimeNode::new(0);
    node.computed = true;
    node.mark_dirty();
    assert!(!node.computed);
}

// ==================== ModelComponents ====================

#[test]
fn model_components_new() {
    let mc = ModelComponents::new();
    assert_eq!(mc.nodes_length, 0);
    assert_eq!(mc.meshes_length, 0);
    assert_eq!(mc.total_resources(), 0);
    assert_eq!(mc.up_axis, 2); // Z_UP
    assert!(mc.root_nodes.is_empty());
}

#[test]
fn model_components_total_resources() {
    let mut mc = ModelComponents::new();
    mc.nodes_length = 10;
    mc.meshes_length = 5;
    mc.materials_length = 3;
    assert_eq!(mc.total_resources(), 18);
}

// ==================== PrimitiveLoadPlan ====================

#[test]
fn primitive_load_plan_new() {
    let plan = PrimitiveLoadPlan::new(4);
    assert_eq!(plan.primitive_type, 4);
    assert_eq!(plan.attributes_length(), 0);
    assert!(!plan.is_indexed());
}

#[test]
fn primitive_load_plan_add_attributes() {
    let mut plan = PrimitiveLoadPlan::new(4);
    plan.add_attribute_plan(AttributeLoadPlan::new("POSITION"));
    plan.add_attribute_plan(AttributeLoadPlan::new("NORMAL"));
    assert_eq!(plan.attributes_length(), 2);
    assert_eq!(plan.attribute_plans[0].attribute_name, "POSITION");
}

#[test]
fn primitive_load_plan_with_indices() {
    let mut plan = PrimitiveLoadPlan::new(4);
    plan.indices_plan = Some(IndicesLoadPlan::new(36, "UNSIGNED_SHORT"));
    assert!(plan.is_indexed());
    assert_eq!(plan.indices_plan.as_ref().unwrap().count, 36);
}

#[test]
fn attribute_load_plan_defaults() {
    let al = AttributeLoadPlan::new("TEXCOORD_0");
    assert_eq!(al.attribute_name, "TEXCOORD_0");
    assert!(!al.load_buffer);
    assert!(!al.load_typed_array);
}

#[test]
fn indices_load_plan_defaults() {
    let il = IndicesLoadPlan::new(100, "UNSIGNED_INT");
    assert_eq!(il.count, 100);
    assert_eq!(il.component_type, "UNSIGNED_INT");
    assert!(!il.load_buffer);
}
