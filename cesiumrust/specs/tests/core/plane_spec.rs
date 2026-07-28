//! Core/PlaneSpec.js → Rust integration tests
//!
//! Faithful port of CesiumJS `Specs/Core/PlaneSpec.js` (29 `it()` cases).
//!
//! ## Platform adaptations
//! - JS result-parameter variants (e.g. `fromPointNormal(point, normal, result)`,
//!   `clone(plane, result)`, `projectPointOntoPlane(..., result)`) are merged into the
//!   owned-return tests: Rust returns owned values, so the "with result" cases carry no
//!   extra behavior and are omitted.
//! - JS "throws without a <arg>" cases (null/undefined argument checks) are omitted:
//!   Rust's type system makes passing `undefined` impossible.
//! - JS "throws if normal is not normalized" cases are CesiumJS debug-only
//!   `DeveloperError` checks (stripped in release builds). In Rust these map to
//!   `debug_assert!`, so the throwing behavior is not part of the public contract and
//!   the corresponding tests are omitted.
//!
//! Ported A-class functional cases: constructs, fromPointNormal, fromCartesian4,
//! getPointDistance, projectPointOntoPlane, clone, equals, transform (uniform +
//! non-uniform scale).

use cesium_geospatial::ray::Plane;
use cesium_specs::{assert_approx, assert_vec3_epsilon, epsilon};
use glam::{DMat3, DMat4, DVec3, DVec4};

/// `it("constructs")`
#[test]
fn test_plane_constructs() {
    let normal = DVec3::X;
    let distance = 1.0;
    let plane = Plane::new(normal, distance);
    assert_vec3_epsilon!(plane.normal, normal, epsilon::EPSILON15);
    assert_approx!(plane.distance, distance, epsilon::EPSILON15);
}

/// `it("constructs from a point and a normal")`
#[test]
fn test_plane_from_point_normal() {
    let normal = DVec3::new(1.0, 2.0, 3.0).normalize();
    let point = DVec3::new(4.0, 5.0, 6.0);
    let plane = Plane::from_point_normal(point, normal);
    assert_vec3_epsilon!(plane.normal, normal, epsilon::EPSILON15);
    assert_approx!(plane.distance, -normal.dot(point), epsilon::EPSILON15);
}

/// `it("constructs from a Cartesian4 without result")`
#[test]
fn test_plane_from_cartesian4() {
    let result = Plane::from_cartesian4(DVec4::X);
    assert_vec3_epsilon!(result.normal, DVec3::X, epsilon::EPSILON15);
    assert_approx!(result.distance, 0.0, epsilon::EPSILON15);
}

/// `it("gets the distance to a point")`
#[test]
fn test_plane_get_point_distance() {
    let normal = DVec3::new(1.0, 2.0, 3.0).normalize();
    let plane = Plane::new(normal, 12.34);
    let point = DVec3::new(4.0, 5.0, 6.0);
    assert_approx!(
        plane.point_distance(point),
        plane.normal.dot(point) + plane.distance,
        epsilon::EPSILON15
    );
}

/// `it("projects a point onto the plane")`
#[test]
fn test_plane_project_point_onto_plane() {
    let plane = Plane::new(DVec3::X, 0.0);
    let point = DVec3::new(1.0, 1.0, 0.0);
    let result = plane.project_point_onto_plane(point);
    assert_vec3_epsilon!(result, DVec3::new(0.0, 1.0, 0.0), epsilon::EPSILON15);

    let plane = Plane::new(DVec3::Y, 0.0);
    let result = plane.project_point_onto_plane(point);
    assert_vec3_epsilon!(result, DVec3::new(1.0, 0.0, 0.0), epsilon::EPSILON15);
}

/// `it("clones a plane instance")`
#[test]
fn test_plane_clone() {
    let normal = DVec3::new(1.0, 2.0, 3.0).normalize();
    let distance = 4.0;
    let plane = Plane::new(normal, distance);

    let result = plane; // Copy semantics == Plane.clone(plane)
    assert_vec3_epsilon!(result.normal, normal, epsilon::EPSILON15);
    assert_approx!(result.distance, distance, epsilon::EPSILON15);
}

/// `it("equals returns true only if two planes are equal by normal and distance")`
#[test]
fn test_plane_equals() {
    let left = Plane::new(DVec3::X, 0.0);
    let mut right = Plane::new(DVec3::Y, 1.0);

    assert!(!(left == right)); // different normal & distance

    right.distance = 0.0;
    assert!(!(left == right)); // different normal

    right.normal = DVec3::X;
    assert!(left == right); // equal

    right.distance = 1.0;
    assert!(!(left == right)); // different distance
}

/// `it("transforms a plane according to a transform")`
#[test]
fn test_plane_transform() {
    let normal = DVec3::new(1.0, 2.0, 3.0).normalize();
    let plane = Plane::new(normal, 12.34);

    // Matrix4.multiplyByMatrix3(Matrix4.fromUniformScale(2.0), Matrix3.fromRotationY(PI))
    let transform =
        DMat4::from_scale(DVec3::splat(2.0)) * DMat4::from_mat3(DMat3::from_rotation_y(std::f64::consts::PI));

    let transformed_plane = plane.transform(&transform);
    assert_approx!(
        transformed_plane.distance,
        plane.distance * 2.0,
        epsilon::EPSILON15
    );
    assert_approx!(
        transformed_plane.normal.x,
        -plane.normal.x,
        epsilon::EPSILON10
    );
    assert_approx!(transformed_plane.normal.y, plane.normal.y, epsilon::EPSILON15);
    assert_approx!(
        transformed_plane.normal.z,
        -plane.normal.z,
        epsilon::EPSILON15
    );
}

/// `it("transforms a plane according to a non-uniform scale transform")`
#[test]
fn test_plane_transform_non_uniform_scale() {
    let normal = DVec3::new(1.0, 0.0, 1.0).normalize();
    let plane = Plane::new(normal, 0.0);
    let plane_origin = DVec3::ZERO;
    let plane_position = DVec3::new(1.0, 0.0, -1.0);
    let plane_diff = plane_position - plane_origin;
    assert_approx!(plane_diff.dot(plane.normal), 0.0, epsilon::EPSILON16);

    let transform = DMat4::from_scale(DVec3::new(4.0, 1.0, 10.0));
    let transform_plane = plane.transform(&transform);
    // Matrix4.multiplyByPointAsVector(transform, planeDiff) == transform * planeDiff (vec3)
    let transform_plane_diff = transform.transform_vector3(plane_diff);
    assert_approx!(
        transform_plane_diff.dot(transform_plane.normal),
        0.0,
        epsilon::EPSILON16
    );
}
