//! Tests for `cesium_core::query_to_object`.

use cesium_core::object_to_query::QueryValue;
use cesium_core::query_to_object::query_to_object;

#[test]
fn empty_string_returns_empty_map() {
    let result = query_to_object("");
    assert!(result.is_empty());
}

#[test]
fn single_key_value() {
    let result = query_to_object("foo=bar");
    assert_eq!(result.len(), 1);
    match result.get("foo").unwrap() {
        QueryValue::Single(v) => assert_eq!(v, "bar"),
        _ => panic!("Expected Single"),
    }
}

#[test]
fn multiple_keys() {
    let result = query_to_object("a=1&b=2");
    assert_eq!(result.len(), 2);
}

#[test]
fn duplicate_keys_become_multiple() {
    let result = query_to_object("a=1&a=2");
    match result.get("a").unwrap() {
        QueryValue::Multiple(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], "1");
            assert_eq!(arr[1], "2");
        }
        _ => panic!("Expected Multiple"),
    }
}

#[test]
fn percent_decoding() {
    let result = query_to_object("key=hello%20world");
    match result.get("key").unwrap() {
        QueryValue::Single(v) => assert_eq!(v, "hello world"),
        _ => panic!("Expected Single"),
    }
}
