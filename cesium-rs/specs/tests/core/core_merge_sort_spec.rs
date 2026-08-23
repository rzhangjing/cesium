//! Tests for `cesium_core::merge_sort`.

use cesium_core::merge_sort::merge_sort;

#[test]
fn sort_empty_array() {
    let mut arr: Vec<i32> = vec![];
    merge_sort(&mut arr, |a, b| a - b);
    assert!(arr.is_empty());
}

#[test]
fn sort_single_element() {
    let mut arr = vec![42];
    merge_sort(&mut arr, |a, b| a - b);
    assert_eq!(arr, vec![42]);
}

#[test]
fn sort_ascending() {
    let mut arr = vec![5, 3, 1, 4, 2];
    merge_sort(&mut arr, |a, b| a - b);
    assert_eq!(arr, vec![1, 2, 3, 4, 5]);
}

#[test]
fn sort_descending() {
    let mut arr = vec![1, 2, 3, 4, 5];
    merge_sort(&mut arr, |a, b| b - a);
    assert_eq!(arr, vec![5, 4, 3, 2, 1]);
}

#[test]
fn sort_already_sorted() {
    let mut arr = vec![1, 2, 3, 4, 5];
    merge_sort(&mut arr, |a, b| a - b);
    assert_eq!(arr, vec![1, 2, 3, 4, 5]);
}

#[test]
fn sort_with_duplicates() {
    let mut arr = vec![3, 1, 3, 2, 1];
    merge_sort(&mut arr, |a, b| a - b);
    assert_eq!(arr, vec![1, 1, 2, 3, 3]);
}
