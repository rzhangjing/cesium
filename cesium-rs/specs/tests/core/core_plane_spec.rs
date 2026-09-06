//! Mirrors packages/engine/Specs/Core/PlaneSpec.js
//!
//! Tests that depend on `Matrix4` (transform specs) are `#[ignore]`d until
//! `Matrix4` is ported (M1-W2).

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::math::CesiumMath;
use cesium_core::plane::Plane;
use cesium_test_utils::{assert_approx_eq_f64, expect_to_throw_dev_error};

fn normalize(v: &Cartesian3) -> Cartesian3 {
    Cartesian3::normalize_new(v)
}

// --- constructor ---

#[test]
fn constructs() {
    let normal = Cartesian3::UNIT_X;
    let distance = 1.0;
    let plane = Plane::new(&normal, distance);
    assert_eq!(plane.normal, normal);
    assert_eq!(plane.distance, distance);
}

#[test]
fn constructor_throws_if_normal_is_not_normalized() {
    expect_to_throw_dev_error(|| {
        Plane::new(&Cartesian3::new(1.0, 2.0, 3.0), 0.0);
    });
}

// --- fromPointNormal ---

#[test]
fn constructs_from_a_point_and_a_normal() {
    let normal = normalize(&Cartesian3::new(1.0, 2.0, 3.0));
    let point = Cartesian3::new(4.0, 5.0, 6.0);
    let plane = Plane::from_point_normal_new(&point, &normal);
    assert_eq!(plane.normal, normal);
    assert_eq!(plane.distance, -Cartesian3::dot(&normal, &point));
}

#[test]
fn constructs_from_a_point_and_a_normal_with_result() {
    let normal = normalize(&Cartesian3::new(1.0, 2.0, 3.0));
    let point = Cartesian3::new(4.0, 5.0, 6.0);
    let mut plane = Plane::new(&Cartesian3::UNIT_X, 0.0);
    Plane::from_point_normal(&point, &normal, &mut plane);
    assert_eq!(plane.normal, normal);
    assert_eq!(plane.distance, -Cartesian3::dot(&normal, &point));
}

#[test]
fn from_point_normal_throws_if_normal_is_not_normalized() {
    expect_to_throw_dev_error(|| {
        Plane::from_point_normal(&Cartesian3::ZERO, &Cartesian3::ZERO, &mut Plane::new(&Cartesian3::UNIT_X, 0.0));
    });
}

// --- fromCartesian4 ---

#[test]
fn constructs_from_a_cartesian4_without_result() {
    let result = Plane::from_cartesian4_new(&Cartesian4::UNIT_X);
    assert_eq!(result.normal, Cartesian3::UNIT_X);
    assert_eq!(result.distance, 0.0);
}

#[test]
fn constructs_from_a_cartesian4_with_result() {
    let mut result = Plane::new(&Cartesian3::UNIT_X, 0.0);
    Plane::from_cartesian4(&Cartesian4::UNIT_X, &mut result);
    assert_eq!(result.normal, Cartesian3::UNIT_X);
    assert_eq!(result.distance, 0.0);
}

#[test]
fn from_cartesian4_throws_if_normal_is_not_normalized() {
    expect_to_throw_dev_error(|| {
        Plane::from_cartesian4_new(&Cartesian4::new(1.0, 2.0, 3.0, 4.0));
    });
}

// --- getPointDistance ---

#[test]
fn gets_the_distance_to_a_point() {
    let normal = normalize(&Cartesian3::new(1.0, 2.0, 3.0));
    let plane = Plane::new(&normal, 12.34);
    let point = Cartesian3::new(4.0, 5.0, 6.0);
    let expected = Cartesian3::dot(&plane.normal, &point) + plane.distance;
    assert_eq!(Plane::get_point_distance(&plane, &point), expected);
}

// --- projectPointOntoPlane ---

#[test]
fn projects_a_point_onto_the_plane_x() {
    let plane = Plane::new(&Cartesian3::UNIT_X, 0.0);
    let point = Cartesian3::new(1.0, 1.0, 0.0);
    let result = Plane::project_point_onto_plane_new(&plane, &point);
    assert_eq!(result, Cartesian3::new(0.0, 1.0, 0.0));
}

#[test]
fn projects_a_point_onto_the_plane_y() {
    let plane = Plane::new(&Cartesian3::UNIT_Y, 0.0);
    let point = Cartesian3::new(1.0, 1.0, 0.0);
    let result = Plane::project_point_onto_plane_new(&plane, &point);
    assert_eq!(result, Cartesian3::new(1.0, 0.0, 0.0));
}

#[test]
fn project_point_onto_plane_uses_result_parameter() {
    let plane = Plane::new(&Cartesian3::UNIT_X, 0.0);
    let point = Cartesian3::new(1.0, 1.0, 0.0);
    let mut result = Cartesian3::default();
    Plane::project_point_onto_plane(&plane, &point, &mut result);
    assert_eq!(result, Cartesian3::new(0.0, 1.0, 0.0));
}

// --- clone ---

#[test]
fn clones_a_plane_instance() {
    let normal = normalize(&Cartesian3::new(1.0, 2.0, 3.0));
    let distance = 4.0;
    let plane = Plane::new(&normal, distance);
    let result = Plane::clone_new(&plane);
    assert_eq!(result.normal, normal);
    assert_eq!(result.distance, distance);
}

#[test]
fn clones_a_plane_instance_into_a_result_parameter() {
    let normal = normalize(&Cartesian3::new(1.0, 2.0, 3.0));
    let distance = 4.0;
    let plane = Plane::new(&normal, distance);
    let mut result = Plane::new(&Cartesian3::UNIT_X, 1.0);
    Plane::clone_plane(&plane, &mut result);
    assert_eq!(result.normal, normal);
    assert_eq!(result.distance, distance);
}

// --- equals ---

#[test]
fn equals_returns_true_only_if_two_planes_are_equal() {
    let left = Plane::new(&Cartesian3::UNIT_X, 0.0);
    let mut right = Plane::new(&Cartesian3::UNIT_Y, 1.0);

    assert!(!Plane::equals(&left, &right));

    right.distance = 0.0;
    assert!(!Plane::equals(&left, &right));

    right.normal = Cartesian3::UNIT_X;
    assert!(Plane::equals(&left, &right));

    right.distance = 1.0;
    assert!(!Plane::equals(&left, &right));
}

// --- constants ---

#[test]
fn origin_xy_plane() {
    assert_eq!(Plane::ORIGIN_XY_PLANE.normal, Cartesian3::UNIT_Z);
    assert_eq!(Plane::ORIGIN_XY_PLANE.distance, 0.0);
}

#[test]
fn origin_yz_plane() {
    assert_eq!(Plane::ORIGIN_YZ_PLANE.normal, Cartesian3::UNIT_X);
    assert_eq!(Plane::ORIGIN_YZ_PLANE.distance, 0.0);
}

#[test]
fn origin_zx_plane() {
    assert_eq!(Plane::ORIGIN_ZX_PLANE.normal, Cartesian3::UNIT_Y);
    assert_eq!(Plane::ORIGIN_ZX_PLANE.distance, 0.0);
}

// --- transform ---

#[test]
fn transforms_a_plane_according_to_a_transform() {
    use cesium_core::matrix4::Matrix4;

    // Plane: normal = UNIT_X, distance = 0 (the YZ plane through origin).
    let plane = Plane::new(&Cartesian3::UNIT_X, 0.0);
    // Translation by (10, 0, 0).
    let transform = Matrix4::from_translation_new(&Cartesian3::new(10.0, 0.0, 0.0));
    let result = Plane::transform_new(&plane, &transform);

    // After translation, the normal stays the same but the plane moves.
    assert!((result.normal.x - 1.0).abs() < 1e-10);
    assert!(result.normal.y.abs() < 1e-10);
    assert!(result.normal.z.abs() < 1e-10);
    // The plane x=0 translated by (10,0,0) becomes x=10, i.e. distance = -10.
    assert!((result.distance - (-10.0)).abs() < 1e-10);
}

#[test]
fn transforms_a_plane_with_non_uniform_scale() {
    use cesium_core::matrix4::Matrix4;

    // Plane: normal = UNIT_Z, distance = 5.
    let plane = Plane::new(&Cartesian3::UNIT_Z, 5.0);
    // Non-uniform scale: (2, 3, 4).
    let transform = Matrix4::from_scale_new(&Cartesian3::new(2.0, 3.0, 4.0));
    let result = Plane::transform_new(&plane, &transform);

    // The normal should remain along Z after axis-aligned scaling.
    assert!(result.normal.x.abs() < 1e-10);
    assert!(result.normal.y.abs() < 1e-10);
    assert!((result.normal.z - 1.0).abs() < 1e-10);
    // Original closest point to origin: (0, 0, 5). Scaled: (0, 0, 20).
    // distance = -dot(normal, point) = -20.
    assert!((result.distance - (-20.0)).abs() < 1e-10);
}
