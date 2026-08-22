//! Mirrors packages/engine/Specs/Core/binarySearchSpec.js

use cesium_core::binary_search::binary_search;
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/binarySearch")

#[test]
fn can_perform_a_binary_search_for_0() {
    let array = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let to_find = 0.0;
    let index = binary_search(&array, &to_find, |a: &f64, b: &f64| a - b);
    assert_eq!(index, 0);
}

#[test]
fn can_perform_a_binary_search_for_item_in_the_list() {
    let array = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let to_find = 7.0;
    let index = binary_search(&array, &to_find, |a: &f64, b: &f64| a - b);
    assert_eq!(index, 7);
}

#[test]
fn can_perform_a_binary_search_for_item_in_between_two_items_in_the_list() {
    let array = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let to_find = 3.5;
    let index = binary_search(&array, &to_find, |a: &f64, b: &f64| a - b);
    assert_eq!(!index, 4);
}

#[test]
fn can_perform_a_binary_search_for_item_before_all_items_in_the_list() {
    let array = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let to_find = -2.0;
    let index = binary_search(&array, &to_find, |a: &f64, b: &f64| a - b);
    assert_eq!(!index, 0);
}

#[test]
fn can_perform_a_binary_search_for_item_after_all_items_in_the_list() {
    let array = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let to_find = 12.0;
    let index = binary_search(&array, &to_find, |a: &f64, b: &f64| a - b);
    assert_eq!(!index, 8);
}

// JS: `function dummy() { return true; }` — the comparator parameter is
// statically required by the Rust signature.

#[test]
#[ignore = "Rust signature requires `array`; the missing-array case is statically impossible"]
fn throws_an_exception_if_array_is_missing() {
    expect_to_throw_dev_error(|| {});
}

#[test]
#[ignore = "Rust signature requires `item_to_find`; the missing-item case is statically impossible"]
fn throws_an_exception_if_item_to_find_is_missing() {
    expect_to_throw_dev_error(|| {});
}

#[test]
#[ignore = "Rust signature requires a comparator; the missing-comparator case is statically impossible"]
fn throws_an_exception_if_comparator_is_missing() {
    expect_to_throw_dev_error(|| {});
}
