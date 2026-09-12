//! Spec mirror of `packages/engine/Specs/Core/AxisAlignedBoundingBoxSpec.js`.
//!
//! Cases the JS covers but Rust cannot express are omitted by construction:
//! the `Check.defined` throws (`fromCorners` without a minimum/maximum,
//! `intersectPlane` without a box/plane) are type errors here, and
//! `clone works with "this" result parameter` aliases one object as both
//! source and destination, which the borrow checker rejects (it is a
//! self-assignment no-op in JS anyway). `fromCorners`/`fromPoints` also take no
//! `result` out-parameter in the port, so only the "without a result parameter"
//! variants apply.

use cesium_core::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::intersect::Intersect;
use cesium_core::plane::Plane;

/// The JS spec's module-level `positions` fixture.
fn positions() -> Vec<Cartesian3> {
    vec![
        Cartesian3::new(3.0, -1.0, -3.0),
        Cartesian3::new(2.0, -2.0, -2.0),
        Cartesian3::new(1.0, -3.0, -1.0),
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(-1.0, 1.0, 1.0),
        Cartesian3::new(-2.0, 2.0, 2.0),
        Cartesian3::new(-3.0, 3.0, 3.0),
    ]
}

fn positions_minimum() -> Cartesian3 {
    Cartesian3::new(-3.0, -3.0, -3.0)
}
fn positions_maximum() -> Cartesian3 {
    Cartesian3::new(3.0, 3.0, 3.0)
}
fn positions_center() -> Cartesian3 {
    Cartesian3::ZERO
}

#[test]
fn default_constructor() {
    let box_a = AxisAlignedBoundingBox::default();
    assert_eq!(box_a.minimum, Cartesian3::ZERO);
    assert_eq!(box_a.maximum, Cartesian3::ZERO);
    assert_eq!(box_a.center, Cartesian3::ZERO);
}

#[test]
fn constructor_with_parameters() {
    let min = Cartesian3::new(1.0, 2.0, 3.0);
    let max = Cartesian3::new(4.0, 5.0, 6.0);
    let center = Cartesian3::new(2.5, 3.5, 4.5);
    let box_a = AxisAlignedBoundingBox::new(min, max, Some(center));
    assert_eq!(box_a.minimum, min);
    assert_eq!(box_a.maximum, max);
    assert_eq!(box_a.center, center);
}

#[test]
fn constructor_computes_center_if_not_supplied() {
    let min = Cartesian3::new(1.0, 2.0, 3.0);
    let max = Cartesian3::new(4.0, 5.0, 6.0);
    let expected_center = Cartesian3::new(2.5, 3.5, 4.5);
    let box_a = AxisAlignedBoundingBox::new(min, max, None);
    assert_eq!(box_a.minimum, min);
    assert_eq!(box_a.maximum, max);
    assert_eq!(box_a.center, expected_center);
}

#[test]
fn from_corners() {
    let min = Cartesian3::new(0.0, 0.0, 0.0);
    let max = Cartesian3::new(1.0, 1.0, 1.0);
    let expected_center = Cartesian3::new(0.5, 0.5, 0.5);
    let box_a = AxisAlignedBoundingBox::from_corners(&min, &max);
    assert_eq!(box_a.minimum, min);
    assert_eq!(box_a.maximum, max);
    assert_eq!(box_a.center, expected_center);
}

#[test]
fn from_points_constructs_empty_box_with_undefined_positions() {
    let box_a = AxisAlignedBoundingBox::from_points(None);
    assert_eq!(box_a.minimum, Cartesian3::ZERO);
    assert_eq!(box_a.maximum, Cartesian3::ZERO);
    assert_eq!(box_a.center, Cartesian3::ZERO);
}

#[test]
fn from_points_constructs_empty_box_with_empty_positions() {
    let box_a = AxisAlignedBoundingBox::from_points(Some(&[]));
    assert_eq!(box_a.minimum, Cartesian3::ZERO);
    assert_eq!(box_a.maximum, Cartesian3::ZERO);
    assert_eq!(box_a.center, Cartesian3::ZERO);
}

#[test]
fn from_points_computes_the_correct_values() {
    let positions = positions();
    let box_a = AxisAlignedBoundingBox::from_points(Some(&positions));
    assert_eq!(box_a.minimum, positions_minimum());
    assert_eq!(box_a.maximum, positions_maximum());
    assert_eq!(box_a.center, positions_center());
}

#[test]
fn computes_the_bounding_box_for_a_single_position() {
    let positions = positions();
    let box_a = AxisAlignedBoundingBox::from_points(Some(&positions[..1]));
    assert_eq!(box_a.minimum, positions[0]);
    assert_eq!(box_a.maximum, positions[0]);
    assert_eq!(box_a.center, positions[0]);
}

/// The accumulator is seeded from `positions[0]`, not from `±f64::MAX`. JS
/// `Math.min(x, NaN)` is `NaN`, so a single NaN position poisons the whole box;
/// Rust's `f64::min` would instead return the other operand and swallow it.
#[test]
fn from_points_propagates_nan_like_math_min() {
    let positions = vec![
        Cartesian3::new(1.0, 1.0, 1.0),
        Cartesian3::new(f64::NAN, 0.0, 0.0),
        Cartesian3::new(-5.0, -5.0, -5.0),
    ];
    let box_a = AxisAlignedBoundingBox::from_points(Some(&positions));
    assert!(box_a.minimum.x.is_nan());
    assert!(box_a.maximum.x.is_nan());
}

#[test]
fn clone_without_a_result_parameter() {
    let box_a = AxisAlignedBoundingBox::new(Cartesian3::UNIT_Y, Cartesian3::UNIT_X, None);
    let result = AxisAlignedBoundingBox::clone_box(Some(&box_a), None).expect("box is defined");
    assert!(AxisAlignedBoundingBox::equals(Some(&box_a), Some(&result)));
}

/// `clone` passes `box.center` straight to the constructor, which *clones* it
/// rather than recomputing the midpoint — an off-midpoint center survives.
#[test]
fn clone_without_a_result_parameter_with_box_of_offset_center() {
    let box_a = AxisAlignedBoundingBox::new(
        Cartesian3::UNIT_Y,
        Cartesian3::UNIT_X,
        Some(Cartesian3::UNIT_Z),
    );
    let result = AxisAlignedBoundingBox::clone_box(Some(&box_a), None).expect("box is defined");
    assert_eq!(result.center, Cartesian3::UNIT_Z);
    assert!(AxisAlignedBoundingBox::equals(Some(&box_a), Some(&result)));
}

/// JS asserts `expect(result).toBe(returnedResult)`: the supplied volume is
/// written into *and* returned. The earlier port cloned the argument, mutated
/// the clone and left the caller's volume untouched.
#[test]
fn clone_with_a_result_parameter_writes_back() {
    let box_a = AxisAlignedBoundingBox::new(Cartesian3::UNIT_Y, Cartesian3::UNIT_X, None);
    let mut result = AxisAlignedBoundingBox::new(Cartesian3::ZERO, Cartesian3::UNIT_Z, None);
    let returned = AxisAlignedBoundingBox::clone_box(Some(&box_a), Some(&mut result));
    assert!(
        AxisAlignedBoundingBox::equals(Some(&box_a), Some(&result)),
        "the supplied result must be overwritten"
    );
    assert!(AxisAlignedBoundingBox::equals(
        Some(&result),
        returned.as_ref()
    ));
}

#[test]
fn clone_returns_none_with_no_parameter() {
    assert_eq!(AxisAlignedBoundingBox::clone_box(None, None), None);
}

#[test]
fn equals_works_in_all_cases() {
    let box_a = AxisAlignedBoundingBox::new(
        Cartesian3::UNIT_X,
        Cartesian3::UNIT_Y,
        Some(Cartesian3::UNIT_Z),
    );
    let bogie = Cartesian3::new(2.0, 3.0, 4.0);

    let same = AxisAlignedBoundingBox::new(
        Cartesian3::UNIT_X,
        Cartesian3::UNIT_Y,
        Some(Cartesian3::UNIT_Z),
    );
    assert!(box_a.equals_instance(Some(&same)));

    let bad_minimum =
        AxisAlignedBoundingBox::new(bogie, Cartesian3::UNIT_Y, Some(Cartesian3::UNIT_Y));
    assert!(!box_a.equals_instance(Some(&bad_minimum)));

    let bad_maximum =
        AxisAlignedBoundingBox::new(Cartesian3::UNIT_X, bogie, Some(Cartesian3::UNIT_Z));
    assert!(!box_a.equals_instance(Some(&bad_maximum)));

    let bad_center =
        AxisAlignedBoundingBox::new(Cartesian3::UNIT_X, Cartesian3::UNIT_Y, Some(bogie));
    assert!(!box_a.equals_instance(Some(&bad_center)));

    // `box.equals(undefined)` → false.
    assert!(!box_a.equals_instance(None));
    assert!(!AxisAlignedBoundingBox::equals(Some(&box_a), None));
    assert!(!AxisAlignedBoundingBox::equals(None, Some(&box_a)));
    // JS `left === right` is true for two `undefined`s.
    assert!(AxisAlignedBoundingBox::equals(None, None));
}

#[test]
fn intersect_plane_works_with_box_on_the_positive_side_of_a_plane() {
    let box_a = AxisAlignedBoundingBox::new(
        Cartesian3::new(-1.0, 0.0, 0.0),
        Cartesian3::ZERO,
        None,
    );
    let normal = Cartesian3::new(-1.0, 0.0, 0.0);
    let position = Cartesian3::UNIT_X;
    let plane = Plane::new(&normal, -Cartesian3::dot(&normal, &position));
    assert_eq!(box_a.intersect_plane_instance(&plane), Intersect::Inside);
}

#[test]
fn intersect_plane_works_with_box_on_the_negative_side_of_a_plane() {
    let box_a = AxisAlignedBoundingBox::new(
        Cartesian3::new(-1.0, 0.0, 0.0),
        Cartesian3::ZERO,
        None,
    );
    let normal = Cartesian3::UNIT_X;
    let position = Cartesian3::UNIT_X;
    let plane = Plane::new(&normal, -Cartesian3::dot(&normal, &position));
    assert_eq!(box_a.intersect_plane_instance(&plane), Intersect::Outside);
}

#[test]
fn intersect_plane_works_with_box_intersecting_a_plane() {
    let box_a = AxisAlignedBoundingBox::new(
        Cartesian3::ZERO,
        Cartesian3::new(2.0, 0.0, 0.0),
        None,
    );
    let normal = Cartesian3::UNIT_X;
    let position = Cartesian3::UNIT_X;
    let plane = Plane::new(&normal, -Cartesian3::dot(&normal, &position));
    assert_eq!(
        box_a.intersect_plane_instance(&plane),
        Intersect::Intersecting
    );
}

/// Regression guard for the removed `half_diagonal()`: CesiumJS derives `h` as
/// `(maximum - minimum) * 0.5` and never consults `center`, while the deleted
/// helper returned `maximum - center`. With an off-midpoint `center` the two
/// formulas disagree — the old one reported `Inside` where CesiumJS reports
/// `Intersecting`.
#[test]
fn intersect_plane_ignores_an_offset_center_when_deriving_the_half_diagonal() {
    let box_a = AxisAlignedBoundingBox::new(
        Cartesian3::new(-1.0, -1.0, -1.0),
        Cartesian3::new(1.0, 1.0, 1.0),
        Some(Cartesian3::new(10.0, 0.0, 0.0)),
    );
    // s = dot(center, UNIT_X) + distance = 10 - 9.5 = 0.5; e = h.x = 1.0.
    // s - e = -0.5 (not > 0) and s + e = 1.5 (not < 0) → INTERSECTING.
    // The old `h = maximum - center = (-9, 1, 1)` would give e = -9 and
    // s - e = 9.5 > 0 → INSIDE.
    let plane = Plane::new(&Cartesian3::UNIT_X, -9.5);
    assert_eq!(
        box_a.intersect_plane_instance(&plane),
        Intersect::Intersecting
    );
}

#[test]
fn intersect_axis_aligned_bounding_box_works() {
    let box_a = AxisAlignedBoundingBox::from_corners(
        &Cartesian3::new(-1.0, -1.0, -1.0),
        &Cartesian3::new(1.0, 1.0, 1.0),
    );
    let overlapping = AxisAlignedBoundingBox::from_corners(
        &Cartesian3::new(0.0, 0.0, 0.0),
        &Cartesian3::new(2.0, 2.0, 2.0),
    );
    // Touching on a face still counts as intersecting (`<=` / `>=`).
    let touching = AxisAlignedBoundingBox::from_corners(
        &Cartesian3::new(1.0, -1.0, -1.0),
        &Cartesian3::new(2.0, 1.0, 1.0),
    );
    let disjoint = AxisAlignedBoundingBox::from_corners(
        &Cartesian3::new(2.0, 2.0, 2.0),
        &Cartesian3::new(3.0, 3.0, 3.0),
    );

    assert!(box_a.intersect_axis_aligned_bounding_box_instance(&overlapping));
    assert!(box_a.intersect_axis_aligned_bounding_box_instance(&touching));
    assert!(!box_a.intersect_axis_aligned_bounding_box_instance(&disjoint));
    // Symmetric, as the JS short-circuit chain implies.
    assert!(!disjoint.intersect_axis_aligned_bounding_box_instance(&box_a));
}
