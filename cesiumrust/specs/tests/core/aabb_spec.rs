//! Core/AxisAlignedBoundingBoxSpec.js → Rust integration tests
//!
//! Faithful port of CesiumJS `Specs/Core/AxisAlignedBoundingBoxSpec.js` (22 `it()` cases).
//!
//! ## Platform adaptations
//! - JS result-parameter variants (`fromCorners(min, max, result)`, `clone(result)`)
//!   are merged into the owned-return tests: Rust returns owned values / uses `Copy`.
//! - JS "throws without a minimum/maximum/box/plane" cases (null/undefined checks) are
//!   omitted: Rust's type system makes passing `undefined` impossible.
//! - JS `fromPoints(undefined)` maps to Rust `from_points(&[])` (both yield the empty box).
//! - JS `clone()` with no argument returns `undefined`; Rust `Copy` has no such path → omitted.
//! - The JS 3-argument constructor `new AxisAlignedBoundingBox(min, max, center)` maps to
//!   `AxisAlignedBoundingBox::with_center`.

use cesium_geospatial::bounding::AxisAlignedBoundingBox;
use cesium_geospatial::ray::{Intersect, Plane};
use cesium_specs::{assert_vec3_epsilon, epsilon};
use glam::DVec3;

fn positions() -> Vec<DVec3> {
    vec![
        DVec3::new(3.0, -1.0, -3.0),
        DVec3::new(2.0, -2.0, -2.0),
        DVec3::new(1.0, -3.0, -1.0),
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(-1.0, 1.0, 1.0),
        DVec3::new(-2.0, 2.0, 2.0),
        DVec3::new(-3.0, 3.0, 3.0),
    ]
}

/// `it("constructor sets expected default values")`
#[test]
fn test_aabb_default() {
    let box_ = AxisAlignedBoundingBox::default();
    assert_vec3_epsilon!(box_.minimum, DVec3::ZERO, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.maximum, DVec3::ZERO, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.center, DVec3::ZERO, epsilon::EPSILON15);
}

/// `it("constructor sets expected parameter values")`
#[test]
fn test_aabb_constructor_parameter_values() {
    let minimum = DVec3::new(1.0, 2.0, 3.0);
    let maximum = DVec3::new(4.0, 5.0, 6.0);
    let center = DVec3::new(2.5, 3.5, 4.5);
    let box_ = AxisAlignedBoundingBox::with_center(minimum, maximum, center);
    assert_vec3_epsilon!(box_.minimum, minimum, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.maximum, maximum, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.center, center, epsilon::EPSILON15);
}

/// `it("constructor computes center if not supplied")`
#[test]
fn test_aabb_constructor_computes_center() {
    let minimum = DVec3::new(1.0, 2.0, 3.0);
    let maximum = DVec3::new(4.0, 5.0, 6.0);
    let expected_center = DVec3::new(2.5, 3.5, 4.5);
    let box_ = AxisAlignedBoundingBox::new(minimum, maximum);
    assert_vec3_epsilon!(box_.minimum, minimum, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.maximum, maximum, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.center, expected_center, epsilon::EPSILON15);
}

/// `it("fromCorners works without a result parameter")`
#[test]
fn test_aabb_from_corners() {
    let minimum = DVec3::new(0.0, 0.0, 0.0);
    let maximum = DVec3::new(1.0, 1.0, 1.0);
    let expected_center = DVec3::new(0.5, 0.5, 0.5);
    let box_ = AxisAlignedBoundingBox::from_corners(minimum, maximum);
    assert_vec3_epsilon!(box_.minimum, minimum, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.maximum, maximum, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.center, expected_center, epsilon::EPSILON15);
}

/// `it("fromPoints constructs empty box with undefined positions")` +
/// `it("fromPoints constructs empty box with empty positions")`
/// (JS `undefined` and `[]` both map to Rust empty slice)
#[test]
fn test_aabb_from_points_empty() {
    let box_ = AxisAlignedBoundingBox::from_points(&[]);
    assert_vec3_epsilon!(box_.minimum, DVec3::ZERO, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.maximum, DVec3::ZERO, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.center, DVec3::ZERO, epsilon::EPSILON15);
}

/// `it("fromPoints computes the correct values")`
#[test]
fn test_aabb_from_points_values() {
    let box_ = AxisAlignedBoundingBox::from_points(&positions());
    assert_vec3_epsilon!(box_.minimum, DVec3::new(-3.0, -3.0, -3.0), epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.maximum, DVec3::new(3.0, 3.0, 3.0), epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.center, DVec3::new(0.0, 0.0, 0.0), epsilon::EPSILON15);
}

/// `it("clone without a result parameter")`
#[test]
fn test_aabb_clone() {
    let box_ = AxisAlignedBoundingBox::new(DVec3::Y, DVec3::X);
    let result = box_; // Copy semantics == box.clone()
    assert!(box_ == result);
}

/// `it("clone without a result parameter with box of offset center")`
#[test]
fn test_aabb_clone_offset_center() {
    let box_ = AxisAlignedBoundingBox::with_center(DVec3::Y, DVec3::X, DVec3::Z);
    let result = box_;
    assert!(box_ == result);
    assert_vec3_epsilon!(result.center, DVec3::Z, epsilon::EPSILON15);
}

/// `it("equals works in all cases")`
#[test]
fn test_aabb_equals() {
    let box_ = AxisAlignedBoundingBox::with_center(DVec3::X, DVec3::Y, DVec3::Z);
    let bogie = DVec3::new(2.0, 3.0, 4.0);

    assert!(box_ == AxisAlignedBoundingBox::with_center(DVec3::X, DVec3::Y, DVec3::Z));
    assert!(box_ != AxisAlignedBoundingBox::with_center(bogie, DVec3::Y, DVec3::Y));
    assert!(box_ != AxisAlignedBoundingBox::with_center(DVec3::X, bogie, DVec3::Z));
    assert!(box_ != AxisAlignedBoundingBox::with_center(DVec3::X, DVec3::Y, bogie));
}

/// `it("computes the bounding box for a single position")`
#[test]
fn test_aabb_single_position() {
    let p = positions()[0];
    let box_ = AxisAlignedBoundingBox::from_points(&[p]);
    assert_vec3_epsilon!(box_.minimum, p, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.maximum, p, epsilon::EPSILON15);
    assert_vec3_epsilon!(box_.center, p, epsilon::EPSILON15);
}

/// `it("intersectPlane works with box on the positive side of a plane")`
#[test]
fn test_aabb_intersect_plane_positive() {
    let box_ = AxisAlignedBoundingBox::new(-DVec3::X, DVec3::ZERO);
    let normal = -DVec3::X;
    let position = DVec3::X;
    let plane = Plane::new(normal, -normal.dot(position));
    assert!(box_.intersect_plane(plane.normal, plane.distance) == Intersect::Inside);
}

/// `it("intersectPlane works with box on the negative side of a plane")`
#[test]
fn test_aabb_intersect_plane_negative() {
    let box_ = AxisAlignedBoundingBox::new(-DVec3::X, DVec3::ZERO);
    let normal = DVec3::X;
    let position = DVec3::X;
    let plane = Plane::new(normal, -normal.dot(position));
    assert!(box_.intersect_plane(plane.normal, plane.distance) == Intersect::Outside);
}

/// `it("intersectPlane works with box intersecting a plane")`
#[test]
fn test_aabb_intersect_plane_intersecting() {
    let box_ = AxisAlignedBoundingBox::new(DVec3::ZERO, DVec3::X * 2.0);
    let normal = DVec3::X;
    let position = DVec3::X;
    let plane = Plane::new(normal, -normal.dot(position));
    assert!(box_.intersect_plane(plane.normal, plane.distance) == Intersect::Intersecting);
}
