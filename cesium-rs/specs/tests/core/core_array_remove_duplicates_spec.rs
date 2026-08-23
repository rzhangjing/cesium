//! Tests for `cesium_core::array_remove_duplicates`.

use cesium_core::array_remove_duplicates::array_remove_duplicates;

fn f64_epsilon_eq(a: &f64, b: &f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn no_duplicates_returns_none() {
    let values = vec![1.0, 2.0, 3.0];
    let result = array_remove_duplicates(&values, f64_epsilon_eq, false, None);
    assert!(result.is_none());
}

#[test]
fn adjacent_duplicates_are_removed() {
    let values = vec![1.0, 1.0, 2.0, 3.0];
    let result = array_remove_duplicates(&values, f64_epsilon_eq, false, None).unwrap();
    assert_eq!(result, vec![1.0, 2.0, 3.0]);
}

#[test]
fn multiple_adjacent_duplicates() {
    let values = vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0];
    let result = array_remove_duplicates(&values, f64_epsilon_eq, false, None).unwrap();
    assert_eq!(result, vec![1.0, 2.0, 3.0]);
}

#[test]
fn wrap_around_removes_last_if_equal_to_first() {
    let values = vec![1.0, 2.0, 3.0, 1.0];
    let result = array_remove_duplicates(&values, f64_epsilon_eq, true, None).unwrap();
    assert_eq!(result, vec![1.0, 2.0, 3.0]);
}

#[test]
fn single_element_returns_none() {
    let values = vec![1.0];
    let result = array_remove_duplicates(&values, f64_epsilon_eq, false, None);
    assert!(result.is_none());
}

#[test]
fn removed_indices_are_collected() {
    let values = vec![1.0, 1.0, 2.0, 3.0];
    let mut removed = Vec::new();
    let result = array_remove_duplicates(&values, f64_epsilon_eq, false, Some(&mut removed)).unwrap();
    assert_eq!(result, vec![1.0, 2.0, 3.0]);
    assert_eq!(removed, vec![1]);
}
