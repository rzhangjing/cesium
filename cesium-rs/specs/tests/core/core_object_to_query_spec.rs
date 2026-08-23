//! Tests for `cesium_core::object_to_query`.

use std::collections::HashMap;

use cesium_core::object_to_query::{object_to_query, QueryValue};

#[test]
fn empty_map_returns_empty_string() {
    let map = HashMap::new();
    assert_eq!(object_to_query(&map), "");
}

#[test]
fn single_entry() {
    let mut map = HashMap::new();
    map.insert("key".to_string(), QueryValue::Single("value".to_string()));
    let result = object_to_query(&map);
    assert_eq!(result, "key=value");
}

#[test]
fn multiple_values_for_same_key() {
    let mut map = HashMap::new();
    map.insert(
        "a".to_string(),
        QueryValue::Multiple(vec!["1".to_string(), "2".to_string()]),
    );
    let result = object_to_query(&map);
    assert!(result.contains("a=1"));
    assert!(result.contains("a=2"));
}

#[test]
fn special_characters_are_encoded() {
    let mut map = HashMap::new();
    map.insert("key".to_string(), QueryValue::Single("hello world".to_string()));
    let result = object_to_query(&map);
    assert!(result.contains("hello%20world"));
}
