//! Mirrors packages/engine/Specs/Core/Cartesian3Spec.js
//!
//! JS `undefined`-argument DeveloperError cases are statically impossible in
//! Rust; they are mirrored as `#[ignore]` stubs. Scalar `fromDegrees` /
//! `fromRadians` cases are covered now that `Ellipsoid` is ported; the
//! `*Array` variants remain `#[ignore]`d until
//! `Ellipsoid.cartographicArrayToCartesianArray` is backfilled.
//! `Ellipsoid.default = Ellipsoid.MOON` is emulated via
//! `set_ellipsoid_radii_squared` (mirrors the JS side effect). Shared
//! generators `createPackableSpecs` / `createPackableArraySpecs` are inlined.

use cesium_core::cartesian3::{self, Cartesian3};
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::spherical::Spherical;
use cesium_test_utils::{assert_approx_eq_f64, expect_to_throw_dev_error};

const PI: f64 = std::f64::consts::PI;

/// JS `toEqualEpsilon` matcher: componentwise absolute-epsilon comparison.
fn assert_c3_eq_epsilon(expected: &Cartesian3, actual: &Cartesian3, epsilon: f64) {
    assert_approx_eq_f64!(expected.x, actual.x, epsilon);
    assert_approx_eq_f64!(expected.y, actual.y, epsilon);
    assert_approx_eq_f64!(expected.z, actual.z, epsilon);
}

/// Componentwise relative-epsilon comparison (JS `toEqualEpsilon` with a
/// single epsilon uses relative tolerance).
fn assert_c3_eq_rel_epsilon(expected: &Cartesian3, actual: &Cartesian3, epsilon: f64) {
    assert_approx_eq_f64!(expected.x, actual.x, 0.0, epsilon);
    assert_approx_eq_f64!(expected.y, actual.y, 0.0, epsilon);
    assert_approx_eq_f64!(expected.z, actual.z, 0.0, epsilon);
}

const WGS84_RADII_SQUARED: Cartesian3 = Cartesian3::new(
    6378137.0 * 6378137.0,
    6378137.0 * 6378137.0,
    6356752.3142451793 * 6356752.3142451793,
);

/// `Ellipsoid.MOON` radii squared (`CesiumMath.LUNAR_RADIUS = 1737400.0`).
const MOON_RADII_SQUARED: Cartesian3 = Cartesian3::new(
    1737400.0 * 1737400.0,
    1737400.0 * 1737400.0,
    1737400.0 * 1737400.0,
);

/// Emulates the spec's `afterEach { Ellipsoid.default = Ellipsoid.WGS84; }`
/// for the duration of a single closure (which sets `Ellipsoid.default =
/// Ellipsoid.MOON` first, like the JS tests do).
fn with_moon_default<T>(f: impl FnOnce() -> T) -> T {
    cartesian3::set_ellipsoid_radii_squared(MOON_RADII_SQUARED);
    let result = f();
    cartesian3::set_ellipsoid_radii_squared(WGS84_RADII_SQUARED);
    result
}

// describe("Core/Cartesian3")

#[test]
fn construct_with_default_values() {
    let cartesian = Cartesian3::default();
    assert_eq!(cartesian.x, 0.0);
    assert_eq!(cartesian.y, 0.0);
    assert_eq!(cartesian.z, 0.0);
}

#[test]
fn construct_with_all_values() {
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    assert_eq!(cartesian.x, 1.0);
    assert_eq!(cartesian.y, 2.0);
    assert_eq!(cartesian.z, 3.0);
}

const FORTY_FIVE_DEGREES: f64 = PI / 4.0;
const SIXTY_DEGREES: f64 = PI / 3.0;

fn spherical_cartesian() -> Cartesian3 {
    Cartesian3::new(1.0, 3.0_f64.sqrt(), -2.0)
}

fn spherical_value() -> Spherical {
    Spherical::new(
        SIXTY_DEGREES,
        FORTY_FIVE_DEGREES + PI / 2.0,
        8.0_f64.sqrt(),
    )
}

#[test]
fn convert_spherical_to_an_existing_cartesian3_instance() {
    let mut existing = Cartesian3::default();
    Cartesian3::from_spherical(&spherical_value(), &mut existing);
    assert_c3_eq_epsilon(&spherical_cartesian(), &existing, CesiumMath::EPSILON15);
}

#[test]
fn from_array_with_an_offset_creates_a_cartesian3() {
    let cartesian = Cartesian3::from_array_new(&[0.0, 1.0, 2.0, 3.0, 0.0], Some(1));
    assert_eq!(cartesian, Cartesian3::new(1.0, 2.0, 3.0));
}

#[test]
fn from_array_creates_a_cartesian3_with_a_result_parameter() {
    let mut cartesian = Cartesian3::default();
    Cartesian3::from_array(&[1.0, 2.0, 3.0], Some(0), &mut cartesian);
    assert_eq!(cartesian, Cartesian3::new(1.0, 2.0, 3.0));
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_array_throws_without_values() {}

#[test]
fn clone_with_a_result_parameter() {
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    Cartesian3::clone_into(&cartesian, &mut result);
    assert_eq!(cartesian, result);
}

#[test]
fn clone_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    let current = cartesian;
    Cartesian3::clone_into(&current, &mut cartesian);
    assert_eq!(cartesian, Cartesian3::new(1.0, 2.0, 3.0));
}

#[test]
fn maximum_component_works_when_x_is_greater() {
    let cartesian = Cartesian3::new(2.0, 1.0, 0.0);
    assert_eq!(Cartesian3::maximum_component(&cartesian), cartesian.x);
}

#[test]
fn maximum_component_works_when_y_is_greater() {
    let cartesian = Cartesian3::new(1.0, 2.0, 0.0);
    assert_eq!(Cartesian3::maximum_component(&cartesian), cartesian.y);
}

#[test]
fn maximum_component_works_when_z_is_greater() {
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    assert_eq!(Cartesian3::maximum_component(&cartesian), cartesian.z);
}

#[test]
fn minimum_component_works_when_x_is_lesser() {
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    assert_eq!(Cartesian3::minimum_component(&cartesian), cartesian.x);
}

#[test]
fn minimum_component_works_when_y_is_lesser() {
    let cartesian = Cartesian3::new(2.0, 1.0, 3.0);
    assert_eq!(Cartesian3::minimum_component(&cartesian), cartesian.y);
}

#[test]
fn minimum_component_works_when_z_is_lesser() {
    let cartesian = Cartesian3::new(2.0, 1.0, 0.0);
    assert_eq!(Cartesian3::minimum_component(&cartesian), cartesian.z);
}

#[test]
fn minimum_by_component() {
    let mut result = Cartesian3::default();

    let cases = [
        (
            Cartesian3::new(2.0, 0.0, 0.0),
            Cartesian3::new(1.0, 0.0, 0.0),
            Cartesian3::new(1.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(1.0, 0.0, 0.0),
            Cartesian3::new(2.0, 0.0, 0.0),
            Cartesian3::new(1.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(2.0, -15.0, 0.0),
            Cartesian3::new(1.0, -20.0, 0.0),
            Cartesian3::new(1.0, -20.0, 0.0),
        ),
        (
            Cartesian3::new(2.0, -20.0, 0.0),
            Cartesian3::new(1.0, -15.0, 0.0),
            Cartesian3::new(1.0, -20.0, 0.0),
        ),
        (
            Cartesian3::new(2.0, -15.0, 26.4),
            Cartesian3::new(1.0, -20.0, 26.5),
            Cartesian3::new(1.0, -20.0, 26.4),
        ),
        (
            Cartesian3::new(2.0, -15.0, 26.5),
            Cartesian3::new(1.0, -20.0, 26.4),
            Cartesian3::new(1.0, -20.0, 26.4),
        ),
    ];
    for (first, second, expected) in cases {
        Cartesian3::minimum_by_component(&first, &second, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn minimum_by_component_with_a_result_parameter() {
    let first = Cartesian3::new(2.0, 0.0, 0.0);
    let second = Cartesian3::new(1.0, 0.0, 0.0);
    let expected = Cartesian3::new(1.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_with_a_result_parameter_that_is_an_input_parameter() {
    let mut first = Cartesian3::new(2.0, 0.0, 0.0);
    let mut second = Cartesian3::new(1.0, 0.0, 0.0);
    let expected = Cartesian3::new(1.0, 0.0, 0.0);

    let first_in = first;
    Cartesian3::minimum_by_component(&first_in, &second, &mut first);
    assert_eq!(first, expected);

    first.x = 1.0;
    second.x = 2.0;
    let second_in = second;
    Cartesian3::minimum_by_component(&first, &second_in, &mut second);
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
    let first = Cartesian3::new(2.0, 0.0, 0.0);
    let mut second = Cartesian3::new(1.0, 0.0, 0.0);
    let mut expected = Cartesian3::new(1.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.x = 3.0;
    expected.x = 2.0;
    Cartesian3::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_works_when_firsts_or_seconds_y_is_lesser() {
    let first = Cartesian3::new(0.0, 2.0, 0.0);
    let mut second = Cartesian3::new(0.0, 1.0, 0.0);
    let mut expected = Cartesian3::new(0.0, 1.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.y = 3.0;
    expected.y = 2.0;
    Cartesian3::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn minimum_by_component_works_when_firsts_or_seconds_z_is_lesser() {
    let first = Cartesian3::new(0.0, 0.0, 2.0);
    let mut second = Cartesian3::new(0.0, 0.0, 1.0);
    let mut expected = Cartesian3::new(0.0, 0.0, 1.0);
    let mut result = Cartesian3::default();
    Cartesian3::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.z = 3.0;
    expected.z = 2.0;
    Cartesian3::minimum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component() {
    let mut result = Cartesian3::default();

    let cases = [
        (
            Cartesian3::new(2.0, 0.0, 0.0),
            Cartesian3::new(1.0, 0.0, 0.0),
            Cartesian3::new(2.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(1.0, 0.0, 0.0),
            Cartesian3::new(2.0, 0.0, 0.0),
            Cartesian3::new(2.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(2.0, -15.0, 0.0),
            Cartesian3::new(1.0, -20.0, 0.0),
            Cartesian3::new(2.0, -15.0, 0.0),
        ),
        (
            Cartesian3::new(2.0, -20.0, 0.0),
            Cartesian3::new(1.0, -15.0, 0.0),
            Cartesian3::new(2.0, -15.0, 0.0),
        ),
        (
            Cartesian3::new(2.0, -15.0, 26.4),
            Cartesian3::new(1.0, -20.0, 26.5),
            Cartesian3::new(2.0, -15.0, 26.5),
        ),
        (
            Cartesian3::new(2.0, -15.0, 26.5),
            Cartesian3::new(1.0, -20.0, 26.4),
            Cartesian3::new(2.0, -15.0, 26.5),
        ),
    ];
    for (first, second, expected) in cases {
        Cartesian3::maximum_by_component(&first, &second, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn maximum_by_component_with_a_result_parameter() {
    let first = Cartesian3::new(2.0, 0.0, 0.0);
    let second = Cartesian3::new(1.0, 0.0, 0.0);
    let expected = Cartesian3::new(2.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_with_a_result_parameter_that_is_an_input_parameter() {
    let mut first = Cartesian3::new(2.0, 0.0, 0.0);
    let mut second = Cartesian3::new(1.0, 0.0, 0.0);
    let expected = Cartesian3::new(2.0, 0.0, 0.0);

    let first_in = first;
    Cartesian3::maximum_by_component(&first_in, &second, &mut first);
    assert_eq!(first, expected);

    first.x = 1.0;
    second.x = 2.0;
    let second_in = second;
    Cartesian3::maximum_by_component(&first, &second_in, &mut second);
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
    let first = Cartesian3::new(2.0, 0.0, 0.0);
    let mut second = Cartesian3::new(1.0, 0.0, 0.0);
    let mut expected = Cartesian3::new(2.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.x = 3.0;
    expected.x = 3.0;
    Cartesian3::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_works_when_firsts_or_seconds_y_is_greater() {
    let first = Cartesian3::new(0.0, 2.0, 0.0);
    let mut second = Cartesian3::new(0.0, 1.0, 0.0);
    let mut expected = Cartesian3::new(0.0, 2.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.y = 3.0;
    expected.y = 3.0;
    Cartesian3::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn maximum_by_component_works_when_firsts_or_seconds_z_is_greater() {
    let first = Cartesian3::new(0.0, 0.0, 2.0);
    let mut second = Cartesian3::new(0.0, 0.0, 1.0);
    let mut expected = Cartesian3::new(0.0, 0.0, 2.0);
    let mut result = Cartesian3::default();
    Cartesian3::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);

    second.z = 3.0;
    expected.z = 3.0;
    Cartesian3::maximum_by_component(&first, &second, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn clamp() {
    let mut result = Cartesian3::default();

    let cases = [
        (
            Cartesian3::new(-1.0, 0.0, 0.0),
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(0.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(2.0, 0.0, 0.0),
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(1.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(0.0, -1.0, 0.0),
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(0.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(0.0, 2.0, 0.0),
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(0.0, 1.0, 0.0),
        ),
        (
            Cartesian3::new(0.0, 0.0, -1.0),
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(0.0, 0.0, 0.0),
        ),
        (
            Cartesian3::new(0.0, 0.0, 2.0),
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(0.0, 0.0, 1.0),
        ),
        (
            Cartesian3::new(-2.0, 3.0, 4.0),
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 1.0, 1.0),
            Cartesian3::new(0.0, 1.0, 1.0),
        ),
        (
            Cartesian3::new(0.0, 0.0, 0.0),
            Cartesian3::new(1.0, 2.0, 3.0),
            Cartesian3::new(1.0, 2.0, 3.0),
            Cartesian3::new(1.0, 2.0, 3.0),
        ),
    ];
    for (value, min, max, expected) in cases {
        Cartesian3::clamp(&value, &min, &max, &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn clamp_with_a_result_parameter() {
    let value = Cartesian3::new(-1.0, -1.0, -1.0);
    let min = Cartesian3::new(0.0, 0.0, 0.0);
    let max = Cartesian3::new(1.0, 1.0, 1.0);
    let expected = Cartesian3::new(0.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::clamp(&value, &min, &max, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn clamp_with_a_result_parameter_that_is_an_input_parameter() {
    let mut value = Cartesian3::new(-1.0, -1.0, -1.0);
    let mut min = Cartesian3::new(0.0, 0.0, 0.0);
    let mut max = Cartesian3::new(1.0, 1.0, 1.0);
    let expected = Cartesian3::new(0.0, 0.0, 0.0);

    let value_in = value;
    Cartesian3::clamp(&value_in, &min, &max, &mut value);
    assert_eq!(value, expected);

    Cartesian3::from_elements(-1.0, -1.0, -1.0, &mut value);
    let min_in = min;
    Cartesian3::clamp(&value, &min_in, &max, &mut min);
    assert_eq!(min, expected);

    Cartesian3::from_elements(0.0, 0.0, 0.0, &mut min);
    let max_in = max;
    Cartesian3::clamp(&value, &min, &max_in, &mut max);
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
    let cartesian = Cartesian3::new(3.0, 4.0, 5.0);
    assert_eq!(Cartesian3::magnitude_squared(&cartesian), 50.0);
}

#[test]
fn magnitude() {
    let cartesian = Cartesian3::new(3.0, 4.0, 5.0);
    assert_eq!(Cartesian3::magnitude(&cartesian), 50.0_f64.sqrt());
}

#[test]
fn distance() {
    let distance = Cartesian3::distance(&Cartesian3::new(1.0, 0.0, 0.0), &Cartesian3::new(2.0, 0.0, 0.0));
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
    let distance_squared = Cartesian3::distance_squared(
        &Cartesian3::new(1.0, 0.0, 0.0),
        &Cartesian3::new(3.0, 0.0, 0.0),
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
    let cartesian = Cartesian3::new(2.0, 0.0, 0.0);
    let expected_result = Cartesian3::new(1.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::normalize(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn normalize_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian3::new(2.0, 0.0, 0.0);
    let expected_result = Cartesian3::new(1.0, 0.0, 0.0);
    let current = cartesian;
    Cartesian3::normalize(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn normalize_throws_with_zero_vector() {
    expect_to_throw_dev_error(|| {
        let mut result = Cartesian3::default();
        Cartesian3::normalize(&Cartesian3::ZERO, &mut result);
    });
}

#[test]
fn multiply_components_works_with_a_result_parameter() {
    let left = Cartesian3::new(2.0, 3.0, 6.0);
    let right = Cartesian3::new(4.0, 5.0, 7.0);
    let expected_result = Cartesian3::new(8.0, 15.0, 42.0);
    let mut result = Cartesian3::default();
    Cartesian3::multiply_components(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn multiply_components_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian3::new(2.0, 3.0, 6.0);
    let right = Cartesian3::new(4.0, 5.0, 7.0);
    let expected_result = Cartesian3::new(8.0, 15.0, 42.0);
    let current = left;
    Cartesian3::multiply_components(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn divide_components_works_with_a_result_parameter() {
    let left = Cartesian3::new(2.0, 3.0, 6.0);
    let right = Cartesian3::new(4.0, 5.0, 8.0);
    let expected_result = Cartesian3::new(0.5, 0.6, 0.75);
    let mut result = Cartesian3::default();
    Cartesian3::divide_components(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn divide_components_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian3::new(2.0, 3.0, 6.0);
    let right = Cartesian3::new(4.0, 5.0, 8.0);
    let expected_result = Cartesian3::new(0.5, 0.6, 0.75);
    let current = left;
    Cartesian3::divide_components(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn dot() {
    let left = Cartesian3::new(2.0, 3.0, 6.0);
    let right = Cartesian3::new(4.0, 5.0, 7.0);
    let expected_result = 65.0;
    let result = Cartesian3::dot(&left, &right);
    assert_eq!(result, expected_result);
}

#[test]
fn add_works_with_a_result_parameter() {
    let left = Cartesian3::new(2.0, 3.0, 6.0);
    let right = Cartesian3::new(4.0, 5.0, 7.0);
    let expected_result = Cartesian3::new(6.0, 8.0, 13.0);
    let mut result = Cartesian3::default();
    Cartesian3::add(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn add_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian3::new(2.0, 3.0, 6.0);
    let right = Cartesian3::new(4.0, 5.0, 7.0);
    let expected_result = Cartesian3::new(6.0, 8.0, 13.0);
    let current = left;
    Cartesian3::add(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn subtract_works_with_a_result_parameter() {
    let left = Cartesian3::new(2.0, 3.0, 4.0);
    let right = Cartesian3::new(1.0, 5.0, 7.0);
    let expected_result = Cartesian3::new(1.0, -2.0, -3.0);
    let mut result = Cartesian3::default();
    Cartesian3::subtract(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn subtract_works_with_this_result_parameter() {
    let mut left = Cartesian3::new(2.0, 3.0, 4.0);
    let right = Cartesian3::new(1.0, 5.0, 7.0);
    let expected_result = Cartesian3::new(1.0, -2.0, -3.0);
    let current = left;
    Cartesian3::subtract(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn multiply_by_scalar_with_a_result_parameter() {
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    let scalar = 2.0;
    let expected_result = Cartesian3::new(2.0, 4.0, 6.0);
    let mut result = Cartesian3::default();
    Cartesian3::multiply_by_scalar(&cartesian, scalar, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn multiply_by_scalar_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    let scalar = 2.0;
    let expected_result = Cartesian3::new(2.0, 4.0, 6.0);
    let current = cartesian;
    Cartesian3::multiply_by_scalar(&current, scalar, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn divide_by_scalar_with_a_result_parameter() {
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    let scalar = 2.0;
    let expected_result = Cartesian3::new(0.5, 1.0, 1.5);
    let mut result = Cartesian3::default();
    Cartesian3::divide_by_scalar(&cartesian, scalar, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn divide_by_scalar_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    let scalar = 2.0;
    let expected_result = Cartesian3::new(0.5, 1.0, 1.5);
    let current = cartesian;
    Cartesian3::divide_by_scalar(&current, scalar, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn negate_without_a_result_parameter() {
    let cartesian = Cartesian3::new(1.0, -2.0, -5.0);
    let expected_result = Cartesian3::new(-1.0, 2.0, 5.0);
    let result = Cartesian3::negate_new(&cartesian);
    assert_eq!(result, expected_result);
}

#[test]
fn negate_with_a_result_parameter() {
    let cartesian = Cartesian3::new(1.0, -2.0, -5.0);
    let expected_result = Cartesian3::new(-1.0, 2.0, 5.0);
    let mut result = Cartesian3::default();
    Cartesian3::negate(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn negate_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian3::new(1.0, -2.0, -5.0);
    let expected_result = Cartesian3::new(-1.0, 2.0, 5.0);
    let current = cartesian;
    Cartesian3::negate(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn abs_without_a_result_parameter() {
    let cartesian = Cartesian3::new(1.0, -2.0, -4.0);
    let expected_result = Cartesian3::new(1.0, 2.0, 4.0);
    let result = Cartesian3::abs_new(&cartesian);
    assert_eq!(result, expected_result);
}

#[test]
fn abs_with_a_result_parameter() {
    let cartesian = Cartesian3::new(1.0, -2.0, -4.0);
    let expected_result = Cartesian3::new(1.0, 2.0, 4.0);
    let mut result = Cartesian3::default();
    Cartesian3::abs(&cartesian, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn abs_with_a_result_parameter_that_is_an_input_parameter() {
    let mut cartesian = Cartesian3::new(1.0, -2.0, -4.0);
    let expected_result = Cartesian3::new(1.0, 2.0, 4.0);
    let current = cartesian;
    Cartesian3::abs(&current, &mut cartesian);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn lerp_works_with_a_result_parameter() {
    let start = Cartesian3::new(4.0, 8.0, 10.0);
    let end = Cartesian3::new(8.0, 20.0, 20.0);
    let t = 0.25;
    let expected_result = Cartesian3::new(5.0, 11.0, 12.5);
    let mut result = Cartesian3::default();
    Cartesian3::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn lerp_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut start = Cartesian3::new(4.0, 8.0, 10.0);
    let end = Cartesian3::new(8.0, 20.0, 20.0);
    let t = 0.25;
    let expected_result = Cartesian3::new(5.0, 11.0, 12.5);
    let current = start;
    Cartesian3::lerp(&current, &end, t, &mut start);
    assert_eq!(start, expected_result);
}

#[test]
fn lerp_extrapolate_forward() {
    let start = Cartesian3::new(4.0, 8.0, 10.0);
    let end = Cartesian3::new(8.0, 20.0, 20.0);
    let t = 2.0;
    let expected_result = Cartesian3::new(12.0, 32.0, 30.0);
    let mut result = Cartesian3::default();
    Cartesian3::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn lerp_extrapolate_backward() {
    let start = Cartesian3::new(4.0, 8.0, 10.0);
    let end = Cartesian3::new(8.0, 20.0, 20.0);
    let t = -1.0;
    let expected_result = Cartesian3::new(0.0, -4.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::lerp(&start, &end, t, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn angle_between_works_for_right_angles() {
    let x = Cartesian3::UNIT_X;
    let y = Cartesian3::UNIT_Y;
    assert_eq!(Cartesian3::angle_between(&x, &y), CesiumMath::PI_OVER_TWO);
    assert_eq!(Cartesian3::angle_between(&y, &x), CesiumMath::PI_OVER_TWO);
}

#[test]
fn angle_between_works_for_acute_angles() {
    let x = Cartesian3::new(0.0, 1.0, 0.0);
    let y = Cartesian3::new(1.0, 1.0, 0.0);
    assert_approx_eq_f64!(
        Cartesian3::angle_between(&x, &y),
        CesiumMath::PI_OVER_FOUR,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        Cartesian3::angle_between(&y, &x),
        CesiumMath::PI_OVER_FOUR,
        CesiumMath::EPSILON14
    );
}

#[test]
fn angle_between_works_for_obtuse_angles() {
    let x = Cartesian3::new(0.0, 1.0, 0.0);
    let y = Cartesian3::new(0.0, -1.0, -1.0);
    assert_approx_eq_f64!(
        Cartesian3::angle_between(&x, &y),
        (PI * 3.0) / 4.0,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        Cartesian3::angle_between(&y, &x),
        (PI * 3.0) / 4.0,
        CesiumMath::EPSILON14
    );
}

#[test]
fn angle_between_works_for_zero_angles() {
    let x = Cartesian3::UNIT_X;
    assert_eq!(Cartesian3::angle_between(&x, &x), 0.0);
}

#[test]
fn most_orthogonal_angle_is_x() {
    let v = Cartesian3::new(0.0, 1.0, 2.0);
    let mut result = Cartesian3::default();
    Cartesian3::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian3::UNIT_X);
}

#[test]
fn most_orthogonal_angle_is_y() {
    let v = Cartesian3::new(1.0, 0.0, 2.0);
    let mut result = Cartesian3::default();
    Cartesian3::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian3::UNIT_Y);
}

#[test]
fn most_orthogonal_angle_is_z() {
    let mut result = Cartesian3::default();

    let v = Cartesian3::new(1.0, 3.0, 0.0);
    Cartesian3::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian3::UNIT_Z);

    let v = Cartesian3::new(3.0, 1.0, 0.0);
    Cartesian3::most_orthogonal_axis(&v, &mut result);
    assert_eq!(result, Cartesian3::UNIT_Z);
}

#[test]
fn equals() {
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    assert!(Cartesian3::equals(Some(&cartesian), Some(&Cartesian3::new(1.0, 2.0, 3.0))));
    assert!(!Cartesian3::equals(Some(&cartesian), Some(&Cartesian3::new(2.0, 2.0, 3.0))));
    assert!(!Cartesian3::equals(Some(&cartesian), Some(&Cartesian3::new(2.0, 1.0, 3.0))));
    assert!(!Cartesian3::equals(Some(&cartesian), Some(&Cartesian3::new(1.0, 2.0, 4.0))));
    assert!(!Cartesian3::equals(Some(&cartesian), None));
}

#[test]
fn equals_epsilon() {
    let mut cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    assert!(cartesian.equals_epsilon_method(&Cartesian3::new(1.0, 2.0, 3.0), None, Some(0.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian3::new(1.0, 2.0, 3.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian3::new(2.0, 2.0, 3.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian3::new(1.0, 3.0, 3.0), None, Some(1.0)));
    assert!(cartesian.equals_epsilon_method(&Cartesian3::new(1.0, 2.0, 4.0), None, Some(1.0)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian3::new(2.0, 2.0, 3.0), None, Some(CesiumMath::EPSILON6)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian3::new(1.0, 3.0, 3.0), None, Some(CesiumMath::EPSILON6)));
    assert!(!cartesian.equals_epsilon_method(&Cartesian3::new(1.0, 2.0, 4.0), None, Some(CesiumMath::EPSILON6)));

    cartesian = Cartesian3::new(3000000.0, 4000000.0, 5000000.0);
    assert!(cartesian.equals_epsilon_method(&Cartesian3::new(3000000.0, 4000000.0, 5000000.0), None, Some(0.0)));
    assert!(cartesian.equals_epsilon_method(
        &Cartesian3::new(3000000.2, 4000000.0, 5000000.0),
        Some(CesiumMath::EPSILON7),
        Some(CesiumMath::EPSILON7)
    ));
    assert!(cartesian.equals_epsilon_method(
        &Cartesian3::new(3000000.0, 4000000.2, 5000000.0),
        Some(CesiumMath::EPSILON7),
        Some(CesiumMath::EPSILON7)
    ));
    assert!(cartesian.equals_epsilon_method(
        &Cartesian3::new(3000000.0, 4000000.0, 5000000.2),
        Some(CesiumMath::EPSILON7),
        Some(CesiumMath::EPSILON7)
    ));
    assert!(cartesian.equals_epsilon_method(
        &Cartesian3::new(3000000.2, 4000000.2, 5000000.2),
        Some(CesiumMath::EPSILON7),
        Some(CesiumMath::EPSILON7)
    ));
    assert!(!cartesian.equals_epsilon_method(
        &Cartesian3::new(3000000.2, 4000000.2, 5000000.2),
        Some(CesiumMath::EPSILON9),
        Some(CesiumMath::EPSILON9)
    ));

    // JS `Cartesian3.equalsEpsilon(undefined, cartesian, 1)` -> false.
    assert!(!Cartesian3::equals_epsilon(
        None,
        Some(&cartesian),
        Some(1.0),
        Some(1.0)
    ));
}

#[test]
fn to_string() {
    let cartesian = Cartesian3::new(1.123, 2.345, 6.789);
    assert_eq!(cartesian.to_string(), "(1.123, 2.345, 6.789)");
}

#[test]
fn cross_works_with_a_result_parameter() {
    let left = Cartesian3::new(1.0, 2.0, 5.0);
    let right = Cartesian3::new(4.0, 3.0, 6.0);
    let expected_result = Cartesian3::new(-3.0, 14.0, -5.0);
    let mut result = Cartesian3::default();
    Cartesian3::cross(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
fn cross_works_with_a_result_parameter_that_is_an_input_parameter() {
    let mut left = Cartesian3::new(1.0, 2.0, 5.0);
    let right = Cartesian3::new(4.0, 3.0, 6.0);
    let expected_result = Cartesian3::new(-3.0, 14.0, -5.0);
    let current = left;
    Cartesian3::cross(&current, &right, &mut left);
    assert_eq!(left, expected_result);
}

#[test]
fn midpoint_works_with_a_result_parameter() {
    let left = Cartesian3::new(0.0, 0.0, 6.0);
    let right = Cartesian3::new(0.0, 0.0, -6.0);
    let expected_result = Cartesian3::new(0.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::midpoint(&left, &right, &mut result);
    assert_eq!(result, expected_result);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn midpoint_throws_with_no_left() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn midpoint_throws_with_no_right() {}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn midpoint_throws_with_no_result() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_spherical_throws_with_no_spherical_parameter() {}

#[test]
fn from_spherical_work_with_no_result_parameter() {
    // JS `not.toThrowDeveloperError()` — just exercise the allocating form.
    let _ = Cartesian3::from_spherical_new(&spherical_value());
}

#[test]
#[ignore = "JS undefined-argument behavior; statically impossible in Rust"]
fn clone_returns_undefined_with_no_parameter() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn undefined_argument_throws_group_1() {}

// JS its (each statically impossible in Rust): maximumComponent/minimumComponent/
// magnitudeSquared/magnitude/normalize with no parameter; dot/multiplyComponents/
// divideComponents/add/subtract/multiplyByScalar/divideByScalar/negate/abs/lerp/
// angleBetween/mostOrthogonalAxis/cross with missing parameters.

#[test]
fn from_elements_returns_a_cartesian3_with_correct_coordinates() {
    let cartesian = Cartesian3::from_elements_new(2.0, 2.0, 4.0);
    let expected_result = Cartesian3::new(2.0, 2.0, 4.0);
    assert_eq!(cartesian, expected_result);
}

#[test]
fn from_elements_result_param_returns_cartesian3_with_correct_coordinates() {
    let mut cartesian3 = Cartesian3::default();
    Cartesian3::from_elements(2.0, 2.0, 4.0, &mut cartesian3);
    let expected_result = Cartesian3::new(2.0, 2.0, 4.0);
    assert_eq!(cartesian3, expected_result);
}

#[test]
fn from_degrees() {
    let lon = -115.0;
    let lat = 37.0;
    let ellipsoid = Ellipsoid::WGS84;
    let actual = Cartesian3::from_degrees_new(lon, lat, None, None);
    let cartographic = Cartographic::from_degrees_new(lon, lat, None);
    let mut expected = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut expected);
    assert_eq!(actual, expected);
}

#[test]
fn from_degrees_with_height() {
    let lon = -115.0;
    let lat = 37.0;
    let height = 100000.0;
    let ellipsoid = Ellipsoid::WGS84;
    let actual = Cartesian3::from_degrees_new(lon, lat, Some(height), None);
    let cartographic = Cartographic::from_degrees_new(lon, lat, Some(height));
    let mut expected = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut expected);
    assert_eq!(actual, expected);
}

#[test]
fn from_degrees_with_result() {
    let lon = -115.0;
    let lat = 37.0;
    let height = 100000.0;
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    Cartesian3::from_degrees(lon, lat, Some(height), None, &mut result);
    let cartographic = Cartographic::from_degrees_new(lon, lat, Some(height));
    let mut expected = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut expected);
    // JS `expect(actual).toBe(result)` — result-param identity is inherent in
    // the `&mut` out-param mapping.
    assert_eq!(result, expected);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_degrees_throws_with_no_longitude() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_degrees_throws_with_no_latitude() {}

#[test]
fn from_degrees_works_with_default_ellipsoid() {
    // JS: `Ellipsoid.default = Ellipsoid.MOON;`
    with_moon_default(|| {
        let expected_position = Cartesian3::new(
            1593514.338295244,
            691991.9979835141,
            20442.318221152018,
        );
        let mut position = Cartesian3::default();
        Cartesian3::from_degrees(23.47315, 0.67416, None, None, &mut position);
        assert_c3_eq_rel_epsilon(&expected_position, &position, CesiumMath::EPSILON8);
    });
}

#[test]
fn from_radians() {
    let lon = CesiumMath::to_radians(150.0);
    let lat = CesiumMath::to_radians(-40.0);
    let ellipsoid = Ellipsoid::WGS84;
    let actual = Cartesian3::from_radians_new(lon, lat, None, None);
    let cartographic = Cartographic::from_radians_new(lon, lat, None);
    let mut expected = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut expected);
    assert_eq!(actual, expected);
}

#[test]
fn from_radians_with_height() {
    let lon = CesiumMath::to_radians(150.0);
    let lat = CesiumMath::to_radians(-40.0);
    let height = 100000.0;
    let ellipsoid = Ellipsoid::WGS84;
    let actual = Cartesian3::from_radians_new(lon, lat, Some(height), None);
    let cartographic = Cartographic::from_radians_new(lon, lat, Some(height));
    let mut expected = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut expected);
    assert_eq!(actual, expected);
}

#[test]
fn from_radians_with_result() {
    let lon = CesiumMath::to_radians(150.0);
    let lat = CesiumMath::to_radians(-40.0);
    let height = 100000.0;
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    Cartesian3::from_radians(lon, lat, Some(height), None, &mut result);
    let cartographic = Cartographic::from_radians_new(lon, lat, Some(height));
    let mut expected = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut expected);
    // JS `expect(actual).toBe(result)` — result-param identity is inherent in
    // the `&mut` out-param mapping.
    assert_eq!(result, expected);
}

#[test]
fn from_radians_works_with_default_ellipsoid() {
    // JS: `Ellipsoid.default = Ellipsoid.MOON;`
    with_moon_default(|| {
        let expected_position = Cartesian3::new(
            1593514.3406204558,
            691991.9927155221,
            20442.315293410087,
        );
        let mut position = Cartesian3::default();
        Cartesian3::from_radians(0.40968375, 0.01176631, None, None, &mut position);
        assert_c3_eq_rel_epsilon(&expected_position, &position, CesiumMath::EPSILON8);
    });
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_radians_throws_with_no_longitude() {}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn from_radians_throws_with_no_latitude() {}

#[test]
#[ignore = "deferred: expected computed via Ellipsoid.cartographicArrayToCartesianArray (Ellipsoid port pending)"]
fn from_degrees_array() {}

#[test]
fn from_degrees_array_works_with_default_ellipsoid() {
    // JS: `Ellipsoid.default = Ellipsoid.MOON;`
    with_moon_default(|| {
        let expected_positions = [
            Cartesian3::new(1593514.338295244, 691991.9979835141, 20442.318221152018),
            Cartesian3::new(1653831.6133167143, -520773.6558050613, -110428.9555038242),
            Cartesian3::new(1556660.3478111108, 98714.16930719782, 765259.9782626687),
        ];
        let positions = Cartesian3::from_degrees_array(
            &[23.47315, 0.67416, 342.52135, -3.64417, 3.6285, 26.13341],
            None,
            None,
        );
        assert_eq!(positions.len(), expected_positions.len());
        for (expected, actual) in expected_positions.iter().zip(positions.iter()) {
            assert_c3_eq_rel_epsilon(expected, actual, CesiumMath::EPSILON8);
        }
    });
}

#[test]
fn from_degrees_array_throws_with_positions_length_less_than_2() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_degrees_array(&[], None, None);
    });
}

#[test]
fn from_degrees_array_throws_with_positions_length_not_multiple_of_2() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_degrees_array(&[1.0, 3.0, 5.0], None, None);
    });
}

#[test]
#[ignore = "deferred: expected computed via Ellipsoid.cartographicArrayToCartesianArray (Ellipsoid port pending)"]
fn from_radians_array() {}

#[test]
#[ignore = "deferred: expected computed via Ellipsoid.cartographicArrayToCartesianArray (Ellipsoid port pending)"]
fn from_radians_array_with_result() {}

#[test]
fn from_radians_array_works_with_default_ellipsoid() {
    // JS: `Ellipsoid.default = Ellipsoid.MOON;`
    with_moon_default(|| {
        let expected_positions = [
            Cartesian3::new(1593514.3406204558, 691991.9927155221, 20442.315293410087),
            Cartesian3::new(1653831.6107836158, -520773.6656886929, -110428.94683022468),
            Cartesian3::new(1556660.3474447567, 98714.16630095398, 765259.9793956806),
        ];
        let positions = Cartesian3::from_radians_array(
            &[0.40968375, 0.01176631, 5.97812531, -0.06360276, 0.06332927, 0.45611405],
            None,
            None,
        );
        assert_eq!(positions.len(), expected_positions.len());
        for (expected, actual) in expected_positions.iter().zip(positions.iter()) {
            assert_c3_eq_rel_epsilon(expected, actual, CesiumMath::EPSILON8);
        }
    });
}

#[test]
fn from_radians_array_throws_with_positions_length_less_than_2() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_radians_array(&[], None, None);
    });
}

#[test]
fn from_radians_array_throws_with_positions_length_not_multiple_of_2() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_radians_array(&[1.0, 3.0, 5.0], None, None);
    });
}

#[test]
#[ignore = "deferred: expected computed via Ellipsoid.cartographicArrayToCartesianArray (Ellipsoid port pending)"]
fn from_degrees_array_heights() {}

#[test]
fn from_degrees_array_heights_works_with_default_ellipsoid() {
    // JS: `Ellipsoid.default = Ellipsoid.MOON;`
    with_moon_default(|| {
        let expected_positions = [
            Cartesian3::new(1593606.0566294384, 692031.8271534222, 20443.494825170732),
            Cartesian3::new(1653926.8033485617, -520803.63011470815, -110435.31149297487),
            Cartesian3::new(1556749.9449302435, 98719.85102524245, 765304.0245374623),
        ];
        let positions = Cartesian3::from_degrees_array_heights(
            &[
                23.47315, 0.67416, 100.0, 342.52135, -3.64417, 100.0, 3.6285, 26.13341, 100.0,
            ],
            None,
            None,
        );
        assert_eq!(positions.len(), expected_positions.len());
        for (expected, actual) in expected_positions.iter().zip(positions.iter()) {
            assert_c3_eq_rel_epsilon(expected, actual, CesiumMath::EPSILON8);
        }
    });
}

#[test]
fn from_degrees_array_heights_throws_with_positions_length_less_than_3() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_degrees_array_heights(&[], None, None);
    });
}

#[test]
fn from_degrees_array_heights_throws_with_positions_length_not_multiple_of_3() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_degrees_array_heights(&[1.0, 3.0, 5.0, 2.0], None, None);
    });
}

#[test]
#[ignore = "deferred: expected computed via Ellipsoid.cartographicArrayToCartesianArray (Ellipsoid port pending)"]
fn from_radians_array_heights() {}

#[test]
#[ignore = "deferred: expected computed via Ellipsoid.cartographicArrayToCartesianArray (Ellipsoid port pending)"]
fn from_radians_array_heights_with_result() {}

#[test]
fn from_radians_array_heights_works_with_default_ellipsoid() {
    // JS: `Ellipsoid.default = Ellipsoid.MOON;`
    with_moon_default(|| {
        let expected_positions = [
            Cartesian3::new(1593606.0589547842, 692031.821885127, 20443.49189726029),
            Cartesian3::new(1653926.8008153175, -520803.6399989086, -110435.30281887612),
            Cartesian3::new(1556749.9445638682, 98719.84801882556, 765304.0256705394),
        ];
        let positions = Cartesian3::from_radians_array_heights(
            &[
                0.40968375,
                0.01176631,
                100.0,
                5.97812531,
                -0.06360276,
                100.0,
                0.06332927,
                0.45611405,
                100.0,
            ],
            None,
            None,
        );
        assert_eq!(positions.len(), expected_positions.len());
        for (expected, actual) in expected_positions.iter().zip(positions.iter()) {
            assert_c3_eq_rel_epsilon(expected, actual, CesiumMath::EPSILON8);
        }
    });
}

#[test]
fn from_radians_array_heights_throws_with_positions_length_less_than_3() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_radians_array_heights(&[], None, None);
    });
}

#[test]
fn from_radians_array_heights_throws_with_positions_length_not_multiple_of_3() {
    expect_to_throw_dev_error(|| {
        Cartesian3::from_radians_array_heights(&[1.0, 3.0, 5.0, 2.0], None, None);
    });
}

#[test]
#[ignore = "JS missing-result DeveloperError; result is mandatory in Rust"]
fn missing_result_throws_group() {}

// JS its (each statically impossible in Rust): minimumByComponent/
// maximumByComponent/clamp/normalize/multiplyComponents/divideComponents/add/
// subtract/multiplyByScalar/divideByScalar/negate/abs/cross/lerp/
// mostOrthogonalAxis with no result.

#[test]
fn projects_vector_a_onto_vector_b() {
    let mut a = Cartesian3::new(0.0, 1.0, 0.0);
    let mut b = Cartesian3::new(1.0, 0.0, 0.0);
    let mut result = Cartesian3::default();
    Cartesian3::project_vector(&a, &b, &mut result);
    assert_eq!(result, Cartesian3::new(0.0, 0.0, 0.0));

    a = Cartesian3::new(1.0, 1.0, 0.0);
    b = Cartesian3::new(1.0, 0.0, 0.0);
    Cartesian3::project_vector(&a, &b, &mut result);
    assert_eq!(result, Cartesian3::new(1.0, 0.0, 0.0));
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn project_vector_throws_when_missing_parameters() {}

//////////////////////////////////////////////////////////////////////
// createPackableSpecs(Cartesian3, new Cartesian3(1, 2, 3), [1, 2, 3])
//////////////////////////////////////////////////////////////////////

fn packable_instance() -> Cartesian3 {
    Cartesian3::new(1.0, 2.0, 3.0)
}

const PACKED_INSTANCE: [f64; 3] = [1.0, 2.0, 3.0];

#[test]
fn packable_can_pack() {
    let mut packed_array = vec![0.0; Cartesian3::PACKED_LENGTH];
    Cartesian3::pack(&packable_instance(), &mut packed_array, None);
    assert_eq!(packed_array.len(), Cartesian3::PACKED_LENGTH);
    assert_c3_eq_epsilon(
        &Cartesian3::new(PACKED_INSTANCE[0], PACKED_INSTANCE[1], PACKED_INSTANCE[2]),
        &Cartesian3::new(packed_array[0], packed_array[1], packed_array[2]),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn packable_can_roundtrip() {
    let mut packed_array = vec![0.0; Cartesian3::PACKED_LENGTH];
    Cartesian3::pack(&packable_instance(), &mut packed_array, None);
    let result = Cartesian3::unpack_new(&packed_array, None);
    assert_eq!(packable_instance(), result);
}

#[test]
fn packable_can_unpack() {
    let result = Cartesian3::unpack_new(&PACKED_INSTANCE, None);
    assert_eq!(result, packable_instance());
}

#[test]
fn packable_can_pack_with_starting_index() {
    let mut packed_array = vec![0.0; 1 + Cartesian3::PACKED_LENGTH];
    let expected: Vec<f64> = [0.0_f64].iter().chain(PACKED_INSTANCE.iter()).copied().collect();
    Cartesian3::pack(&packable_instance(), &mut packed_array, Some(1));
    for i in 0..expected.len() {
        assert_approx_eq_f64!(packed_array[i], expected[i], CesiumMath::EPSILON15);
    }
}

#[test]
fn packable_can_unpack_with_starting_index() {
    let packed_array: Vec<f64> = [0.0_f64].iter().chain(PACKED_INSTANCE.iter()).copied().collect();
    let result = Cartesian3::unpack_new(&packed_array, Some(1));
    assert_eq!(packable_instance(), result);
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_undefined_throws_group() {}

// JS its: pack throws with undefined value / undefined array; unpack throws
// with undefined array.

//////////////////////////////////////////////////////////////////////
// createPackableArraySpecs(Cartesian3, [(1,2,3),(4,5,6)], [1,2,3,4,5,6], 3)
//////////////////////////////////////////////////////////////////////

fn packable_unpacked_array() -> Vec<Cartesian3> {
    vec![Cartesian3::new(1.0, 2.0, 3.0), Cartesian3::new(4.0, 5.0, 6.0)]
}

const PACKED_ARRAY: [f64; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

const PACKABLE_STRIDE: usize = 3;

#[test]
fn packable_array_can_pack() {
    let actual_packed_array = Cartesian3::pack_array(&packable_unpacked_array(), None);
    assert_eq!(actual_packed_array.len(), PACKED_ARRAY.len());
    assert_eq!(actual_packed_array, PACKED_ARRAY);
}

#[test]
fn packable_array_can_roundtrip() {
    let actual_packed_array = Cartesian3::pack_array(&packable_unpacked_array(), None);
    let result = Cartesian3::unpack_array(&actual_packed_array, None);
    assert_eq!(result, packable_unpacked_array());
}

#[test]
fn packable_array_can_unpack() {
    let result = Cartesian3::unpack_array(&PACKED_ARRAY, None);
    assert_eq!(result, packable_unpacked_array());
}

#[test]
#[ignore = "DEVIATION: Rust has a single Vec<f64> representation; JS typed-array branch not ported"]
fn packable_array_pack_array_works_with_typed_arrays() {}

#[test]
fn packable_array_pack_array_resizes_arrays_as_needed() {
    let empty_array: Vec<f64> = Vec::new();
    let result = Cartesian3::pack_array(&packable_unpacked_array(), Some(empty_array));
    assert_eq!(result, PACKED_ARRAY);

    let larger_array = vec![0.0; PACKED_ARRAY.len() + 1];
    let result = Cartesian3::pack_array(&packable_unpacked_array(), Some(larger_array));
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
    let array = Cartesian3::unpack_array(&PACKED_ARRAY, None);
    assert_eq!(array, packable_unpacked_array());
}

#[test]
#[ignore = "JS undefined-argument DeveloperError; statically impossible in Rust"]
fn packable_array_unpack_array_throws_with_undefined_array() {}

#[test]
fn packable_array_unpack_array_works_with_a_result_parameter() {
    let array: Vec<Cartesian3> = Vec::new();
    let result = Cartesian3::unpack_array(&PACKED_ARRAY, Some(array));
    assert_eq!(result, packable_unpacked_array());

    let array: Vec<Cartesian3> = vec![Cartesian3::default(); packable_unpacked_array().len()];
    let result = Cartesian3::unpack_array(&PACKED_ARRAY, Some(array));
    assert_eq!(result, packable_unpacked_array());
}

#[test]
fn packable_array_unpack_array_throws_with_array_less_than_the_minimum_length() {
    expect_to_throw_dev_error(|| {
        Cartesian3::unpack_array(&[1.0], None);
    });
}

#[test]
fn unpack_array_throws_with_array_not_multiple_of_stride() {
    expect_to_throw_dev_error(|| {
        Cartesian3::unpack_array(&vec![1.0; PACKABLE_STRIDE + 1], None);
    });
}
