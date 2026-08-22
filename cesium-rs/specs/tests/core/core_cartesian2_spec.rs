//! Mirrors packages/engine/Specs/Core/Cartesian2Spec.js
//!
//! JS `undefined`-argument DeveloperError cases are statically impossible in
//! Rust; they are mirrored as `#[ignore]` stubs to keep the spec surface
//! one-to-one. Shared generators `createPackableSpecs` /
//! `createPackableArraySpecs` (repo-root `Specs/`) are inlined below.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::math::CesiumMath;
use cesium_test_utils::{assert_approx_eq_f64, expect_to_throw_dev_error};

const PI: f64 = std::f64::consts::PI;

/// JS `toEqualEpsilon` matcher: componentwise absolute-epsilon comparison.
fn assert_c2_eq_epsilon(expected: &Cartesian2, actual: &Cartesian2, epsilon: f64) {
    assert_approx_eq_f64!(expected.x, actual.x, epsilon);
    assert_approx_eq_f64!(expected.y, actual.y, epsilon);
}

// describe("Core/Cartesian2")

#[test]
fn construct_with_default_values() {
    let cartesian = Cartesian2::default();
    assert_eq!(cartesian.x, 0.0);
    assert_eq!(cartesian.y, 0.0);
}

#[test]
fn construct_with_only_an_x_value() {
    // JS `new Cartesian2(1.0)` defaults `y` to 0.0.
    let cartesian = Cartesian2::new(1.0, 0.0);
    assert_eq!(cartesian.x, 1.0);
    assert_eq!(cartesian.y, 0.0);
}

#[test]
fn construct_with_all_values() {
    let cartesian = Cartesian2::new(1.0, 2.0);
    assert_eq!(cartesian.x, 1.0);
    assert_eq!(cartesian.y, 2.0);
}

#[test]
fn from_array_creates_a_cartesian2() {
    let cartesian = Cartesian2::from_array_new(&[1.0, 2.0], None);
    assert_eq!(cartesian, Cartesian2::new(1.0, 2.0));
}

#[test]
fn from_array_with_an_offset_creates_a_cartesian2() {
    let cartesian = Cartesian2::from_array_new(&[0.0, 1.0, 2.0, 0.0], Some(1));
    assert_eq!(cartesian, Cartesian2::new(1.0, 2.0));
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_array_throws_without_values() {}

#[test]
fn clone_with_a_result_parameter() {
    let cartesian = Cartesian2::new(1.0, 2.0);
    let mut result = Cartesian2::default();
    Cartesian2::clone_into(&cartesian, &mut result);
    assert_eq!(cartesian, result);
}

#[test]
fn clone_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian2::new(1.0, 2.0);
    let current = cartesian;
    Cartesian2::clone_into(&current, &mut cartesian);
    assert_eq!(cartesian, Cartesian2::new(1.0, 2.0));
}

#[test]
fn maximum_component_works_when_x_is_greater() {
    let cartesian = Cartesian2::new(2.0, 1.0);
    assert_eq!(Cartesian2::maximum_component(&cartesian), cartesian.x);
}

#[test]
fn maximum_component_works_when_y_is_greater() {
    let cartesian = Cartesian2::new(1.0, 2.0);
    assert_eq!(Cartesian2::maximum_component(&cartesian), cartesian.y);
}

#[test]
fn minimum_component_works_when_x_is_lesser() {
    let cartesian = Cartesian2::new(1.0, 2.0);
    assert_eq!(Cartesian2::minimum_component(&cartesian), cartesian.x);
}

#[test]
fn minimum_component_works_when_y_is_lesser() {
    let cartesian = Cartesian2::new(2.0, 1.0);
    assert_eq!(Cartesian2::minimum_component(&cartesian), cartesian.y);
}

#[test]
fn minimum_by_component() {
    let mut result = Cartesian2::default();

    let cases = [
        (
            Cartesian2::new(2.0, 0.0),
            Cartesian2::new(1.0, 0.0),
            Cartesian2::new(1.0, 0.0),
        ),
        (
            Cartesian2::new(1.0, 0.0),
            Cartesian2::new(2.0, 0.0),
            Cartesian2::new(1.0, 0.0),
        ),
        (
            Cartesian2::new(2.0, -15.0),
            Cartesian2::new(1.0, -20.0),
            Cartesian2::new(1.0, -20.0),
        ),
        (
            Cartesian2::new(2.0, -20.0),
            Cartesian2::new(1.0, -15.0),
            Cartesian2::new(1.0, -20.0),
        ),
        (
            Cartesian2::new(2.0, -15.0),
            Cartesian2::new(1.0, -20.0),
            Cartesian2::new(1.0, -20.0),
        ),
        (
            Cartesian2::new(2.0, -15.0),
            Cartesian2::new(1.0, -20.0),
            Cartesian2::new(1.0, -20.0),
        ),
    ];
    for (first, second, expected) in cases {
        Cartesian2::minimum_by_component(&first, &second, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn minimum_by_component_with_a_result_parameter() {
    let first = Cartesian2::new(2.0, 0.0);
    let second = Cartesian2::new(1.0, 0.0);
    let expected = Cartesian2::new(1.0, 0.0);
    let mut result = Cartesian2::default();
    Cartesian2::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_with_a_result_parameter_that_is_an_input_parameter() {
    let mut first = Cartesian2::new(2.0, 0.0);
    let mut second = Cartesian2::new(1.0, 0.0);
    let expected = Cartesian2::new(1.0, 0.0);

    let first_in = first;
    Cartesian2::minimum_by_component(&first_in, &second, &mut first);
    assert_eq!(first, expected);

    first.x = 1.0;
    second.x = 2.0;
    let second_in = second;
    Cartesian2::minimum_by_component(&first, &second_in, &mut second);
    assert_eq!(second, expected);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn minimum_by_component_throws_without_first() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn minimum_by_component_throws_without_second() {}

#[test]
fn minimum_by_component_works_when_firsts_or_seconds_x_is_lesser() {
    let first = Cartesian2::new(2.0, 0.0);
    let mut second = Cartesian2::new(1.0, 0.0);
    let mut expected = Cartesian2::new(1.0, 0.0);

    let mut result = expected;
    Cartesian2::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.x = 3.0;
    expected.x = 2.0;
    Cartesian2::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_works_when_firsts_or_seconds_y_is_lesser() {
    let first = Cartesian2::new(0.0, 2.0);
    let mut second = Cartesian2::new(0.0, 1.0);
    let mut expected = Cartesian2::new(0.0, 1.0);
    let mut result = Cartesian2::default();
    Cartesian2::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.y = 3.0;
    expected.y = 2.0;
    Cartesian2::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component() {
    let mut result = Cartesian2::default();

    let cases = [
        (
            Cartesian2::new(2.0, 0.0),
            Cartesian2::new(1.0, 0.0),
            Cartesian2::new(2.0, 0.0),
        ),
        (
            Cartesian2::new(1.0, 0.0),
            Cartesian2::new(2.0, 0.0),
            Cartesian2::new(2.0, 0.0),
        ),
        (
            Cartesian2::new(2.0, -15.0),
            Cartesian2::new(1.0, -20.0),
            Cartesian2::new(2.0, -15.0),
        ),
        (
            Cartesian2::new(2.0, -20.0),
            Cartesian2::new(1.0, -15.0),
            Cartesian2::new(2.0, -15.0),
        ),
        (
            Cartesian2::new(2.0, -15.0),
            Cartesian2::new(1.0, -20.0),
            Cartesian2::new(2.0, -15.0),
        ),
        (
            Cartesian2::new(2.0, -15.0),
            Cartesian2::new(1.0, -20.0),
            Cartesian2::new(2.0, -15.0),
        ),
    ];
    for (first, second, expected) in cases {
        Cartesian2::maximum_by_component(&first, &second, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn maximum_by_component_with_a_result_parameter() {
    let first = Cartesian2::new(2.0, 0.0);
    let second = Cartesian2::new(1.0, 0.0);
    let expected = Cartesian2::new(2.0, 0.0);
    let mut result = Cartesian2::default();
    Cartesian2::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_with_a_result_parameter_that_is_an_input_parameter() {
    let mut first = Cartesian2::new(2.0, 0.0);
    let mut second = Cartesian2::new(1.0, 0.0);
    let expected = Cartesian2::new(2.0, 0.0);

    let first_in = first;
    Cartesian2::maximum_by_component(&first_in, &second, &mut first);
    assert_eq!(first, expected);

    first.x = 1.0;
    second.x = 2.0;
    let second_in = second;
    Cartesian2::maximum_by_component(&first, &second_in, &mut second);
    assert_eq!(second, expected);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn maximum_by_component_throws_without_first() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn maximum_by_component_throws_without_second() {}

#[test]
fn maximum_by_component_works_when_firsts_or_seconds_x_is_greater() {
    let first = Cartesian2::new(2.0, 0.0);
    let mut second = Cartesian2::new(1.0, 0.0);
    let mut expected = Cartesian2::new(2.0, 0.0);
    let mut result = Cartesian2::default();
    Cartesian2::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.x = 3.0;
    expected.x = 3.0;
    Cartesian2::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_works_when_firsts_or_seconds_y_is_greater() {
    let first = Cartesian2::new(0.0, 2.0);
    let mut second = Cartesian2::new(0.0, 1.0);
    let mut expected = Cartesian2::new(0.0, 2.0);
    let mut result = Cartesian2::default();
    Cartesian2::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.y = 3.0;
    expected.y = 3.0;
    Cartesian2::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn clamp() {
    let mut result = Cartesian2::default();

    let cases = [
        (
            Cartesian2::new(-1.0, 0.0),
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 1.0),
            Cartesian2::new(0.0, 0.0),
        ),
        (
            Cartesian2::new(2.0, 0.0),
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 1.0),
            Cartesian2::new(1.0, 0.0),
        ),
        (
            Cartesian2::new(0.0, -1.0),
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 1.0),
            Cartesian2::new(0.0, 0.0),
        ),
        (
            Cartesian2::new(0.0, 2.0),
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 1.0),
            Cartesian2::new(0.0, 1.0),
        ),
        (
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 1.0),
            Cartesian2::new(0.0, 0.0),
        ),
        (
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 1.0),
            Cartesian2::new(0.0, 0.0),
        ),
        (
            Cartesian2::new(-2.0, 3.0),
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 1.0),
            Cartesian2::new(0.0, 1.0),
        ),
        (
            Cartesian2::new(0.0, 0.0),
            Cartesian2::new(1.0, 2.0),
            Cartesian2::new(1.0, 2.0),
            Cartesian2::new(1.0, 2.0),
        ),
    ];
    for (value, min, max, expected) in cases {
        Cartesian2::clamp(&value, &min, &max, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn clamp_with_a_result_parameter() {
    let value = Cartesian2::new(-1.0, -1.0);
    let min = Cartesian2::new(0.0, 0.0);
    let max = Cartesian2::new(1.0, 1.0);
    let expected = Cartesian2::new(0.0, 0.0);
    let mut result = Cartesian2::default();
    Cartesian2::clamp(&value, &min, &max, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn clamp_with_a_result_parameter_that_is_an_input_parameter() {
    let mut value = Cartesian2::new(-1.0, -1.0);
    let mut min = Cartesian2::new(0.0, 0.0);
    let mut max = Cartesian2::new(1.0, 1.0);
    let expected = Cartesian2::new(0.0, 0.0);

    let value_in = value;
    Cartesian2::clamp(&value_in, &min, &max, &mut value);
    assert_eq!(value, expected);

    Cartesian2::from_elements(-1.0, -1.0, &mut value);
    let min_in = min;
    Cartesian2::clamp(&value, &min_in, &max, &mut min);
    assert_eq!(min, expected);

    Cartesian2::from_elements(0.0, 0.0, &mut value);
    let max_in = max;
    Cartesian2::clamp(&value, &min, &max_in, &mut max);
    assert_eq!(max, expected);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn clamp_throws_without_value() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn clamp_throws_without_min() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn clamp_throws_without_max() {}

#[test]
fn magnitude_squared() {
    let cartesian = Cartesian2::new(2.0, 3.0);
    assert_eq!(Cartesian2::magnitude_squared(&cartesian), 13.0);
}

#[test]
fn magnitude() {
    let cartesian = Cartesian2::new(2.0, 3.0);
    assert_eq!(Cartesian2::magnitude(&cartesian), 13.0_f64.sqrt());
}

#[test]
fn distance() {
    let distance = Cartesian2::distance(&Cartesian2::new(1.0, 0.0), &Cartesian2::new(2.0, 0.0));
    assert_eq!(distance, 1.0);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn distance_throws_without_left() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn distance_throws_without_right() {}

#[test]
fn distance_squared() {
    let distance_squared =
        Cartesian2::distance_squared(&Cartesian2::new(1.0, 0.0), &Cartesian2::new(3.0, 0.0));
    assert_eq!(distance_squared, 4.0);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn distance_squared_throws_without_left() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn distance_squared_throws_without_right() {}

#[test]
fn normalize_works_with_a_result_parameter() {
    let cartesian = Cartesian2::new(2.0, 0.0);
    let expected_result = Cartesian2::new(1.0, 0.0);
    let mut result = Cartesian2::default();
    Cartesian2::normalize(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn normalize_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian2::new(2.0, 0.0);
    let expected_result = Cartesian2::new(1.0, 0.0);
    let current = cartesian;
    Cartesian2::normalize(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn normalize_throws_with_zero_vector() {
    expect_to_throw_dev_error(|| {
        let mut result = Cartesian2::default();
        Cartesian2::normalize(&Cartesian2::ZERO, &mut result);
    });
}

#[test]
fn multiply_components_works_with_a_result_parameter() {
    let left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(4.0, 5.0);
    let expected_result = Cartesian2::new(8.0, 15.0);
    let mut result = Cartesian2::default();
    Cartesian2::multiply_components(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn multiply_components_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(4.0, 5.0);
    let expected_result = Cartesian2::new(8.0, 15.0);
    let current = left;
    Cartesian2::multiply_components(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn divide_components_works_with_a_result_parameter() {
    let left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(4.0, 5.0);
    let expected_result = Cartesian2::new(0.5, 0.6);
    let mut result = Cartesian2::default();
    Cartesian2::divide_components(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn divide_components_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(4.0, 5.0);
    let expected_result = Cartesian2::new(0.5, 0.6);
    let current = left;
    Cartesian2::divide_components(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn dot() {
    let left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(4.0, 5.0);
    let expected_result = 23.0;
    let result = Cartesian2::dot(&left, &right);
    assert_eq!(result, expected_result);
}

#[test]
fn cross() {
    let left = Cartesian2::new(0.0, 1.0);
    let right = Cartesian2::new(1.0, 0.0);
    let expected_result = -1.0;
    let result = Cartesian2::cross(&left, &right);
    assert_eq!(result, expected_result);
}

#[test]
fn add_works_with_a_result_parameter() {
    let left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(4.0, 5.0);
    let expected_result = Cartesian2::new(6.0, 8.0);
    let mut result = Cartesian2::default();
    Cartesian2::add(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn add_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(4.0, 5.0);
    let expected_result = Cartesian2::new(6.0, 8.0);
    let current = left;
    Cartesian2::add(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn subtract_works_with_a_result_parameter() {
    let left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(1.0, 5.0);
    let expected_result = Cartesian2::new(1.0, -2.0);
    let mut result = Cartesian2::default();
    Cartesian2::subtract(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn subtract_works_with_this_result_parameter() {
    let mut left = Cartesian2::new(2.0, 3.0);
    let right = Cartesian2::new(1.0, 5.0);
    let expected_result = Cartesian2::new(1.0, -2.0);
    let current = left;
    Cartesian2::subtract(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn multiply_by_scalar_with_a_result_parameter() {
    let cartesian = Cartesian2::new(1.0, 2.0);
    let scalar = 2.0;
    let expected_result = Cartesian2::new(2.0, 4.0);
    let mut result = Cartesian2::default();
    Cartesian2::multiply_by_scalar(&cartesian, scalar, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn multiply_by_scalar_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian2::new(1.0, 2.0);
    let scalar = 2.0;
    let expected_result = Cartesian2::new(2.0, 4.0);
    let current = cartesian;
    Cartesian2::multiply_by_scalar(&current, scalar, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn divide_by_scalar_with_a_result_parameter() {
    let cartesian = Cartesian2::new(1.0, 2.0);
    let scalar = 2.0;
    let expected_result = Cartesian2::new(0.5, 1.0);
    let mut result = Cartesian2::default();
    Cartesian2::divide_by_scalar(&cartesian, scalar, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn divide_by_scalar_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian2::new(1.0, 2.0);
    let scalar = 2.0;
    let expected_result = Cartesian2::new(0.5, 1.0);
    let current = cartesian;
    Cartesian2::divide_by_scalar(&current, scalar, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn negate_with_a_result_parameter() {
    let cartesian = Cartesian2::new(1.0, -2.0);
    let expected_result = Cartesian2::new(-1.0, 2.0);
    let mut result = Cartesian2::default();
    Cartesian2::negate(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn negate_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian2::new(1.0, -2.0);
    let expected_result = Cartesian2::new(-1.0, 2.0);
    let current = cartesian;
    Cartesian2::negate(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn abs_with_a_result_parameter() {
    let cartesian = Cartesian2::new(1.0, -2.0);
    let expected_result = Cartesian2::new(1.0, 2.0);
    let mut result = Cartesian2::default();
    Cartesian2::abs(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn abs_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian2::new(1.0, -2.0);
    let expected_result = Cartesian2::new(1.0, 2.0);
    let current = cartesian;
    Cartesian2::abs(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn lerp_works_with_a_result_parameter() {
    let start = Cartesian2::new(4.0, 8.0);
    let end = Cartesian2::new(8.0, 20.0);
    let t = 0.25;
    let expected_result = Cartesian2::new(5.0, 11.0);
    let mut result = Cartesian2::default();
    Cartesian2::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn lerp_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut start = Cartesian2::new(4.0, 8.0);
    let end = Cartesian2::new(8.0, 20.0);
    let t = 0.25;
    let expected_result = Cartesian2::new(5.0, 11.0);
    let current = start;
    Cartesian2::lerp(&current, &end, t, &mut start);
    assert_eq!(start, expected_result);
}

#[test]
fn lerp_extrapolate_forward() {
    let start = Cartesian2::new(4.0, 8.0);
    let end = Cartesian2::new(8.0, 20.0);
    let t = 2.0;
    let expected_result = Cartesian2::new(12.0, 32.0);
    let mut result = Cartesian2::default();
    Cartesian2::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn lerp_extrapolate_backward() {
    let start = Cartesian2::new(4.0, 8.0);
    let end = Cartesian2::new(8.0, 20.0);
    let t = -1.0;
    let expected_result = Cartesian2::new(0.0, -4.0);
    let mut result = Cartesian2::default();
    Cartesian2::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn angle_between_works_for_right_angles() {
    let x = Cartesian2::UNIT_X;
    let y = Cartesian2::UNIT_Y;
    assert_eq!(Cartesian2::angle_between(&x, &y), CesiumMath::PI_OVER_TWO);
    assert_eq!(Cartesian2::angle_between(&y, &x), CesiumMath::PI_OVER_TWO);
}

#[test]
fn angle_between_works_for_acute_angles() {
    let x = Cartesian2::new(0.0, 1.0);
    let y = Cartesian2::new(1.0, 1.0);
    assert_approx_eq_f64!(
        Cartesian2::angle_between(&x, &y),
        CesiumMath::PI_OVER_FOUR,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        Cartesian2::angle_between(&y, &x),
        CesiumMath::PI_OVER_FOUR,
        CesiumMath::EPSILON14
    );
}

#[test]
fn angle_between_works_for_obtuse_angles() {
    let x = Cartesian2::new(0.0, 1.0);
    let y = Cartesian2::new(-1.0, -1.0);
    assert_approx_eq_f64!(
        Cartesian2::angle_between(&x, &y),
        (PI * 3.0) / 4.0,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        Cartesian2::angle_between(&y, &x),
        (PI * 3.0) / 4.0,
        CesiumMath::EPSILON14
    );
}

#[test]
fn angle_between_works_for_zero_angles() {
    let x = Cartesian2::UNIT_X;
    assert_eq!(Cartesian2::angle_between(&x, &x), 0.0);
}

#[test]
fn most_orthogonal_angle_is_x() {
    let v = Cartesian2::new(0.0, 1.0);
    let mut result = Cartesian2::default();
    Cartesian2::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian2::UNIT_X);
}

#[test]
fn most_orthogonal_angle_is_y() {
    let v = Cartesian2::new(1.0, 0.0);
    let mut result = Cartesian2::default();
    Cartesian2::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian2::UNIT_Y);
}

#[test]
fn equals() {
    let cartesian = Cartesian2::new(1.0, 2.0);
    assert!(Cartesian2::equals(Some(&cartesian), Some(&Cartesian2::new(1.0, 2.0))));
    assert!(!Cartesian2::equals(Some(&cartesian), Some(&Cartesian2::new(2.0, 2.0))));
    assert!(!Cartesian2::equals(Some(&cartesian), Some(&Cartesian2::new(2.0, 1.0))));
    assert!(!Cartesian2::equals(Some(&cartesian), None));
}

#[test]
fn equals_epsilon() {
    let mut cartesian = Cartesian2::new(1.0, 2.0);
    assert!(cartesian.equals_epsilon_method(&Cartesian2::new(1.0, 2.0), None, Some(0.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian2::new(1.0, 2.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian2::new(2.0, 2.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian2::new(1.0, 3.0), None, Some(1.0)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian2::new(1.0, 3.0), None, Some(CesiumMath::EPSILON6)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian2::new(1.0, 3.0), None, Some(CesiumMath::EPSILON6)));

    cartesian = Cartesian2::new(3000000.0, 4000000.0);
    assert!(cartesian.equals_epsilon_method(&Cartesian2::new(3000000.0, 4000000.0), None, Some(0.0)));
    assert!(cartesian
        .equals_epsilon_method(&Cartesian2::new(3000000.0, 4000000.2), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(cartesian
        .equals_epsilon_method(&Cartesian2::new(3000000.2, 4000000.0), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(cartesian
        .equals_epsilon_method(&Cartesian2::new(3000000.2, 4000000.2), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(!cartesian
        .equals_epsilon_method(&Cartesian2::new(3000000.2, 4000000.2), Some(CesiumMath::EPSILON9), Some(CesiumMath::EPSILON9)));

    // JS `Cartesian2.equalsEpsilon(undefined, cartesian, 1)` -> false.
    assert!(!Cartesian2::equals_epsilon(
        None,
        Some(&cartesian),
        Some(1.0),
        Some(1.0)
    ));
}

#[test]
fn to_string() {
    let cartesian = Cartesian2::new(1.123, 2.345);
    assert_eq!(cartesian.to_string(), "(1.123, 2.345)");
}

#[test]
#[ignore = "JS undefined-argument behavior; statically impossible in Rust"]
fn clone_returns_undefined_with_no_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn maximum_component_throws_with_no_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn minimum_component_throws_with_no_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn magnitude_squared_throws_with_no_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn magnitude_throws_with_no_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn normalize_throws_with_no_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn dot_throws_with_no_left_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn dot_throws_with_no_right_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn multiply_components_throw_with_no_left_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn multiply_components_throw_with_no_right_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn divide_components_throw_with_no_left_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn divide_components_throw_with_no_right_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn add_throws_with_no_left_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn add_throws_with_no_right_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn subtract_throws_with_no_left_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn subtract_throws_with_no_right_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn multiply_by_scalar_throws_with_no_cartesian_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn multiply_by_scalar_throws_with_no_scalar_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn divide_by_scalar_throws_with_no_cartesian_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn divide_by_scalar_throws_with_no_scalar_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn negate_throws_with_no_cartesian_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn abs_throws_with_no_cartesian_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn lerp_throws_with_no_start_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn lerp_throws_with_no_end_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn lerp_throws_with_no_t_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn angle_between_throws_with_no_left_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn angle_between_throws_with_no_right_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn most_orthogonal_axis_throws_with_no_cartesian_parameter() {}

#[test]
fn from_elements_returns_a_cartesian2_with_correct_coordinates() {
    let cartesian2 = Cartesian2::from_elements_new(2.0, 2.0);
    let expected_result = Cartesian2::new(2.0, 2.0);
    assert_eq!(cartesian2, expected_result);
}

#[test]
fn from_elements_result_param_returns_cartesian2_with_correct_coordinates() {
    let mut cartesian2 = Cartesian2::default();
    Cartesian2::from_elements(2.0, 2.0, &mut cartesian2);
    let expected_result = Cartesian2::new(2.0, 2.0);
    assert_eq!(cartesian2, expected_result);
}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn minimum_by_component_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn maximum_by_component_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn clamp_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn normalize_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn multiply_components_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn divide_components_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn add_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn subtract_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn multiply_by_scalar_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn divide_by_scalar_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn negate_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn abs_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn lerp_throws_with_no_result() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn most_orthogonal_axis_throws_with_no_result() {}

//////////////////////////////////////////////////////////////////////
// createPackableSpecs(Cartesian2, new Cartesian2(1, 2), [1, 2])
//////////////////////////////////////////////////////////////////////

fn packable_instance() -> Cartesian2 {
    Cartesian2::new(1.0, 2.0)
}

const PACKED_INSTANCE: [f64; 2] = [1.0, 2.0];

#[test]
fn packable_can_pack() {
    // JS grows an empty array; the Rust `pack` writes into a sized slice.
    let mut packed_array = vec![0.0; Cartesian2::PACKED_LENGTH];
    Cartesian2::pack(&packable_instance(), &mut packed_array, None);
    assert_eq!(packed_array.len(), Cartesian2::PACKED_LENGTH);
    assert_c2_eq_epsilon(
        &Cartesian2::new(PACKED_INSTANCE[0], PACKED_INSTANCE[1]),
        &Cartesian2::new(packed_array[0], packed_array[1]),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn packable_can_roundtrip() {
    let mut packed_array = vec![0.0; Cartesian2::PACKED_LENGTH];
    Cartesian2::pack(&packable_instance(), &mut packed_array, None);
    let result = Cartesian2::unpack_new(&packed_array, None);
    assert_eq!(packable_instance(), result);
}

#[test]
fn packable_can_unpack() {
    let result = Cartesian2::unpack_new(&PACKED_INSTANCE, None);
    assert_eq!(result, packable_instance());
}

#[test]
fn packable_can_pack_with_starting_index() {
    let mut packed_array = vec![0.0; 1 + Cartesian2::PACKED_LENGTH];
    let expected: Vec<f64> = [0.0_f64].iter().chain(PACKED_INSTANCE.iter()).copied().collect();
    Cartesian2::pack(&packable_instance(), &mut packed_array, Some(1));
    for i in 0..expected.len() {
        assert_approx_eq_f64!(packed_array[i], expected[i], CesiumMath::EPSILON15);
    }
}

#[test]
fn packable_can_unpack_with_starting_index() {
    let packed_array: Vec<f64> = [0.0_f64].iter().chain(PACKED_INSTANCE.iter()).copied().collect();
    let result = Cartesian2::unpack_new(&packed_array, Some(1));
    assert_eq!(packable_instance(), result);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_pack_throws_with_undefined_value() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_pack_throws_with_undefined_array() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_unpack_throws_with_undefined_array() {}

//////////////////////////////////////////////////////////////////////
// createPackableArraySpecs(Cartesian2, [(1,2),(3,4)], [1,2,3,4], 2)
//////////////////////////////////////////////////////////////////////

fn packable_unpacked_array() -> Vec<Cartesian2> {
    vec![Cartesian2::new(1.0, 2.0), Cartesian2::new(3.0, 4.0)]
}

const PACKED_ARRAY: [f64; 4] = [1.0, 2.0, 3.0, 4.0];

const PACKABLE_STRIDE: usize = 2;

#[test]
fn packable_array_can_pack() {
    let actual_packed_array = Cartesian2::pack_array(&packable_unpacked_array(), None);
    assert_eq!(actual_packed_array.len(), PACKED_ARRAY.len());
    assert_eq!(actual_packed_array, PACKED_ARRAY);
}

#[test]
fn packable_array_can_roundtrip() {
    let actual_packed_array = Cartesian2::pack_array(&packable_unpacked_array(), None);
    let result = Cartesian2::unpack_array(&actual_packed_array, None);
    assert_eq!(result, packable_unpacked_array());
}

#[test]
fn packable_array_can_unpack() {
    let result = Cartesian2::unpack_array(&PACKED_ARRAY, None);
    assert_eq!(result, packable_unpacked_array());
}

#[test]
#[ignore = "DEVIATION: Rust has a single Vec<f64> representation; JS typed-array branch not ported"]
fn packable_array_pack_array_works_with_typed_arrays() {}

#[test]
fn packable_array_pack_array_resizes_arrays_as_needed() {
    let empty_array: Vec<f64> = Vec::new();
    let result = Cartesian2::pack_array(&packable_unpacked_array(), Some(empty_array));
    assert_eq!(result, PACKED_ARRAY);

    let larger_array = vec![0.0; PACKED_ARRAY.len() + 1];
    let result = Cartesian2::pack_array(&packable_unpacked_array(), Some(larger_array));
    assert_eq!(result, PACKED_ARRAY);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_array_pack_array_throws_with_undefined_array() {}

#[test]
#[ignore = "DEVIATION: Rust has a single Vec<f64> representation; JS typed-array branch not ported"]
fn packable_array_pack_array_throws_for_typed_arrays_of_the_wrong_size() {}

#[test]
fn packable_array_unpack_array_works_for_typed_arrays() {
    // Vec<f64> mirrors both JS regular and typed arrays.
    let array = Cartesian2::unpack_array(&PACKED_ARRAY, None);
    assert_eq!(array, packable_unpacked_array());
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_array_unpack_array_throws_with_undefined_array() {}

#[test]
fn packable_array_unpack_array_works_with_a_result_parameter() {
    let array: Vec<Cartesian2> = Vec::new();
    let result = Cartesian2::unpack_array(&PACKED_ARRAY, Some(array));
    assert_eq!(result, packable_unpacked_array());

    let array: Vec<Cartesian2> = vec![Cartesian2::default(); packable_unpacked_array().len()];
    let result = Cartesian2::unpack_array(&PACKED_ARRAY, Some(array));
    assert_eq!(result, packable_unpacked_array());
}

#[test]
fn packable_array_unpack_array_throws_with_array_less_than_the_minimum_length() {
    expect_to_throw_dev_error(|| {
        Cartesian2::unpack_array(&[1.0], None);
    });
}

#[test]
fn unpack_array_throws_with_array_not_multiple_of_stride() {
    expect_to_throw_dev_error(|| {
        Cartesian2::unpack_array(&vec![1.0; PACKABLE_STRIDE + 1], None);
    });
}
