//! Port of `Core/AssociativeArraySpec.js`.
use cesium_core::associative_array::AssociativeArray;

#[test]
fn default_constructor() {
    let aa: AssociativeArray<i32> = AssociativeArray::new();
    assert_eq!(aa.length(), 0);
    assert!(aa.values().is_empty());
}

#[test]
fn set_and_get() {
    let mut aa = AssociativeArray::new();
    assert!(!aa.contains("key1"));

    aa.set("key1".into(), 1);
    aa.set("key2".into(), 2);
    aa.set("key3".into(), 3);

    assert_eq!(aa.get("key1"), Some(&1));
    assert_eq!(aa.get("key2"), Some(&2));
    assert_eq!(aa.get("key3"), Some(&3));
    assert_eq!(aa.length(), 3);

    assert!(aa.contains("key1"));
    assert!(aa.contains("key2"));
    assert!(aa.contains("key3"));
}

#[test]
fn set_overwrites() {
    let mut aa = AssociativeArray::new();
    aa.set("key1".into(), 1);
    aa.set("key2".into(), 2);
    aa.set("key2".into(), 4);
    assert_eq!(aa.length(), 2);
    assert_eq!(aa.get("key2"), Some(&4));
}

#[test]
fn remove_all() {
    let mut aa = AssociativeArray::new();
    aa.set("key1".into(), 1);
    aa.set("key2".into(), 2);
    aa.remove_all();
    assert_eq!(aa.length(), 0);
    assert!(aa.values().is_empty());
}

#[test]
fn remove_nonexistent_returns_false() {
    let mut aa: AssociativeArray<i32> = AssociativeArray::new();
    assert!(!aa.remove("nonexistent"));
}
