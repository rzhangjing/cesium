//! Mirrors packages/engine/Specs/Core/Cartesian4Spec.js
//!
//! JS `undefined`-argument DeveloperError cases are statically impossible in
//! Rust; they are mirrored as `#[ignore]` stubs to keep the spec surface
//! one-to-one. `fromColor` cases are `#[ignore]`d until the `Color` port
//! lands. Shared generators `createPackableSpecs` /
//! `createPackableArraySpecs` (repo-root `Specs/`) are inlined below.

use cesium_core::cartesian4::Cartesian4;
use cesium_core::math::CesiumMath;
use cesium_test_utils::{assert_approx_eq_f64, expect_to_throw_dev_error};

/// JS `toEqualEpsilon` matcher: componentwise absolute-epsilon comparison.
fn assert_c4_eq_epsilon(expected: &Cartesian4, actual: &Cartesian4, epsilon: f64) {
    assert_approx_eq_f64!(expected.x, actual.x, epsilon);
    assert_approx_eq_f64!(expected.y, actual.y, epsilon);
    assert_approx_eq_f64!(expected.z, actual.z, epsilon);
    assert_approx_eq_f64!(expected.w, actual.w, epsilon);
}

// describe("Core/Cartesian4")

#[test]
fn construct_with_default_values() {
    let cartesian = Cartesian4::default();
    assert_eq!(cartesian.x, 0.0);
    assert_eq!(cartesian.y, 0.0);
    assert_eq!(cartesian.z, 0.0);
    assert_eq!(cartesian.w, 0.0);
}

#[test]
fn construct_with_all_values() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(cartesian.x, 1.0);
    assert_eq!(cartesian.y, 2.0);
    assert_eq!(cartesian.z, 3.0);
    assert_eq!(cartesian.w, 4.0);
}

#[test]
fn from_array_creates_a_cartesian4() {
    let cartesian = Cartesian4::from_array_new(&[1.0, 2.0, 3.0, 4.0], None);
    assert_eq!(cartesian, Cartesian4::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn from_array_with_an_offset_creates_a_cartesian4() {
    let cartesian = Cartesian4::from_array_new(&[0.0, 1.0, 2.0, 3.0, 4.0, 0.0], Some(1));
    assert_eq!(cartesian, Cartesian4::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn from_array_creates_a_cartesian4_with_a_result_parameter() {
    let mut cartesian = Cartesian4::default();
    Cartesian4::from_array(&[1.0, 2.0, 3.0, 4.0], Some(0), &mut cartesian);
    assert_eq!(cartesian, Cartesian4::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_array_throws_without_values() {}

#[test]
fn from_elements_returns_a_cartesian4_with_correct_coordinates() {
    let cartesian4 = Cartesian4::from_elements_new(2.0, 2.0, 4.0, 7.0);
    let expected_result = Cartesian4::new(2.0, 2.0, 4.0, 7.0);
    assert_eq!(cartesian4, expected_result);
}

#[test]
fn from_elements_result_param_returns_cartesian4_with_correct_coordinates() {
    let mut cartesian4 = Cartesian4::default();
    Cartesian4::from_elements(2.0, 2.0, 4.0, 7.0, &mut cartesian4);
    let expected_result = Cartesian4::new(2.0, 2.0, 4.0, 7.0);
    assert_eq!(cartesian4, expected_result);
}

#[test]
#[ignore = "deferred: fromColor depends on the Color port (Scene/Core, later batch)"]
fn from_color_returns_a_cartesian4_with_correct_coordinates() {}

#[test]
#[ignore = "deferred: fromColor depends on the Color port (Scene/Core, later batch)"]
fn from_color_result_param_returns_cartesian4_with_correct_coordinates() {}

#[test]
#[ignore = "deferred: fromColor depends on the Color port (Scene/Core, later batch)"]
fn from_color_throws_without_color() {}

#[test]
fn clone_without_a_result_parameter() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    let result = cartesian.clone();
    assert_eq!(cartesian, result);
}

#[test]
fn clone_with_a_result_parameter() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    let mut result = Cartesian4::default();
    Cartesian4::clone_into(&cartesian, &mut result);
    assert_eq!(cartesian, result);
}

#[test]
fn clone_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    let current = cartesian;
    Cartesian4::clone_into(&current, &mut cartesian);
    assert_eq!(cartesian, Cartesian4::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn maximum_component_works_when_x_is_greater() {
    let cartesian = Cartesian4::new(2.0, 1.0, 0.0, -1.0);
    assert_eq!(Cartesian4::maximum_component(&cartesian), cartesian.x);
}

#[test]
fn maximum_component_works_when_y_is_greater() {
    let cartesian = Cartesian4::new(1.0, 2.0, 0.0, -1.0);
    assert_eq!(Cartesian4::maximum_component(&cartesian), cartesian.y);
}

#[test]
fn maximum_component_works_when_z_is_greater() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, -1.0);
    assert_eq!(Cartesian4::maximum_component(&cartesian), cartesian.z);
}

#[test]
fn maximum_component_works_when_w_is_greater() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(Cartesian4::maximum_component(&cartesian), cartesian.w);
}

#[test]
fn minimum_component_works_when_x_is_lesser() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(Cartesian4::minimum_component(&cartesian), cartesian.x);
}

#[test]
fn minimum_component_works_when_y_is_lesser() {
    let cartesian = Cartesian4::new(2.0, 1.0, 3.0, 4.0);
    assert_eq!(Cartesian4::minimum_component(&cartesian), cartesian.y);
}

#[test]
fn minimum_component_works_when_z_is_lesser() {
    let cartesian = Cartesian4::new(2.0, 1.0, 0.0, 4.0);
    assert_eq!(Cartesian4::minimum_component(&cartesian), cartesian.z);
}

#[test]
fn minimum_component_works_when_w_is_lesser() {
    let cartesian = Cartesian4::new(2.0, 1.0, 0.0, -1.0);
    assert_eq!(Cartesian4::minimum_component(&cartesian), cartesian.w);
}

#[test]
fn minimum_by_component() {
    let mut result = Cartesian4::default();

    let cases = [
        (
            Cartesian4::new(2.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(1.0, 0.0, 0.0, 0.0),
            Cartesian4::new(2.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 0.0, 0.0),
            Cartesian4::new(1.0, -20.0, 0.0, 0.0),
            Cartesian4::new(1.0, -20.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(2.0, -20.0, 0.0, 0.0),
            Cartesian4::new(1.0, -15.0, 0.0, 0.0),
            Cartesian4::new(1.0, -20.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.4, 0.0),
            Cartesian4::new(1.0, -20.0, 26.5, 0.0),
            Cartesian4::new(1.0, -20.0, 26.4, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.5, 0.0),
            Cartesian4::new(1.0, -20.0, 26.4, 0.0),
            Cartesian4::new(1.0, -20.0, 26.4, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.4, -450.0),
            Cartesian4::new(1.0, -20.0, 26.5, 450.0),
            Cartesian4::new(1.0, -20.0, 26.4, -450.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.5, 450.0),
            Cartesian4::new(1.0, -20.0, 26.4, -450.0),
            Cartesian4::new(1.0, -20.0, 26.4, -450.0),
        ),
    ];
    for (first, second, expected) in cases {
        Cartesian4::minimum_by_component(&first, &second, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn minimum_by_component_with_a_result_parameter() {
    let first = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let second = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let expected = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_with_a_result_parameter_that_is_an_input_parameter() {
    let mut first = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let mut second = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let expected = Cartesian4::new(1.0, 0.0, 0.0, 0.0);

    let first_in = first;
    Cartesian4::minimum_by_component(&first_in, &second, &mut first);
    assert_eq!(first, expected);

    first.x = 1.0;
    second.x = 2.0;
    let second_in = second;
    Cartesian4::minimum_by_component(&first, &second_in, &mut second);
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
    let first = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let mut second = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let mut expected = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.x = 3.0;
    expected.x = 2.0;
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_works_when_firsts_or_seconds_y_is_lesser() {
    let first = Cartesian4::new(0.0, 2.0, 0.0, 0.0);
    let mut second = Cartesian4::new(0.0, 1.0, 0.0, 0.0);
    let mut expected = Cartesian4::new(0.0, 1.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.y = 3.0;
    expected.y = 2.0;
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_works_when_firsts_or_seconds_z_is_lesser() {
    let first = Cartesian4::new(0.0, 0.0, 2.0, 0.0);
    let mut second = Cartesian4::new(0.0, 0.0, 1.0, 0.0);
    let mut expected = Cartesian4::new(0.0, 0.0, 1.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.z = 3.0;
    expected.z = 2.0;
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_works_when_firsts_or_seconds_w_is_lesser() {
    let first = Cartesian4::new(0.0, 0.0, 0.0, 2.0);
    let mut second = Cartesian4::new(0.0, 0.0, 0.0, 1.0);
    let mut expected = Cartesian4::new(0.0, 0.0, 0.0, 1.0);
    let mut result = Cartesian4::default();
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.w = 3.0;
    expected.w = 2.0;
    Cartesian4::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component() {
    let mut result = Cartesian4::default();

    let cases = [
        (
            Cartesian4::new(2.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 0.0, 0.0, 0.0),
            Cartesian4::new(2.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(1.0, 0.0, 0.0, 0.0),
            Cartesian4::new(2.0, 0.0, 0.0, 0.0),
            Cartesian4::new(2.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 0.0, 0.0),
            Cartesian4::new(1.0, -20.0, 0.0, 0.0),
            Cartesian4::new(2.0, -15.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(2.0, -20.0, 0.0, 0.0),
            Cartesian4::new(1.0, -15.0, 0.0, 0.0),
            Cartesian4::new(2.0, -15.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.4, 0.0),
            Cartesian4::new(1.0, -20.0, 26.5, 0.0),
            Cartesian4::new(2.0, -15.0, 26.5, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.5, 0.0),
            Cartesian4::new(1.0, -20.0, 26.4, 0.0),
            Cartesian4::new(2.0, -15.0, 26.5, 0.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.5, 450.0),
            Cartesian4::new(1.0, -20.0, 26.4, -450.0),
            Cartesian4::new(2.0, -15.0, 26.5, 450.0),
        ),
        (
            Cartesian4::new(2.0, -15.0, 26.5, -450.0),
            Cartesian4::new(1.0, -20.0, 26.4, 450.0),
            Cartesian4::new(2.0, -15.0, 26.5, 450.0),
        ),
    ];
    for (first, second, expected) in cases {
        Cartesian4::maximum_by_component(&first, &second, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn maximum_by_component_with_a_result_parameter() {
    let first = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let second = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let expected = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_with_a_result_parameter_that_is_an_input_parameter() {
    let mut first = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let mut second = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let expected = Cartesian4::new(2.0, 0.0, 0.0, 0.0);

    let first_in = first;
    Cartesian4::maximum_by_component(&first_in, &second, &mut first);
    assert_eq!(first, expected);

    first.x = 1.0;
    second.x = 2.0;
    let second_in = second;
    Cartesian4::maximum_by_component(&first, &second_in, &mut second);
    assert_eq!(second, expected);
}

#[test]
fn maximum_by_component_with_a_result_parameter_that_is_second() {
    // JS duplicates the previous `it` with `result === second` both times.
    let mut first = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let mut second = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let expected = Cartesian4::new(2.0, 0.0, 0.0, 0.0);

    let second_in = second;
    Cartesian4::maximum_by_component(&first, &second_in, &mut second);
    assert_eq!(second, expected);

    first.x = 1.0;
    second.x = 2.0;
    let second_in = second;
    Cartesian4::maximum_by_component(&first, &second_in, &mut second);
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
    let first = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let mut second = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let mut expected = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.x = 3.0;
    expected.x = 3.0;
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_works_when_firsts_or_seconds_y_is_greater() {
    let first = Cartesian4::new(0.0, 2.0, 0.0, 0.0);
    let mut second = Cartesian4::new(0.0, 1.0, 0.0, 0.0);
    let mut expected = Cartesian4::new(0.0, 2.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.y = 3.0;
    expected.y = 3.0;
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_works_when_firsts_or_seconds_z_is_greater() {
    let first = Cartesian4::new(0.0, 0.0, 2.0, 0.0);
    let mut second = Cartesian4::new(0.0, 0.0, 1.0, 0.0);
    let mut expected = Cartesian4::new(0.0, 0.0, 2.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.z = 3.0;
    expected.z = 3.0;
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_works_when_firsts_or_seconds_w_is_greater() {
    let first = Cartesian4::new(0.0, 0.0, 0.0, 2.0);
    let mut second = Cartesian4::new(0.0, 0.0, 0.0, 1.0);
    let mut expected = Cartesian4::new(0.0, 0.0, 0.0, 2.0);
    let mut result = Cartesian4::default();
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.w = 3.0;
    expected.w = 3.0;
    Cartesian4::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn clamp() {
    let mut result = Cartesian4::default();

    // JS passes 3-element `new Cartesian4(x, y, z)` values whose `w`
    // defaults to 0.0.
    let cases = [
        (
            Cartesian4::new(-1.0, 0.0, 0.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 1.0, 1.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(2.0, 0.0, 0.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 1.0, 1.0, 0.0),
            Cartesian4::new(1.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(0.0, -1.0, 0.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 1.0, 1.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(0.0, 2.0, 0.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 1.0, 1.0, 0.0),
            Cartesian4::new(0.0, 1.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(0.0, 0.0, -1.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 1.0, 1.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
        ),
        (
            Cartesian4::new(0.0, 0.0, 2.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 1.0, 1.0, 0.0),
            Cartesian4::new(0.0, 0.0, 1.0, 0.0),
        ),
        (
            Cartesian4::new(-2.0, 3.0, 4.0, 0.0),
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 1.0, 1.0, 0.0),
            Cartesian4::new(0.0, 1.0, 1.0, 0.0),
        ),
        (
            Cartesian4::new(0.0, 0.0, 0.0, 0.0),
            Cartesian4::new(1.0, 2.0, 3.0, 0.0),
            Cartesian4::new(1.0, 2.0, 3.0, 0.0),
            Cartesian4::new(1.0, 2.0, 3.0, 0.0),
        ),
    ];
    for (value, min, max, expected) in cases {
        Cartesian4::clamp(&value, &min, &max, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn clamp_with_a_result_parameter() {
    let value = Cartesian4::new(-1.0, -1.0, -1.0, -1.0);
    let min = Cartesian4::new(0.0, 0.0, 0.0, 0.0);
    let max = Cartesian4::new(1.0, 1.0, 1.0, 1.0);
    let expected = Cartesian4::new(0.0, 0.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::clamp(&value, &min, &max, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn clamp_with_a_result_parameter_that_is_an_input_parameter() {
    let mut value = Cartesian4::new(-1.0, -1.0, -1.0, -1.0);
    let mut min = Cartesian4::new(0.0, 0.0, 0.0, 0.0);
    let mut max = Cartesian4::new(1.0, 1.0, 1.0, 1.0);
    let expected = Cartesian4::new(0.0, 0.0, 0.0, 0.0);

    let value_in = value;
    Cartesian4::clamp(&value_in, &min, &max, &mut value);
    assert_eq!(value, expected);

    Cartesian4::from_elements(-1.0, -1.0, -1.0, -1.0, &mut value);
    let min_in = min;
    Cartesian4::clamp(&value, &min_in, &max, &mut min);
    assert_eq!(min, expected);

    Cartesian4::from_elements(0.0, 0.0, 0.0, 0.0, &mut min);
    let max_in = max;
    Cartesian4::clamp(&value, &min, &max_in, &mut max);
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
    let cartesian = Cartesian4::new(3.0, 4.0, 5.0, 6.0);
    assert_eq!(Cartesian4::magnitude_squared(&cartesian), 86.0);
}

#[test]
fn magnitude() {
    let cartesian = Cartesian4::new(3.0, 4.0, 5.0, 6.0);
    assert_eq!(Cartesian4::magnitude(&cartesian), 86.0_f64.sqrt());
}

#[test]
fn distance() {
    let distance = Cartesian4::distance(
        &Cartesian4::new(1.0, 0.0, 0.0, 0.0),
        &Cartesian4::new(2.0, 0.0, 0.0, 0.0),
    );
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
    let distance_squared = Cartesian4::distance_squared(
        &Cartesian4::new(1.0, 0.0, 0.0, 0.0),
        &Cartesian4::new(3.0, 0.0, 0.0, 0.0),
    );
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
    let cartesian = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let expected_result = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let mut result = Cartesian4::default();
    Cartesian4::normalize(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn normalize_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian4::new(2.0, 0.0, 0.0, 0.0);
    let expected_result = Cartesian4::new(1.0, 0.0, 0.0, 0.0);
    let current = cartesian;
    Cartesian4::normalize(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn normalize_throws_with_zero_vector() {
    expect_to_throw_dev_error(|| {
        let mut result = Cartesian4::default();
        Cartesian4::normalize(&Cartesian4::ZERO, &mut result);
    });
}

#[test]
fn multiply_components_works_with_a_result_parameter() {
    let left = Cartesian4::new(2.0, 3.0, 6.0, 8.0);
    let right = Cartesian4::new(4.0, 5.0, 7.0, 9.0);
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(8.0, 15.0, 42.0, 72.0);
    Cartesian4::multiply_components(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn multiply_components_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian4::new(2.0, 3.0, 6.0, 8.0);
    let right = Cartesian4::new(4.0, 5.0, 7.0, 9.0);
    let expected_result = Cartesian4::new(8.0, 15.0, 42.0, 72.0);
    let left_in = left;
    Cartesian4::multiply_components(&left_in, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn divide_components_works_with_a_result_parameter() {
    let left = Cartesian4::new(2.0, 3.0, 6.0, 15.0);
    let right = Cartesian4::new(4.0, 5.0, 8.0, 2.0);
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(0.5, 0.6, 0.75, 7.5);
    Cartesian4::divide_components(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn divide_components_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian4::new(2.0, 3.0, 6.0, 15.0);
    let right = Cartesian4::new(4.0, 5.0, 8.0, 2.0);
    let expected_result = Cartesian4::new(0.5, 0.6, 0.75, 7.5);
    let left_in = left;
    Cartesian4::divide_components(&left_in, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn dot() {
    let left = Cartesian4::new(2.0, 3.0, 6.0, 8.0);
    let right = Cartesian4::new(4.0, 5.0, 7.0, 9.0);
    let expected_result = 137.0;
    let result = Cartesian4::dot(&left, &right);
    assert_eq!(result, expected_result);
}

#[test]
fn add_works_with_a_result_parameter() {
    let left = Cartesian4::new(2.0, 3.0, 6.0, 8.0);
    let right = Cartesian4::new(4.0, 5.0, 7.0, 9.0);
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(6.0, 8.0, 13.0, 17.0);
    Cartesian4::add(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn add_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian4::new(2.0, 3.0, 6.0, 8.0);
    let right = Cartesian4::new(4.0, 5.0, 7.0, 9.0);
    let expected_result = Cartesian4::new(6.0, 8.0, 13.0, 17.0);
    let left_in = left;
    Cartesian4::add(&left_in, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn subtract_works_with_a_result_parameter() {
    let left = Cartesian4::new(2.0, 3.0, 4.0, 8.0);
    let right = Cartesian4::new(1.0, 5.0, 7.0, 9.0);
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(1.0, -2.0, -3.0, -1.0);
    Cartesian4::subtract(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn subtract_works_with_this_result_parameter() {
    let mut left = Cartesian4::new(2.0, 3.0, 4.0, 8.0);
    let right = Cartesian4::new(1.0, 5.0, 7.0, 9.0);
    let expected_result = Cartesian4::new(1.0, -2.0, -3.0, -1.0);
    let left_in = left;
    Cartesian4::subtract(&left_in, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn multiply_by_scalar_with_a_result_parameter() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    let mut result = Cartesian4::default();
    let scalar = 2.0;
    let expected_result = Cartesian4::new(2.0, 4.0, 6.0, 8.0);
    Cartesian4::multiply_by_scalar(&cartesian, scalar, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn multiply_by_scalar_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    let scalar = 2.0;
    let expected_result = Cartesian4::new(2.0, 4.0, 6.0, 8.0);
    let current = cartesian;
    Cartesian4::multiply_by_scalar(&current, scalar, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn divide_by_scalar_with_a_result_parameter() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    let mut result = Cartesian4::default();
    let scalar = 2.0;
    let expected_result = Cartesian4::new(0.5, 1.0, 1.5, 2.0);
    Cartesian4::divide_by_scalar(&cartesian, scalar, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn divide_by_scalar_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    let scalar = 2.0;
    let expected_result = Cartesian4::new(0.5, 1.0, 1.5, 2.0);
    let current = cartesian;
    Cartesian4::divide_by_scalar(&current, scalar, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn negate_with_a_result_parameter() {
    let cartesian = Cartesian4::new(1.0, -2.0, -5.0, 4.0);
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(-1.0, 2.0, 5.0, -4.0);
    Cartesian4::negate(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn negate_with_a_result_parameter_that_is_an_input_parameter() {
    // JS `new Cartesian4(1.0, -2.0, -5.0)` defaults `w` to 0.0.
    let mut cartesian = Cartesian4::new(1.0, -2.0, -5.0, 0.0);
    let expected_result = Cartesian4::new(-1.0, 2.0, 5.0, 0.0);
    let current = cartesian;
    Cartesian4::negate(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn abs_with_a_result_parameter() {
    let cartesian = Cartesian4::new(1.0, -2.0, -4.0, -3.0);
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(1.0, 2.0, 4.0, 3.0);
    Cartesian4::abs(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn abs_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian4::new(1.0, -2.0, -4.0, -3.0);
    let expected_result = Cartesian4::new(1.0, 2.0, 4.0, 3.0);
    let current = cartesian;
    Cartesian4::abs(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn lerp_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut start = Cartesian4::new(4.0, 8.0, 10.0, 20.0);
    let end = Cartesian4::new(8.0, 20.0, 20.0, 30.0);
    let t = 0.25;
    let expected_result = Cartesian4::new(5.0, 11.0, 12.5, 22.5);
    let start_in = start;
    Cartesian4::lerp(&start_in, &end, t, &mut start);
    assert_eq!(start, expected_result);
}

#[test]
fn lerp_extrapolate_forward() {
    let start = Cartesian4::new(4.0, 8.0, 10.0, 20.0);
    let end = Cartesian4::new(8.0, 20.0, 20.0, 30.0);
    let t = 2.0;
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(12.0, 32.0, 30.0, 40.0);
    Cartesian4::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn lerp_extrapolate_backward() {
    let start = Cartesian4::new(4.0, 8.0, 10.0, 20.0);
    let end = Cartesian4::new(8.0, 20.0, 20.0, 30.0);
    let t = -1.0;
    let mut result = Cartesian4::default();
    let expected_result = Cartesian4::new(0.0, -4.0, 0.0, 10.0);
    Cartesian4::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn most_orthogonal_angle_is_x() {
    let v = Cartesian4::new(0.0, 1.0, 2.0, 3.0);
    let mut result = Cartesian4::default();
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_X);
}

#[test]
fn most_orthogonal_angle_is_y() {
    let v = Cartesian4::new(1.0, 0.0, 2.0, 3.0);
    let mut result = Cartesian4::default();
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_Y);
}

#[test]
fn most_orthogonal_angle_is_z() {
    let mut result = Cartesian4::default();

    let v = Cartesian4::new(2.0, 3.0, 0.0, 1.0);
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_Z);

    let v = Cartesian4::new(3.0, 2.0, 0.0, 1.0);
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_Z);
}

#[test]
fn most_orthogonal_angle_is_w() {
    let mut result = Cartesian4::default();

    let v = Cartesian4::new(1.0, 2.0, 3.0, 0.0);
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_W);

    let v = Cartesian4::new(2.0, 3.0, 1.0, 0.0);
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_W);

    let v = Cartesian4::new(3.0, 1.0, 2.0, 0.0);
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_W);

    let v = Cartesian4::new(3.0, 2.0, 1.0, 0.0);
    Cartesian4::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian4::UNIT_W);
}

#[test]
fn equals() {
    let cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    assert!(Cartesian4::equals(Some(&cartesian), Some(&Cartesian4::new(1.0, 2.0, 3.0, 4.0))));
    assert!(!Cartesian4::equals(Some(&cartesian), Some(&Cartesian4::new(2.0, 2.0, 3.0, 4.0))));
    assert!(!Cartesian4::equals(Some(&cartesian), Some(&Cartesian4::new(2.0, 1.0, 3.0, 4.0))));
    assert!(!Cartesian4::equals(Some(&cartesian), Some(&Cartesian4::new(1.0, 2.0, 4.0, 4.0))));
    assert!(!Cartesian4::equals(Some(&cartesian), Some(&Cartesian4::new(1.0, 2.0, 3.0, 5.0))));
    assert!(!Cartesian4::equals(Some(&cartesian), None));
}

#[test]
fn equals_epsilon() {
    let mut cartesian = Cartesian4::new(1.0, 2.0, 3.0, 4.0);
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 2.0, 3.0, 4.0), None, Some(0.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 2.0, 3.0, 4.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(2.0, 2.0, 3.0, 4.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 3.0, 3.0, 4.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 2.0, 4.0, 4.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 2.0, 3.0, 5.0), None, Some(1.0)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian4::new(2.0, 2.0, 3.0, 4.0), None, Some(CesiumMath::EPSILON6)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 3.0, 3.0, 4.0), None, Some(CesiumMath::EPSILON6)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 2.0, 4.0, 4.0), None, Some(CesiumMath::EPSILON6)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian4::new(1.0, 2.0, 3.0, 5.0), None, Some(CesiumMath::EPSILON6)));
    // JS `cartesian.equalsEpsilon(undefined, 1)` — mirrored statically:
    assert!(!Cartesian4::equals_epsilon(Some(&cartesian), None, None, Some(1.0)));

    cartesian = Cartesian4::new(3000000.0, 4000000.0, 5000000.0, 6000000.0);
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(3000000.0, 4000000.0, 5000000.0, 6000000.0), None, Some(0.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(3000000.2, 4000000.0, 5000000.0, 6000000.0), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(3000000.0, 4000000.2, 5000000.0, 6000000.0), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(3000000.0, 4000000.0, 5000000.2, 6000000.0), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(3000000.0, 4000000.0, 5000000.0, 6000000.2), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(cartesian.equals_epsilon_method(&Cartesian4::new(3000000.2, 4000000.2, 5000000.2, 6000000.2), Some(CesiumMath::EPSILON7), Some(CesiumMath::EPSILON7)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian4::new(3000000.2, 4000000.2, 5000000.2, 6000000.2), Some(CesiumMath::EPSILON9), Some(CesiumMath::EPSILON9)));
    assert!(!Cartesian4::equals_epsilon(Some(&cartesian), None, None, Some(1.0)));

    assert!(!Cartesian4::equals_epsilon(None, Some(&cartesian), None, Some(1.0)));
}

#[test]
fn to_string() {
    let cartesian = Cartesian4::new(1.123, 2.345, 6.789, 6.123);
    assert_eq!(cartesian.to_string(), "(1.123, 2.345, 6.789, 6.123)");
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
fn dot_throws_with_no_right_parameter() {}

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
fn most_orthogonal_axis_throws_with_no_cartesian_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn minimum_by_component_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn maximum_by_component_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn clamp_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn normalize_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn multiply_components_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn divide_components_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn add_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn subtract_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn multiply_by_scalar_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn divide_by_scalar_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn negate_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn abs_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn most_orthogonal_axis_throws_with_no_result() {}

#[test]
fn packs_and_unpacks_floating_point_values_for_representation_as_uint8_4_vectors() {
    fn test_float(float: f64) {
        let packed_float = Cartesian4::pack_float_new(float);
        assert!(0.0 <= packed_float.x && packed_float.x <= 255.0);
        assert!(0.0 <= packed_float.y && packed_float.y <= 255.0);
        assert!(0.0 <= packed_float.z && packed_float.z <= 255.0);
        assert!(0.0 <= packed_float.w && packed_float.w <= 255.0);

        let unpacked_float = Cartesian4::unpack_float(&packed_float);
        assert_eq!(unpacked_float, float);
    }

    fn test_float_nan(float: f64) {
        assert!(float.is_nan());
        let packed_float = Cartesian4::pack_float_new(float);
        let unpacked_float = Cartesian4::unpack_float(&packed_float);
        assert!(unpacked_float.is_nan());
    }

    fn test_float_out_of_range(float: f64) {
        let packed_float = Cartesian4::pack_float_new(float);
        let unpacked_float = Cartesian4::unpack_float(&packed_float);
        assert_eq!(unpacked_float, CesiumMath::sign(float) * f64::INFINITY);
    }

    test_float(0.0);
    test_float(-1.0);
    test_float(1.0);
    test_float(123.5);
    test_float(16777216.0);

    test_float(f64::INFINITY); // 64-bit infinity -> 32-bit infinity
    test_float(f64::NEG_INFINITY); // 64-bit infinity -> 32-bit infinity
    test_float_nan(f64::NAN); // 64-bit NaN -> 32bit NaN

    test_float_out_of_range(f64::MAX);
    test_float_out_of_range(f64::MIN);

    // `Float32Array` view values (already 32-bit-representable).
    test_float(f32::INFINITY as f64);
    test_float(f32::NEG_INFINITY as f64);
    test_float_nan(f32::NAN as f64);
}

//////////////////////////////////////////////////////////////////////
// createPackableSpecs(Cartesian4, new Cartesian4(1, 2, 3, 4), [1, 2, 3, 4])
//////////////////////////////////////////////////////////////////////

fn packable_instance() -> Cartesian4 {
    Cartesian4::new(1.0, 2.0, 3.0, 4.0)
}

const PACKED_INSTANCE: [f64; 4] = [1.0, 2.0, 3.0, 4.0];

#[test]
fn packable_can_pack() {
    let mut packed_array = vec![0.0; Cartesian4::PACKED_LENGTH];
    Cartesian4::pack(&packable_instance(), &mut packed_array, None);
    assert_eq!(packed_array.len(), Cartesian4::PACKED_LENGTH);
    assert_c4_eq_epsilon(
        &Cartesian4::new(
            PACKED_INSTANCE[0],
            PACKED_INSTANCE[1],
            PACKED_INSTANCE[2],
            PACKED_INSTANCE[3],
        ),
        &Cartesian4::new(
            packed_array[0],
            packed_array[1],
            packed_array[2],
            packed_array[3],
        ),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn packable_can_roundtrip() {
    let mut packed_array = vec![0.0; Cartesian4::PACKED_LENGTH];
    Cartesian4::pack(&packable_instance(), &mut packed_array, None);
    let result = Cartesian4::unpack_new(&packed_array, None);
    assert_eq!(packable_instance(), result);
}

#[test]
fn packable_can_unpack() {
    let result = Cartesian4::unpack_new(&PACKED_INSTANCE, None);
    assert_eq!(result, packable_instance());
}

#[test]
fn packable_can_pack_with_starting_index() {
    let mut packed_array = vec![0.0; 1 + Cartesian4::PACKED_LENGTH];
    let expected: Vec<f64> = [0.0_f64].iter().chain(PACKED_INSTANCE.iter()).copied().collect();
    Cartesian4::pack(&packable_instance(), &mut packed_array, Some(1));
    for i in 0..expected.len() {
        assert_approx_eq_f64!(packed_array[i], expected[i], CesiumMath::EPSILON15);
    }
}

#[test]
fn packable_can_unpack_with_starting_index() {
    let packed_array: Vec<f64> = [0.0_f64].iter().chain(PACKED_INSTANCE.iter()).copied().collect();
    let result = Cartesian4::unpack_new(&packed_array, Some(1));
    assert_eq!(packable_instance(), result);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_undefined_throws_group() {}

// JS its: pack throws with undefined value / undefined array; unpack throws
// with undefined array.

//////////////////////////////////////////////////////////////////////
// createPackableArraySpecs(Cartesian4, [(1,2,3,4),(5,6,7,8)], [1..=8], 4)
//////////////////////////////////////////////////////////////////////

fn packable_unpacked_array() -> Vec<Cartesian4> {
    vec![
        Cartesian4::new(1.0, 2.0, 3.0, 4.0),
        Cartesian4::new(5.0, 6.0, 7.0, 8.0),
    ]
}

const PACKED_ARRAY: [f64; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

const PACKABLE_STRIDE: usize = 4;

#[test]
fn packable_array_can_pack() {
    let actual_packed_array = Cartesian4::pack_array(&packable_unpacked_array(), None);
    assert_eq!(actual_packed_array.len(), PACKED_ARRAY.len());
    assert_eq!(actual_packed_array, PACKED_ARRAY);
}

#[test]
fn packable_array_can_roundtrip() {
    let actual_packed_array = Cartesian4::pack_array(&packable_unpacked_array(), None);
    let result = Cartesian4::unpack_array(&actual_packed_array, None);
    assert_eq!(result, packable_unpacked_array());
}

#[test]
fn packable_array_can_unpack() {
    let result = Cartesian4::unpack_array(&PACKED_ARRAY, None);
    assert_eq!(result, packable_unpacked_array());
}

#[test]
#[ignore = "DEVIATION: Rust has a single Vec<f64> representation; JS typed-array branch not ported"]
fn packable_array_pack_array_works_with_typed_arrays() {}

#[test]
fn packable_array_pack_array_resizes_arrays_as_needed() {
    let empty_array: Vec<f64> = Vec::new();
    let result = Cartesian4::pack_array(&packable_unpacked_array(), Some(empty_array));
    assert_eq!(result, PACKED_ARRAY);

    let larger_array = vec![0.0; PACKED_ARRAY.len() + 1];
    let result = Cartesian4::pack_array(&packable_unpacked_array(), Some(larger_array));
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
    let array = Cartesian4::unpack_array(&PACKED_ARRAY, None);
    assert_eq!(array, packable_unpacked_array());
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_array_unpack_array_throws_with_undefined_array() {}

#[test]
fn packable_array_unpack_array_works_with_a_result_parameter() {
    let array: Vec<Cartesian4> = Vec::new();
    let result = Cartesian4::unpack_array(&PACKED_ARRAY, Some(array));
    assert_eq!(result, packable_unpacked_array());

    let array: Vec<Cartesian4> = vec![Cartesian4::default(); packable_unpacked_array().len()];
    let result = Cartesian4::unpack_array(&PACKED_ARRAY, Some(array));
    assert_eq!(result, packable_unpacked_array());
}

#[test]
fn packable_array_unpack_array_throws_with_array_less_than_the_minimum_length() {
    expect_to_throw_dev_error(|| {
        Cartesian4::unpack_array(&[1.0], None);
    });
}

#[test]
fn unpack_array_throws_with_array_not_multiple_of_stride() {
    expect_to_throw_dev_error(|| {
        Cartesian4::unpack_array(&vec![1.0; PACKABLE_STRIDE + 1], None);
    });
}
