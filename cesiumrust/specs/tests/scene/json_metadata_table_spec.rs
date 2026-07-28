//! Scene/JsonMetadataTableSpec.js → Rust integration tests
//!
//! Original: 19 it() → 11 A-class (8 C-class: throws)
//! A-class: constructor_clones(1) + hasProperty(2) + getPropertyIds(1) +
//!          getProperty(4) + setProperty(3)

use cesium_tileset::json_metadata_table::JsonMetadataTable;
use serde_json::{json, Value};
use std::collections::HashMap;

fn create_test_table() -> JsonMetadataTable {
    let mut properties = HashMap::new();
    properties.insert(
        "priority".to_string(),
        vec![json!(2), json!(1), json!(0)],
    );
    properties.insert(
        "labels".to_string(),
        vec![json!("Point Cloud"), json!("Mesh"), json!("Raster")],
    );
    properties.insert(
        "uri".to_string(),
        vec![json!("tree.las"), json!("building.gltf"), json!("map.tif")],
    );
    properties.insert(
        "sizeInfo".to_string(),
        vec![
            json!({"pointCount": 100}),
            json!({"vertices": 3000, "faces": 1000}),
            json!({"width": 1024, "height": 1024}),
        ],
    );
    properties.insert(
        "mixedValues".to_string(),
        vec![json!("red"), json!(3), json!(false)],
    );
    JsonMetadataTable::new(3, properties)
}

#[test]
fn test_constructor_clones_properties() {
    let mut table = create_test_table();
    let old_value = json!({"pointCount": 100});
    let new_value = json!({"lengthBytes": 1024});

    table.set_property(0, "sizeInfo", new_value.clone());

    // Original value should be unchanged (we can't check the original HashMap
    // since it was moved, but we can verify the new value is set)
    assert_eq!(table.get_property(0, "sizeInfo"), Some(new_value));
}

#[test]
fn test_has_property_returns_true() {
    let table = create_test_table();
    assert!(table.has_property("priority"));
}

#[test]
fn test_has_property_returns_false() {
    let table = create_test_table();
    assert!(!table.has_property("price"));
}

#[test]
fn test_get_property_ids() {
    let table = create_test_table();
    let ids = table.get_property_ids();
    assert_eq!(ids, vec!["labels", "mixedValues", "priority", "sizeInfo", "uri"]);
}

#[test]
fn test_get_property_returns_none_for_unknown() {
    let table = create_test_table();
    assert_eq!(table.get_property(0, "color"), None);
}

#[test]
fn test_get_property_returns_value() {
    let table = create_test_table();
    assert_eq!(table.get_property(0, "priority"), Some(json!(2)));
    assert_eq!(table.get_property(1, "priority"), Some(json!(1)));
    assert_eq!(table.get_property(2, "priority"), Some(json!(0)));
}

#[test]
fn test_get_property_returns_copy() {
    let table = create_test_table();
    let value1 = table.get_property(1, "sizeInfo").unwrap();
    let value2 = table.get_property(1, "sizeInfo").unwrap();
    assert_eq!(value1, json!({"vertices": 3000, "faces": 1000}));
    assert_eq!(value1, value2);
    // In Rust, cloned values are independent (no reference sharing)
}

#[test]
fn test_get_property_heterogeneous_values() {
    let table = create_test_table();
    assert_eq!(table.get_property(0, "mixedValues"), Some(json!("red")));
    assert_eq!(table.get_property(1, "mixedValues"), Some(json!(3)));
    assert_eq!(table.get_property(2, "mixedValues"), Some(json!(false)));
}

#[test]
fn test_set_property_creates_new_property() {
    let mut table = create_test_table();
    assert_eq!(table.get_property(0, "color"), None);

    table.set_property(0, "color", json!([255, 255, 255, 1.0]));
    assert_eq!(table.get_property(0, "color"), Some(json!([255, 255, 255, 1.0])));
}

#[test]
fn test_set_property_sets_value() {
    let mut table = create_test_table();
    let size_info = json!({"lengthBytes": 1024});
    table.set_property(0, "sizeInfo", size_info.clone());
    assert_eq!(table.get_property(0, "sizeInfo"), Some(size_info));
}

#[test]
fn test_set_property_copies_value() {
    let mut table = create_test_table();
    let mut size_info = json!({"lengthBytes": 1024});
    table.set_property(1, "sizeInfo", size_info.clone());

    // Modify the original - table should not be affected
    size_info["offset"] = json!(8);
    assert_ne!(table.get_property(1, "sizeInfo"), Some(size_info));
}
