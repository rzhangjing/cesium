//! Mirrors packages/engine/Specs/Core/SphericalSpec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;
use cesium_core::spherical::Spherical;
use cesium_test_utils::assert_approx_eq_f64;

const PI: f64 = std::f64::consts::PI;

/// JS `toEqualEpsilon` matcher: componentwise absolute-epsilon comparison.
fn assert_spherical_eq_epsilon(expected: &Spherical, actual: &Spherical, epsilon: f64) {
    assert_approx_eq_f64!(expected.clock, actual.clock, epsilon);
    assert_approx_eq_f64!(expected.cone, actual.cone, epsilon);
    assert_approx_eq_f64!(expected.magnitude, actual.magnitude, epsilon);
}

// describe("Core/Spherical")

#[test]
fn default_constructing_sets_properties_to_their_expected_values() {
    let v = Spherical::default();
    assert_eq!(v.clock, 0.0);
    assert_eq!(v.cone, 0.0);
    assert_eq!(v.magnitude, 1.0);
}

#[test]
fn constructor_parameters_are_assigned_to_the_appropriate_properties() {
    let v = Spherical::new(1.0, 2.0, 3.0);
    assert_eq!(v.clock, 1.0);
    assert_eq!(v.cone, 2.0);
    assert_eq!(v.magnitude, 3.0);
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
fn can_convert_cartesian3_to_a_new_spherical_instance() {
    assert_spherical_eq_epsilon(
        &spherical_value(),
        &Spherical::from_cartesian3_new(&spherical_cartesian()),
        CesiumMath::EPSILON15,
    );
}

#[test]
fn can_convert_cartesian3_to_an_existing_spherical_instance() {
    let mut existing = Spherical::default();
    Spherical::from_cartesian3(&spherical_cartesian(), &mut existing);
    assert_spherical_eq_epsilon(&spherical_value(), &existing, CesiumMath::EPSILON15);
}

#[test]
fn cloning_with_no_result_parameter_returns_a_new_instance() {
    let v = Spherical::new(1.0, 2.0, 3.0);
    let clone = v.clone();
    assert_eq!(clone, v);
}

#[test]
fn cloning_with_result_modifies_existing_instance_and_returns_it() {
    // JS clones into an arbitrary duck-typed `result`; in Rust the result is
    // statically a `Spherical`.
    let v = Spherical::new(1.0, 2.0, 3.0);
    let mut w = Spherical::default();
    assert_ne!(v, w);
    Spherical::clone_into(&v, &mut w);
    assert_eq!(v, w);
}

#[test]
fn normalizing_with_no_result_parameter_creates_new_instance_and_sets_magnitude_to_one() {
    let v = Spherical::new(0.0, 2.0, 3.0);
    let w = Spherical::normalize_new(&v);
    assert_ne!(w, v);
    assert_eq!(w.clock, 0.0);
    assert_eq!(w.cone, 2.0);
    assert_eq!(w.magnitude, 1.0);
}

#[test]
fn normalizing_with_result_parameter_modifies_instance_and_sets_magnitude_to_one() {
    let v = Spherical::new(0.0, 2.0, 3.0);
    let mut w = Spherical::default();
    Spherical::normalize(&v, &mut w);
    assert_ne!(w, v);
    assert_eq!(w.clock, 0.0);
    assert_eq!(w.cone, 2.0);
    assert_eq!(w.magnitude, 1.0);
}

#[test]
fn normalizing_with_this_as_result_parameter_modifies_instance_and_sets_magnitude_to_one() {
    let mut v = Spherical::new(0.0, 2.0, 3.0);
    let current = v;
    Spherical::normalize(&current, &mut v);
    assert_eq!(v.clock, 0.0);
    assert_eq!(v.cone, 2.0);
    assert_eq!(v.magnitude, 1.0);
}

#[test]
fn equals_epsilon_returns_true_for_expected_values() {
    assert!(Spherical::new(1.0, 2.0, 1.0).equals_epsilon_method(&Spherical::new(1.0, 2.0, 1.0), 0.0));
    assert!(Spherical::new(1.0, 2.0, 1.0).equals_epsilon_method(&Spherical::new(1.0, 2.0, 2.0), 1.0));
}

#[test]
fn equals_epsilon_returns_false_for_expected_values() {
    assert!(!Spherical::new(1.0, 2.0, 1.0).equals_epsilon_method(&Spherical::new(1.0, 2.0, 3.0), 1.0));
}

#[test]
fn to_string_returns_the_expected_format() {
    let v = Spherical::new(1.0, 2.0, 3.0);
    assert_eq!(v.to_string(), "(1, 2, 3)");
}
