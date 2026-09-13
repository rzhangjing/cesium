//! Scene/Cesium3DTileBatchTable + FeatureTable → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/Cesium3DTileFeatureTable.js
//! - Scene/Cesium3DTileBatchTable.js
//! - Scene/BatchTableHierarchy.js
//!
//! A-class tests: ComponentType/AccessorType parsing, FeatureTable global/binary
//! properties, BatchTable JSON/binary get/set, BatchTableHierarchy class/parent/property.
//! C-class omitted: WebGL buffer upload, shader integration, picking.

use cesium_tileset::batch_table::{
    AccessorType, BatchTable, BatchTableHierarchy, ComponentType, FeatureTable,
};
use serde_json::json;

// === ComponentType ===

#[test]
fn component_type_byte_sizes() {
    assert_eq!(ComponentType::Int8.byte_size(), 1);
    assert_eq!(ComponentType::Uint8.byte_size(), 1);
    assert_eq!(ComponentType::Int16.byte_size(), 2);
    assert_eq!(ComponentType::Uint16.byte_size(), 2);
    assert_eq!(ComponentType::Int32.byte_size(), 4);
    assert_eq!(ComponentType::Uint32.byte_size(), 4);
    assert_eq!(ComponentType::Float32.byte_size(), 4);
    assert_eq!(ComponentType::Float64.byte_size(), 8);
}

#[test]
fn component_type_from_name() {
    assert_eq!(ComponentType::from_name("FLOAT"), Some(ComponentType::Float32));
    assert_eq!(ComponentType::from_name("FLOAT32"), Some(ComponentType::Float32));
    assert_eq!(ComponentType::from_name("DOUBLE"), Some(ComponentType::Float64));
    assert_eq!(ComponentType::from_name("UNSIGNED_BYTE"), Some(ComponentType::Uint8));
    assert_eq!(ComponentType::from_name("UINT8"), Some(ComponentType::Uint8));
    assert_eq!(ComponentType::from_name("SHORT"), Some(ComponentType::Int16));
    assert_eq!(ComponentType::from_name("UNSIGNED_SHORT"), Some(ComponentType::Uint16));
    assert_eq!(ComponentType::from_name("INT"), Some(ComponentType::Int32));
    assert_eq!(ComponentType::from_name("UNSIGNED_INT"), Some(ComponentType::Uint32));
    assert_eq!(ComponentType::from_name("INVALID"), None);
}

// === AccessorType ===

#[test]
fn accessor_type_component_count() {
    assert_eq!(AccessorType::Scalar.component_count(), 1);
    assert_eq!(AccessorType::Vec2.component_count(), 2);
    assert_eq!(AccessorType::Vec3.component_count(), 3);
    assert_eq!(AccessorType::Vec4.component_count(), 4);
}

#[test]
fn accessor_type_from_name() {
    assert_eq!(AccessorType::from_name("SCALAR"), Some(AccessorType::Scalar));
    assert_eq!(AccessorType::from_name("VEC2"), Some(AccessorType::Vec2));
    assert_eq!(AccessorType::from_name("VEC3"), Some(AccessorType::Vec3));
    assert_eq!(AccessorType::from_name("VEC4"), Some(AccessorType::Vec4));
    assert_eq!(AccessorType::from_name("MAT4"), None);
}

// === FeatureTable ===

fn make_feature_table() -> FeatureTable {
    // 3 points with binary POSITION data (3 * 3 floats = 36 bytes)
    let mut binary = Vec::new();
    // Point 0: (1.0, 2.0, 3.0)
    binary.extend_from_slice(&1.0f32.to_le_bytes());
    binary.extend_from_slice(&2.0f32.to_le_bytes());
    binary.extend_from_slice(&3.0f32.to_le_bytes());
    // Point 1: (4.0, 5.0, 6.0)
    binary.extend_from_slice(&4.0f32.to_le_bytes());
    binary.extend_from_slice(&5.0f32.to_le_bytes());
    binary.extend_from_slice(&6.0f32.to_le_bytes());
    // Point 2: (7.0, 8.0, 9.0)
    binary.extend_from_slice(&7.0f32.to_le_bytes());
    binary.extend_from_slice(&8.0f32.to_le_bytes());
    binary.extend_from_slice(&9.0f32.to_le_bytes());

    let json = json!({
        "POINTS_LENGTH": 3,
        "RTC_CENTER": [100.0, 200.0, 300.0],
        "POSITION": {
            "byteOffset": 0
        }
    });

    FeatureTable::new(Some(json), binary)
}

#[test]
fn feature_table_features_length() {
    let ft = make_feature_table();
    assert_eq!(ft.features_length, 3);
}

#[test]
fn feature_table_features_length_batch() {
    let json = json!({ "BATCH_LENGTH": 10 });
    let ft = FeatureTable::new(Some(json), vec![]);
    assert_eq!(ft.features_length, 10);
}

#[test]
fn feature_table_features_length_instances() {
    let json = json!({ "INSTANCES_LENGTH": 5 });
    let ft = FeatureTable::new(Some(json), vec![]);
    assert_eq!(ft.features_length, 5);
}

#[test]
fn feature_table_has_property() {
    let ft = make_feature_table();
    assert!(ft.has_property("POSITION"));
    assert!(ft.has_property("RTC_CENTER"));
    assert!(!ft.has_property("NORMAL"));
}

#[test]
fn feature_table_global_u32() {
    let ft = make_feature_table();
    assert_eq!(ft.get_global_u32("POINTS_LENGTH"), Some(3));
    assert_eq!(ft.get_global_u32("NONEXISTENT"), None);
}

#[test]
fn feature_table_global_f64() {
    let json = json!({ "POINTS_LENGTH": 1, "SCALE": 2.5 });
    let ft = FeatureTable::new(Some(json), vec![]);
    assert_eq!(ft.get_global_f64("SCALE"), Some(2.5));
}

#[test]
fn feature_table_global_vec3() {
    let ft = make_feature_table();
    let rtc = ft.get_global_vec3("RTC_CENTER").unwrap();
    assert!((rtc[0] - 100.0).abs() < 1e-10);
    assert!((rtc[1] - 200.0).abs() < 1e-10);
    assert!((rtc[2] - 300.0).abs() < 1e-10);
}

#[test]
fn feature_table_binary_ref() {
    let ft = make_feature_table();
    let bin_ref = ft.get_binary_ref("POSITION").unwrap();
    assert_eq!(bin_ref.byte_offset, 0);
}

#[test]
fn feature_table_read_f32_array() {
    let ft = make_feature_table();
    let values = ft.read_f32_array(0, 9).unwrap();
    assert_eq!(values.len(), 9);
    assert!((values[0] - 1.0).abs() < 1e-6);
    assert!((values[4] - 5.0).abs() < 1e-6);
    assert!((values[8] - 9.0).abs() < 1e-6);
}

#[test]
fn feature_table_read_f32_array_out_of_bounds() {
    let ft = make_feature_table();
    assert!(ft.read_f32_array(0, 100).is_none());
}

#[test]
fn feature_table_get_positions() {
    let ft = make_feature_table();
    let positions = ft.get_positions().unwrap();
    assert_eq!(positions.len(), 3);
    assert!((positions[0][0] - 1.0).abs() < 1e-6);
    assert!((positions[0][1] - 2.0).abs() < 1e-6);
    assert!((positions[0][2] - 3.0).abs() < 1e-6);
    assert!((positions[2][0] - 7.0).abs() < 1e-6);
}

#[test]
fn feature_table_null_json() {
    let ft = FeatureTable::new(None, vec![]);
    assert_eq!(ft.features_length, 0);
    assert!(!ft.has_property("POSITION"));
}

// === BatchTable ===

fn make_batch_table() -> BatchTable {
    let json = json!({
        "height": [10.0, 20.0, 30.0],
        "name": ["Building A", "Building B", "Building C"],
        "area": {
            "byteOffset": 0,
            "componentType": "FLOAT",
            "type": "SCALAR"
        }
    });

    // Binary: 3 f32 values for area
    let mut binary = Vec::new();
    binary.extend_from_slice(&100.5f32.to_le_bytes());
    binary.extend_from_slice(&200.5f32.to_le_bytes());
    binary.extend_from_slice(&300.5f32.to_le_bytes());

    BatchTable::new(Some(json), binary, 3)
}

#[test]
fn batch_table_property_names() {
    let bt = make_batch_table();
    let mut names = bt.property_names();
    names.sort();
    assert_eq!(names, vec!["area", "height", "name"]);
}

#[test]
fn batch_table_has_property() {
    let bt = make_batch_table();
    assert!(bt.has_property("height"));
    assert!(bt.has_property("name"));
    assert!(bt.has_property("area"));
    assert!(!bt.has_property("nonexistent"));
}

#[test]
fn batch_table_get_json_property() {
    let bt = make_batch_table();
    assert_eq!(bt.get_property("height", 0), Some(json!(10.0)));
    assert_eq!(bt.get_property("height", 2), Some(json!(30.0)));
    assert_eq!(bt.get_property("name", 1), Some(json!("Building B")));
}

#[test]
fn batch_table_get_property_out_of_range() {
    let bt = make_batch_table();
    assert_eq!(bt.get_property("height", 99), None);
}

#[test]
fn batch_table_get_binary_property() {
    let bt = make_batch_table();
    let val = bt.get_property("area", 0).unwrap();
    assert!((val.as_f64().unwrap() - 100.5).abs() < 0.01);

    let val2 = bt.get_property("area", 2).unwrap();
    assert!((val2.as_f64().unwrap() - 300.5).abs() < 0.01);
}

#[test]
fn batch_table_get_property_all_json() {
    let bt = make_batch_table();
    let all = bt.get_property_all("height").unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0], json!(10.0));
    assert_eq!(all[1], json!(20.0));
    assert_eq!(all[2], json!(30.0));
}

#[test]
fn batch_table_get_property_all_binary() {
    let bt = make_batch_table();
    let all = bt.get_property_all("area").unwrap();
    assert_eq!(all.len(), 3);
    assert!((all[0].as_f64().unwrap() - 100.5).abs() < 0.01);
}

#[test]
fn batch_table_set_property() {
    let mut bt = make_batch_table();
    assert!(bt.set_property("height", 1, json!(99.0)));
    assert_eq!(bt.get_property("height", 1), Some(json!(99.0)));
}

#[test]
fn batch_table_set_property_out_of_range() {
    let mut bt = make_batch_table();
    assert!(!bt.set_property("height", 99, json!(0.0)));
}

#[test]
fn batch_table_byte_length() {
    let bt = make_batch_table();
    assert_eq!(bt.byte_length(), 12); // 3 * 4 bytes
}

#[test]
fn batch_table_empty() {
    let bt = BatchTable::new(None, vec![], 0);
    assert_eq!(bt.features_length, 0);
    assert!(bt.property_names().is_empty());
}

// === BatchTableHierarchy ===

fn make_hierarchy_json() -> serde_json::Value {
    json!({
        "classes": [
            {
                "name": "Building",
                "length": 2,
                "properties": {
                    "height": [10.0, 20.0],
                    "address": ["123 Main St", "456 Oak Ave"]
                }
            },
            {
                "name": "Floor",
                "length": 3,
                "properties": {
                    "level": [1, 2, 3]
                }
            }
        ],
        "instancesLength": 5,
        "classIds": [0, 0, 1, 1, 1],
        "parentIds": [4294967295u32, 4294967295u32, 0, 0, 1]
    })
}

#[test]
fn hierarchy_from_json() {
    let json = make_hierarchy_json();
    let hierarchy = BatchTableHierarchy::from_json(&json, &[]).unwrap();
    assert_eq!(hierarchy.instances_length, 5);
    assert_eq!(hierarchy.classes.len(), 2);
    assert_eq!(hierarchy.classes[0].name, "Building");
    assert_eq!(hierarchy.classes[0].length, 2);
    assert_eq!(hierarchy.classes[1].name, "Floor");
    assert_eq!(hierarchy.classes[1].length, 3);
}

#[test]
fn hierarchy_class_ids() {
    let json = make_hierarchy_json();
    let hierarchy = BatchTableHierarchy::from_json(&json, &[]).unwrap();
    assert_eq!(hierarchy.get_class_id(0), Some(0)); // Building
    assert_eq!(hierarchy.get_class_id(2), Some(1)); // Floor
    assert_eq!(hierarchy.get_class_id(4), Some(1)); // Floor
}

#[test]
fn hierarchy_parent_ids() {
    let json = make_hierarchy_json();
    let hierarchy = BatchTableHierarchy::from_json(&json, &[]).unwrap();
    assert_eq!(hierarchy.get_parent_id(0), Some(u32::MAX)); // no parent
    assert_eq!(hierarchy.get_parent_id(2), Some(0)); // parent is instance 0
    assert_eq!(hierarchy.get_parent_id(4), Some(1)); // parent is instance 1
}

#[test]
fn hierarchy_get_property() {
    let json = make_hierarchy_json();
    let hierarchy = BatchTableHierarchy::from_json(&json, &[]).unwrap();

    // Instance 0 is Building class (index 0 in Building class)
    let height = hierarchy.get_property(0, "height").unwrap();
    assert_eq!(height, json!(10.0));

    // Instance 1 is Building class (index 1 in Building class)
    let height1 = hierarchy.get_property(1, "height").unwrap();
    assert_eq!(height1, json!(20.0));

    // Instance 2 is Floor class (index 0 in Floor class)
    let level = hierarchy.get_property(2, "level").unwrap();
    assert_eq!(level, json!(1));
}

#[test]
fn hierarchy_class_name() {
    let json = make_hierarchy_json();
    let hierarchy = BatchTableHierarchy::from_json(&json, &[]).unwrap();
    assert_eq!(hierarchy.get_class_name(0), Some("Building"));
    assert_eq!(hierarchy.get_class_name(3), Some("Floor"));
}
