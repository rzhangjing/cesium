//! Mirror of `packages/engine/Specs/Core/EllipsoidSpec.js` (one-to-one).
//!
//! Conventions:
//! - Jasmine `it(...)` titles map to `#[test] fn` names (snake_case).
//! - `toEqualEpsilon` -> `assert_cartesian3_epsilon` / `assert_cartographic_epsilon`.
//! - `toThrowDeveloperError` -> `#[should_panic]` (debug builds).
//! - JS cases passing `undefined` or relying on result-parameter identity are
//!   statically impossible in Rust; they are folded or kept as commented stubs.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::rectangle::Rectangle;

fn assert_cartesian3_epsilon(left: &Cartesian3, right: &Cartesian3, epsilon: f64) {
    assert!(
        (left.x - right.x).abs() <= epsilon
            && (left.y - right.y).abs() <= epsilon
            && (left.z - right.z).abs() <= epsilon,
        "expected {:?} to equal {:?} within {}",
        left,
        right,
        epsilon
    );
}

fn assert_cartographic_epsilon(left: &Cartographic, right: &Cartographic, epsilon: f64) {
    assert!(
        (left.longitude - right.longitude).abs() <= epsilon
            && (left.latitude - right.latitude).abs() <= epsilon
            && (left.height - right.height).abs() <= epsilon,
        "expected ({}, {}, {}) to equal ({}, {}, {}) within {}",
        left.longitude,
        left.latitude,
        left.height,
        right.longitude,
        right.latitude,
        right.height,
        epsilon
    );
}

const RADII: Cartesian3 = Cartesian3::new(1.0, 2.0, 3.0);

//All values computed using STK Components
const SPACE_CARTESIAN: Cartesian3 = Cartesian3::new(
    4582719.8827300891,
    -4582719.8827300882,
    1725510.4250797231,
);
const SPACE_CARTESIAN_GEODETIC_SURFACE_NORMAL: Cartesian3 = Cartesian3::new(
    0.6829975339864266,
    -0.68299753398642649,
    0.25889908678270795,
);

fn space_cartographic() -> Cartographic {
    Cartographic::new(
        CesiumMath::to_radians(-45.0),
        CesiumMath::to_radians(15.0),
        330000.0,
    )
}

fn surface_cartographic() -> Cartographic {
    Cartographic::new(
        CesiumMath::to_radians(25.0),
        CesiumMath::to_radians(45.0),
        0.0,
    )
}

const SURFACE_CARTESIAN: Cartesian3 = Cartesian3::new(
    4094327.7921465295,
    1909216.4044747739,
    4487348.4088659193,
);

#[test]
fn default_constructor_creates_zero_ellipsoid() {
    let ellipsoid = Ellipsoid::new(0.0, 0.0, 0.0);
    assert_eq!(*ellipsoid.radii(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.radii_squared(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.radii_to_the_fourth(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.one_over_radii(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.one_over_radii_squared(), Cartesian3::ZERO);
    assert_eq!(ellipsoid.minimum_radius(), 0.0);
    assert_eq!(ellipsoid.maximum_radius(), 0.0);
}

#[test]
fn from_cartesian3_creates_zero_ellipsoid_with_no_parameters() {
    let ellipsoid = Ellipsoid::from_cartesian3(None);
    assert_eq!(*ellipsoid.radii(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.radii_squared(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.radii_to_the_fourth(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.one_over_radii(), Cartesian3::ZERO);
    assert_eq!(*ellipsoid.one_over_radii_squared(), Cartesian3::ZERO);
    assert_eq!(ellipsoid.minimum_radius(), 0.0);
    assert_eq!(ellipsoid.maximum_radius(), 0.0);
}

#[test]
fn constructor_computes_correct_values() {
    let ellipsoid = Ellipsoid::new(RADII.x, RADII.y, RADII.z);
    let radii_squared = Cartesian3::multiply_components_new(&RADII, &RADII);
    let radii_to_the_fourth =
        Cartesian3::multiply_components_new(&radii_squared, &radii_squared);
    let one_over_radii = Cartesian3::new(1.0 / RADII.x, 1.0 / RADII.y, 1.0 / RADII.z);
    let one_over_radii_squared = Cartesian3::new(
        1.0 / radii_squared.x,
        1.0 / radii_squared.y,
        1.0 / radii_squared.z,
    );
    assert_eq!(*ellipsoid.radii(), RADII);
    assert_eq!(*ellipsoid.radii_squared(), radii_squared);
    assert_eq!(*ellipsoid.radii_to_the_fourth(), radii_to_the_fourth);
    assert_eq!(*ellipsoid.one_over_radii(), one_over_radii);
    assert_eq!(*ellipsoid.one_over_radii_squared(), one_over_radii_squared);
    assert_eq!(ellipsoid.minimum_radius(), 1.0);
    assert_eq!(ellipsoid.maximum_radius(), 3.0);
}

#[test]
fn from_cartesian3_computes_correct_values() {
    let ellipsoid = Ellipsoid::from_cartesian3(Some(&RADII));
    let radii_squared = Cartesian3::multiply_components_new(&RADII, &RADII);
    let radii_to_the_fourth =
        Cartesian3::multiply_components_new(&radii_squared, &radii_squared);
    let one_over_radii = Cartesian3::new(1.0 / RADII.x, 1.0 / RADII.y, 1.0 / RADII.z);
    let one_over_radii_squared = Cartesian3::new(
        1.0 / radii_squared.x,
        1.0 / radii_squared.y,
        1.0 / radii_squared.z,
    );
    assert_eq!(*ellipsoid.radii(), RADII);
    assert_eq!(*ellipsoid.radii_squared(), radii_squared);
    assert_eq!(*ellipsoid.radii_to_the_fourth(), radii_to_the_fourth);
    assert_eq!(*ellipsoid.one_over_radii(), one_over_radii);
    assert_eq!(*ellipsoid.one_over_radii_squared(), one_over_radii_squared);
    assert_eq!(ellipsoid.minimum_radius(), 1.0);
    assert_eq!(ellipsoid.maximum_radius(), 3.0);
}

#[test]
fn geodetic_surface_normal_cartographic_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut returned_result = Cartesian3::default();
    ellipsoid.geodetic_surface_normal_cartographic(&space_cartographic(), &mut returned_result);
    assert_cartesian3_epsilon(
        &returned_result,
        &Cartesian3::new(
            0.68301270189221941,
            -0.6830127018922193,
            0.25881904510252074,
        ),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn geodetic_surface_normal_cartographic_works_with_a_result_parameter() {
    // Rust out-parameter pattern: the `result` parameter IS the destination.
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    ellipsoid.geodetic_surface_normal_cartographic(&space_cartographic(), &mut result);
    assert_cartesian3_epsilon(
        &result,
        &Cartesian3::new(
            0.68301270189221941,
            -0.6830127018922193,
            0.25881904510252074,
        ),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn geodetic_surface_normal_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut returned_result = Cartesian3::default();
    assert!(ellipsoid.geodetic_surface_normal(&SPACE_CARTESIAN, &mut returned_result));
    assert_cartesian3_epsilon(
        &returned_result,
        &SPACE_CARTESIAN_GEODETIC_SURFACE_NORMAL,
        CesiumMath::EPSILON15,
    );
}

#[test]
fn geodetic_surface_normal_returns_false_when_given_the_origin() {
    // JS returns `undefined`; the Rust port returns `false`.
    let ellipsoid = Ellipsoid::WGS84;
    let mut returned_result = Cartesian3::default();
    assert!(!ellipsoid.geodetic_surface_normal(&Cartesian3::ZERO, &mut returned_result));
}

#[test]
fn geodetic_surface_normal_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    assert!(ellipsoid.geodetic_surface_normal(&SPACE_CARTESIAN, &mut result));
    assert_cartesian3_epsilon(
        &result,
        &SPACE_CARTESIAN_GEODETIC_SURFACE_NORMAL,
        CesiumMath::EPSILON15,
    );
}

#[test]
fn cartographic_to_cartesian_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut returned_result = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&space_cartographic(), &mut returned_result);
    assert_cartesian3_epsilon(&returned_result, &SPACE_CARTESIAN, CesiumMath::EPSILON7);
}

#[test]
fn cartographic_to_cartesian_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&space_cartographic(), &mut result);
    assert_cartesian3_epsilon(&result, &SPACE_CARTESIAN, CesiumMath::EPSILON7);
}

#[test]
fn cartographic_array_to_cartesian_array_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let returned_result = ellipsoid
        .cartographic_array_to_cartesian_array(&[space_cartographic(), surface_cartographic()]);
    assert_eq!(returned_result.len(), 2);
    assert_cartesian3_epsilon(&returned_result[0], &SPACE_CARTESIAN, CesiumMath::EPSILON7);
    assert_cartesian3_epsilon(&returned_result[1], &SURFACE_CARTESIAN, CesiumMath::EPSILON7);
}

#[test]
fn cartographic_array_to_cartesian_array_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = vec![Cartesian3::default()];
    ellipsoid.cartographic_array_to_cartesian_array_into(
        &[space_cartographic(), surface_cartographic()],
        &mut result,
    );
    assert_eq!(result.len(), 2);
    assert_cartesian3_epsilon(&result[0], &SPACE_CARTESIAN, CesiumMath::EPSILON7);
    assert_cartesian3_epsilon(&result[1], &SURFACE_CARTESIAN, CesiumMath::EPSILON7);
}

#[test]
fn cartesian_to_cartographic_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut returned_result = Cartographic::default();
    assert!(ellipsoid.cartesian_to_cartographic(&SURFACE_CARTESIAN, &mut returned_result));
    assert_cartographic_epsilon(&returned_result, &surface_cartographic(), CesiumMath::EPSILON8);
}

#[test]
fn cartesian_to_cartographic_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = Cartographic::default();
    assert!(ellipsoid.cartesian_to_cartographic(&SURFACE_CARTESIAN, &mut result));
    assert_cartographic_epsilon(&result, &surface_cartographic(), CesiumMath::EPSILON8);
}

#[test]
fn cartesian_to_cartographic_works_close_to_center() {
    let mut returned_result = Cartographic::default();
    assert!(Ellipsoid::WGS84.cartesian_to_cartographic(
        &Cartesian3::new(1e-50, 1e-60, 1e-70),
        &mut returned_result,
    ));
    let expected = Cartographic::new(
        9.999999999999999e-11,
        1.0067394967422763e-20,
        -6378137.0,
    );
    assert_eq!(returned_result.longitude, expected.longitude);
    assert_eq!(returned_result.latitude, expected.latitude);
    assert_eq!(returned_result.height, expected.height);
}

#[test]
fn cartesian_to_cartographic_returns_false_very_close_to_center() {
    // JS returns `undefined`; the Rust port returns `false`.
    let ellipsoid = Ellipsoid::WGS84;
    let mut returned_result = Cartographic::default();
    assert!(!ellipsoid.cartesian_to_cartographic(
        &Cartesian3::new(1e-150, 1e-150, 1e-150),
        &mut returned_result,
    ));
}

#[test]
fn cartesian_to_cartographic_returns_false_at_center() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut returned_result = Cartographic::default();
    assert!(!ellipsoid.cartesian_to_cartographic(&Cartesian3::ZERO, &mut returned_result));
}

#[test]
fn cartesian_array_to_cartographic_array_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let returned_result = ellipsoid
        .cartesian_array_to_cartographic_array(&[SPACE_CARTESIAN, SURFACE_CARTESIAN]);
    assert_eq!(returned_result.len(), 2);
    assert_cartographic_epsilon(&returned_result[0], &space_cartographic(), CesiumMath::EPSILON7);
    assert_cartographic_epsilon(&returned_result[1], &surface_cartographic(), CesiumMath::EPSILON7);
}

#[test]
fn cartesian_array_to_cartographic_array_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut result = vec![Cartographic::default()];
    ellipsoid.cartesian_array_to_cartographic_array_into(
        &[SPACE_CARTESIAN, SURFACE_CARTESIAN],
        &mut result,
    );
    assert_eq!(result.len(), 2);
    assert_cartographic_epsilon(&result[0], &space_cartographic(), CesiumMath::EPSILON7);
    assert_cartographic_epsilon(&result[1], &surface_cartographic(), CesiumMath::EPSILON7);
}

#[test]
fn scale_to_geodetic_surface_scaled_in_the_x_direction() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(1.0, 0.0, 0.0);
    let cartesian = Cartesian3::new(9.0, 0.0, 0.0);
    let mut returned_result = Cartesian3::default();
    assert!(ellipsoid.scale_to_geodetic_surface(&cartesian, &mut returned_result));
    assert_eq!(returned_result, expected);
}

#[test]
fn scale_to_geodetic_surface_scaled_in_the_y_direction() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(0.0, 2.0, 0.0);
    let cartesian = Cartesian3::new(0.0, 8.0, 0.0);
    let mut returned_result = Cartesian3::default();
    assert!(ellipsoid.scale_to_geodetic_surface(&cartesian, &mut returned_result));
    assert_eq!(returned_result, expected);
}

#[test]
fn scale_to_geodetic_surface_scaled_in_the_z_direction() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(0.0, 0.0, 3.0);
    let cartesian = Cartesian3::new(0.0, 0.0, 8.0);
    let mut returned_result = Cartesian3::default();
    assert!(ellipsoid.scale_to_geodetic_surface(&cartesian, &mut returned_result));
    assert_eq!(returned_result, expected);
}

#[test]
fn scale_to_geodetic_surface_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(
        0.2680893773941855,
        1.1160466902266495,
        2.3559801120411263,
    );
    let cartesian = Cartesian3::new(4.0, 5.0, 6.0);
    let mut returned_result = Cartesian3::default();
    assert!(ellipsoid.scale_to_geodetic_surface(&cartesian, &mut returned_result));
    assert_cartesian3_epsilon(&returned_result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn scale_to_geodetic_surface_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(
        0.2680893773941855,
        1.1160466902266495,
        2.3559801120411263,
    );
    let cartesian = Cartesian3::new(4.0, 5.0, 6.0);
    let mut result = Cartesian3::default();
    assert!(ellipsoid.scale_to_geodetic_surface(&cartesian, &mut result));
    assert_cartesian3_epsilon(&result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn scale_to_geocentric_surface_scaled_in_the_x_direction() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(1.0, 0.0, 0.0);
    let cartesian = Cartesian3::new(9.0, 0.0, 0.0);
    let mut returned_result = Cartesian3::default();
    ellipsoid.scale_to_geocentric_surface(&cartesian, &mut returned_result);
    assert_eq!(returned_result, expected);
}

#[test]
fn scale_to_geocentric_surface_scaled_in_the_y_direction() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(0.0, 2.0, 0.0);
    let cartesian = Cartesian3::new(0.0, 8.0, 0.0);
    let mut returned_result = Cartesian3::default();
    ellipsoid.scale_to_geocentric_surface(&cartesian, &mut returned_result);
    assert_eq!(returned_result, expected);
}

#[test]
fn scale_to_geocentric_surface_scaled_in_the_z_direction() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(0.0, 0.0, 3.0);
    let cartesian = Cartesian3::new(0.0, 0.0, 8.0);
    let mut returned_result = Cartesian3::default();
    ellipsoid.scale_to_geocentric_surface(&cartesian, &mut returned_result);
    assert_eq!(returned_result, expected);
}

#[test]
fn scale_to_geocentric_surface_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(
        0.7807200583588266,
        0.9759000729485333,
        1.1710800875382399,
    );
    let cartesian = Cartesian3::new(4.0, 5.0, 6.0);
    let mut returned_result = Cartesian3::default();
    ellipsoid.scale_to_geocentric_surface(&cartesian, &mut returned_result);
    assert_cartesian3_epsilon(&returned_result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn scale_to_geocentric_surface_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let expected = Cartesian3::new(
        0.7807200583588266,
        0.9759000729485333,
        1.1710800875382399,
    );
    let cartesian = Cartesian3::new(4.0, 5.0, 6.0);
    let mut result = Cartesian3::default();
    ellipsoid.scale_to_geocentric_surface(&cartesian, &mut result);
    assert_cartesian3_epsilon(&result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn scale_to_geodetic_surface_returns_false_at_center() {
    // JS returns `undefined`; the Rust port returns `false`.
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let cartesian = Cartesian3::new(0.0, 0.0, 0.0);
    let mut returned_result = Cartesian3::default();
    assert!(!ellipsoid.scale_to_geodetic_surface(&cartesian, &mut returned_result));
}

#[test]
fn transform_position_to_scaled_space_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(2.0, 3.0, 4.0);
    let expected = Cartesian3::new(2.0, 2.0, 2.0);
    let cartesian = Cartesian3::new(4.0, 6.0, 8.0);
    let mut returned_result = Cartesian3::default();
    ellipsoid.transform_position_to_scaled_space(&cartesian, &mut returned_result);
    assert_cartesian3_epsilon(&returned_result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn transform_position_to_scaled_space_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(2.0, 3.0, 4.0);
    let expected = Cartesian3::new(3.0, 3.0, 3.0);
    let cartesian = Cartesian3::new(6.0, 9.0, 12.0);
    let mut result = Cartesian3::default();
    ellipsoid.transform_position_to_scaled_space(&cartesian, &mut result);
    assert_cartesian3_epsilon(&result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn transform_position_from_scaled_space_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(2.0, 3.0, 4.0);
    let expected = Cartesian3::new(4.0, 6.0, 8.0);
    let cartesian = Cartesian3::new(2.0, 2.0, 2.0);
    let mut returned_result = Cartesian3::default();
    ellipsoid.transform_position_from_scaled_space(&cartesian, &mut returned_result);
    assert_cartesian3_epsilon(&returned_result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn transform_position_from_scaled_space_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::new(2.0, 3.0, 4.0);
    let expected = Cartesian3::new(6.0, 9.0, 12.0);
    let cartesian = Cartesian3::new(3.0, 3.0, 3.0);
    let mut result = Cartesian3::default();
    ellipsoid.transform_position_from_scaled_space(&cartesian, &mut result);
    assert_cartesian3_epsilon(&result, &expected, CesiumMath::EPSILON16);
}

#[test]
fn equals_works_in_all_cases() {
    let ellipsoid = Ellipsoid::new(1.0, 0.0, 0.0);
    assert_eq!(ellipsoid.equals(&Ellipsoid::new(1.0, 0.0, 0.0)), true);
    assert_eq!(ellipsoid.equals(&Ellipsoid::new(1.0, 1.0, 0.0)), false);
    // `ellipsoid.equals(undefined)` -> false is statically impossible in Rust.
}

#[test]
fn to_string_produces_expected_values() {
    let expected = "(1, 2, 3)";
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    assert_eq!(ellipsoid.to_string_repr(), expected);
}

#[test]
#[should_panic]
fn constructor_throws_if_x_less_than_0() {
    let _ = Ellipsoid::new(-1.0, 0.0, 0.0);
}

#[test]
#[should_panic]
fn constructor_throws_if_y_less_than_0() {
    let _ = Ellipsoid::new(0.0, -1.0, 0.0);
}

#[test]
#[should_panic]
fn constructor_throws_if_z_less_than_0() {
    let _ = Ellipsoid::new(0.0, 0.0, -1.0);
}

#[test]
fn geocentric_surface_normal_is_cartesian3_normalize() {
    // JS asserts reference equality between the two functions; the Rust port
    // verifies behavioral equivalence instead.
    let cartesian = Cartesian3::new(1.0, 2.0, 3.0);
    let mut a = Cartesian3::default();
    let mut b = Cartesian3::default();
    Ellipsoid::geocentric_surface_normal(&cartesian, &mut a);
    Cartesian3::normalize(&cartesian, &mut b);
    assert_eq!(a, b);
}

// Statically impossible in Rust (typed parameters, no `undefined`):
// it("geodeticSurfaceNormalCartographic throws with no cartographic", ...)
// it("geodeticSurfaceNormal throws with no cartesian", ...)
// it("cartographicToCartesian throws with no cartographic", ...)
// it("cartographicArrayToCartesianArray throws with no cartographics", ...)
// it("cartesianToCartographic throws with no cartesian", ...)
// it("cartesianArrayToCartographicArray throws with no cartesians", ...)
// it("scaleToGeodeticSurface throws with no cartesian", ...)
// it("scaleToGeocentricSurface throws with no cartesian", ...)

#[test]
fn clone_copies_any_object_with_the_proper_structure() {
    // JS clones a duck-typed plain object; Rust is statically typed, so the
    // mirror clones a proper Ellipsoid and verifies full value equality.
    let my_ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let cloned = Ellipsoid::clone_ellipsoid(&my_ellipsoid);
    assert_eq!(cloned.radii(), my_ellipsoid.radii());
    assert_eq!(cloned.radii_squared(), my_ellipsoid.radii_squared());
    assert_eq!(cloned.radii_to_the_fourth(), my_ellipsoid.radii_to_the_fourth());
    assert_eq!(cloned.one_over_radii(), my_ellipsoid.one_over_radii());
    assert_eq!(
        cloned.one_over_radii_squared(),
        my_ellipsoid.one_over_radii_squared()
    );
    assert_eq!(cloned.minimum_radius(), my_ellipsoid.minimum_radius());
    assert_eq!(cloned.maximum_radius(), my_ellipsoid.maximum_radius());
}

#[test]
fn clone_uses_result_parameter_if_provided() {
    // Rust `clone_ellipsoid` returns an owned value; result parameter folded.
    let my_ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let cloned = Ellipsoid::clone_ellipsoid(&my_ellipsoid);
    assert_eq!(cloned, my_ellipsoid);
}

// Statically impossible in Rust (typed position parameter, no `undefined`):
// it("getSurfaceNormalIntersectionWithZAxis throws with no position", ...)

#[test]
#[should_panic]
fn get_surface_normal_intersection_with_z_axis_throws_if_not_ellipsoid_of_revolution() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let cartesian = Cartesian3::default();
    let mut result = Cartesian3::default();
    ellipsoid.get_surface_normal_intersection_with_z_axis(&cartesian, None, &mut result);
}

#[test]
#[should_panic]
fn get_surface_normal_intersection_with_z_axis_throws_if_radii_z_is_zero() {
    // The revolution check (1.0 != 2.0) fires first, mirroring the JS
    // check ordering in `getSurfaceNormalIntersectionWithZAxis`.
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 0.0);
    let cartesian = Cartesian3::default();
    let mut result = Cartesian3::default();
    ellipsoid.get_surface_normal_intersection_with_z_axis(&cartesian, None, &mut result);
}

#[test]
fn get_surface_normal_intersection_with_z_axis_works_without_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees_new(35.23, 33.23, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut returned_result = Cartesian3::default();
    assert!(ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        None,
        &mut returned_result,
    ));
}

#[test]
fn get_surface_normal_intersection_with_z_axis_works_with_a_result_parameter() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees_new(35.23, 33.23, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut result = cartesian_on_the_surface;
    assert!(ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        None,
        &mut result,
    ));
    // JS asserts `returnedResult` is the same object as the result parameter;
    // the Rust out-parameter fulfills the same role.
}

#[test]
fn get_surface_normal_intersection_with_z_axis_returns_false_outside_with_buffer() {
    // JS returns `undefined`; the Rust port returns `false`.
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees_new(35.23, 33.23, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut returned_result = Cartesian3::default();
    assert!(!ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        Some(ellipsoid.radii().z),
        &mut returned_result,
    ));
}

#[test]
fn get_surface_normal_intersection_with_z_axis_returns_false_outside_without_buffer() {
    let major_axis = 10.0;
    let minor_axis = 1.0;
    let ellipsoid = Ellipsoid::new(major_axis, major_axis, minor_axis);
    let cartographic = Cartographic::from_degrees_new(45.0, 90.0, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut returned_result = Cartesian3::default();
    assert!(!ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        None,
        &mut returned_result,
    ));
}

#[test]
fn get_surface_normal_intersection_with_z_axis_matches_independently_computed_value() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees_new(35.23, 33.23, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut surface_normal = Cartesian3::default();
    assert!(ellipsoid.geodetic_surface_normal(&cartesian_on_the_surface, &mut surface_normal));
    let magnitude = cartesian_on_the_surface.x / surface_normal.x;

    let mut expected = Cartesian3::default();
    expected.z = cartesian_on_the_surface.z - surface_normal.z * magnitude;
    let mut result = Cartesian3::default();
    assert!(ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        None,
        &mut result,
    ));
    assert_cartesian3_epsilon(&result, &expected, CesiumMath::EPSILON8);

    // at the equator
    let cartesian_on_the_surface = Cartesian3::new(ellipsoid.radii().x, 0.0, 0.0);
    let mut result = Cartesian3::default();
    assert!(ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        None,
        &mut result,
    ));
    assert_cartesian3_epsilon(&result, &Cartesian3::ZERO, CesiumMath::EPSILON8);
}

#[test]
fn get_surface_normal_intersection_with_z_axis_origin_produces_accurate_cartographic() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees_new(35.23, 33.23, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut surface_normal = Cartesian3::default();
    assert!(ellipsoid.geodetic_surface_normal(&cartesian_on_the_surface, &mut surface_normal));

    let mut result = Cartesian3::default();
    assert!(ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        None,
        &mut result,
    ));

    let surface_normal_with_length =
        Cartesian3::multiply_by_scalar_new(&surface_normal, ellipsoid.maximum_radius());
    let position = Cartesian3::add_new(&result, &surface_normal_with_length);
    let mut result_cartographic = Cartographic::default();
    assert!(ellipsoid.cartesian_to_cartographic(&position, &mut result_cartographic));
    result_cartographic.height = 0.0;
    assert_cartographic_epsilon(&result_cartographic, &cartographic, CesiumMath::EPSILON8);

    // at the north pole
    let cartographic = Cartographic::from_degrees_new(0.0, 90.0, None);
    let cartesian_on_the_surface = Cartesian3::new(0.0, 0.0, ellipsoid.radii().z);
    let mut surface_normal = Cartesian3::default();
    assert!(ellipsoid.geodetic_surface_normal(&cartesian_on_the_surface, &mut surface_normal));
    let surface_normal_with_length =
        Cartesian3::multiply_by_scalar_new(&surface_normal, ellipsoid.maximum_radius());
    let mut result = Cartesian3::default();
    assert!(ellipsoid.get_surface_normal_intersection_with_z_axis(
        &cartesian_on_the_surface,
        None,
        &mut result,
    ));
    let position = Cartesian3::add_new(&result, &surface_normal_with_length);
    let mut result_cartographic = Cartographic::default();
    assert!(ellipsoid.cartesian_to_cartographic(&position, &mut result_cartographic));
    result_cartographic.height = 0.0;
    assert_cartographic_epsilon(&result_cartographic, &cartographic, CesiumMath::EPSILON8);
}

// Statically impossible in Rust (typed position parameter, no `undefined`):
// it("getLocalCurvature throws with no position", ...)

#[test]
fn get_local_curvature_returns_expected_values_at_the_equator() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees_new(0.0, 0.0, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut returned_result = Cartesian2::default();
    ellipsoid.get_local_curvature(&cartesian_on_the_surface, &mut returned_result);
    let expected_result = Cartesian2::new(
        1.0 / ellipsoid.maximum_radius(),
        ellipsoid.maximum_radius()
            / (ellipsoid.minimum_radius() * ellipsoid.minimum_radius()),
    );
    assert!(
        (returned_result.x - expected_result.x).abs() <= CesiumMath::EPSILON8
            && (returned_result.y - expected_result.y).abs() <= CesiumMath::EPSILON8,
        "expected {:?} to equal {:?} within EPSILON8",
        returned_result,
        expected_result
    );
}

#[test]
fn get_local_curvature_returns_expected_values_at_the_north_pole() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::from_degrees_new(0.0, 90.0, None);
    let mut cartesian_on_the_surface = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian_on_the_surface);
    let mut returned_result = Cartesian2::default();
    ellipsoid.get_local_curvature(&cartesian_on_the_surface, &mut returned_result);
    let semi_latus_rectum = (ellipsoid.maximum_radius() * ellipsoid.maximum_radius())
        / ellipsoid.minimum_radius();
    let expected_result = Cartesian2::new(1.0 / semi_latus_rectum, 1.0 / semi_latus_rectum);
    assert!(
        (returned_result.x - expected_result.x).abs() <= CesiumMath::EPSILON8
            && (returned_result.y - expected_result.y).abs() <= CesiumMath::EPSILON8,
        "expected {:?} to equal {:?} within EPSILON8",
        returned_result,
        expected_result
    );
}

#[test]
fn ellipsoid_is_initialized_with_squared_x_over_squared_z_property() {
    // `_squaredXOverSquaredZ` is private in the Rust port; verify its effect
    // through `getSurfaceNormalIntersectionWithZAxis` (z' = z * (1 - x²/z²)).
    let ellipsoid = Ellipsoid::new(4.0, 4.0, 3.0);
    let squared_x_over_squared_z =
        ellipsoid.radii_squared().x / ellipsoid.radii_squared().z;
    let position = Cartesian3::new(2.0, 1.0, 2.5);
    let mut result = Cartesian3::default();
    assert!(ellipsoid.get_surface_normal_intersection_with_z_axis(&position, None, &mut result));
    assert!((result.z - position.z * (1.0 - squared_x_over_squared_z)).abs() < CesiumMath::EPSILON15);
}

// Statically impossible in Rust (typed `&Rectangle` parameter, no `undefined`):
// it("surfaceArea throws without rectangle", ...)

#[test]
fn computes_surface_area() {
    // area of an oblate spheroid
    let ellipsoid = Ellipsoid::new(4.0, 4.0, 3.0);
    let a2 = ellipsoid.radii_squared().x;
    let c2 = ellipsoid.radii_squared().z;
    let e = (1.0 - c2 / a2).sqrt();
    let area = CesiumMath::TWO_PI * a2
        + CesiumMath::PI * (c2 / e) * ((1.0 + e) / (1.0 - e)).ln();
    let computed = ellipsoid.surface_area(&Rectangle::new(
        -CesiumMath::PI,
        -CesiumMath::PI_OVER_TWO,
        CesiumMath::PI,
        CesiumMath::PI_OVER_TWO,
    ));
    assert!(
        (computed - area).abs() <= CesiumMath::EPSILON3 * area.abs().max(1.0),
        "oblate surface area {} != {} within EPSILON3",
        computed,
        area
    );

    // area of a prolate spheroid
    let ellipsoid = Ellipsoid::new(3.0, 3.0, 4.0);
    let a2 = ellipsoid.radii_squared().x;
    let c2 = ellipsoid.radii_squared().z;
    let e = (1.0 - a2 / c2).sqrt();
    let a = ellipsoid.radii().x;
    let c = ellipsoid.radii().z;
    let area = CesiumMath::TWO_PI * a2 + CesiumMath::TWO_PI * ((a * c) / e) * e.asin();
    let computed = ellipsoid.surface_area(&Rectangle::new(
        -CesiumMath::PI,
        -CesiumMath::PI_OVER_TWO,
        CesiumMath::PI,
        CesiumMath::PI_OVER_TWO,
    ));
    assert!(
        (computed - area).abs() <= CesiumMath::EPSILON3 * area.abs().max(1.0),
        "prolate surface area {} != {} within EPSILON3",
        computed,
        area
    );
}

// --- createPackableSpecs(Ellipsoid, Ellipsoid.WGS84, [radii.x, radii.y, radii.z]) ---

#[test]
fn packable_pack_works() {
    let mut packed = vec![0.0f64; Ellipsoid::PACKED_LENGTH];
    Ellipsoid::pack(&Ellipsoid::WGS84, &mut packed, None);
    assert_eq!(
        packed,
        [
            Ellipsoid::WGS84.radii().x,
            Ellipsoid::WGS84.radii().y,
            Ellipsoid::WGS84.radii().z,
        ]
    );
}

#[test]
fn packable_pack_works_with_starting_index() {
    let mut packed = vec![0.0f64; Ellipsoid::PACKED_LENGTH + 2];
    Ellipsoid::pack(&Ellipsoid::WGS84, &mut packed, Some(1));
    assert_eq!(
        packed,
        [
            0.0,
            Ellipsoid::WGS84.radii().x,
            Ellipsoid::WGS84.radii().y,
            Ellipsoid::WGS84.radii().z,
            0.0,
        ]
    );
}

#[test]
fn packable_packed_length_is_correct() {
    assert_eq!(Ellipsoid::PACKED_LENGTH, 3);
}

#[test]
fn packable_unpack_works() {
    let unpacked = Ellipsoid::unpack(
        &[
            Ellipsoid::WGS84.radii().x,
            Ellipsoid::WGS84.radii().y,
            Ellipsoid::WGS84.radii().z,
        ],
        None,
    );
    assert!(unpacked.equals(&Ellipsoid::WGS84));
}

#[test]
fn packable_unpack_works_with_starting_index() {
    let unpacked = Ellipsoid::unpack(
        &[
            0.0,
            Ellipsoid::WGS84.radii().x,
            Ellipsoid::WGS84.radii().y,
            Ellipsoid::WGS84.radii().z,
        ],
        Some(1),
    );
    assert!(unpacked.equals(&Ellipsoid::WGS84));
}

// Statically impossible in Rust (no mutable static `Ellipsoid.default` setter
// accepting `undefined`):
// it("set default throws if undefined", ...)
