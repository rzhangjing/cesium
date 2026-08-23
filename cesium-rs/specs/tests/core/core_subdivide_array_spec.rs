//! Tests for `cesium_core::subdivide_array`.

use cesium_core::subdivide_array::subdivide_array;

#[test]
fn zero_sub_arrays_returns_empty() {
    let result = subdivide_array(&[1, 2, 3], 0);
    assert!(result.is_empty());
}

#[test]
fn empty_array_returns_empty_sub_arrays() {
    let result = subdivide_array::<i32>(&[], 3);
    assert_eq!(result.len(), 3);
    for sub in &result {
        assert!(sub.is_empty());
    }
}

#[test]
fn single_sub_array_contains_all() {
    let result = subdivide_array(&[1, 2, 3, 4], 1);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], vec![1, 2, 3, 4]);
}

#[test]
fn two_sub_arrays_split_roughly_equally() {
    let result = subdivide_array(&[1, 2, 3, 4], 2);
    assert_eq!(result.len(), 2);
    let total: usize = result.iter().map(|s| s.len()).sum();
    assert_eq!(total, 4);
}

#[test]
fn more_sub_arrays_than_elements() {
    let result = subdivide_array(&[1, 2], 5);
    assert_eq!(result.len(), 5);
    let total: usize = result.iter().map(|s| s.len()).sum();
    assert_eq!(total, 2);
}
