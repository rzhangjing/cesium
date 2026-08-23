//! Tests for `cesium_core::Occluder`.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::occluder::Occluder;
use cesium_core::visibility::Visibility;

#[test]
fn constructor_sets_position_and_radius() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let camera = Cartesian3::new(0.0, 0.0, 10.0);
    let occ = Occluder::new(&bs, &camera);
    assert!((occ.position().x).abs() < 1e-10);
    assert!((occ.radius() - 1.0).abs() < 1e-10);
}

#[test]
fn point_in_front_of_occluder_is_visible() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let camera = Cartesian3::new(0.0, 0.0, 10.0);
    let occ = Occluder::new(&bs, &camera);
    // Point between camera and occluder, slightly in front
    let point = Cartesian3::new(0.0, 0.0, 2.0);
    assert!(occ.is_point_visible(&point));
}

#[test]
fn point_behind_occluder_is_not_visible() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let camera = Cartesian3::new(0.0, 0.0, 10.0);
    let occ = Occluder::new(&bs, &camera);
    // Point behind the occluder
    let point = Cartesian3::new(0.0, 0.0, -5.0);
    assert!(!occ.is_point_visible(&point));
}

#[test]
fn bounding_sphere_visible_when_in_front() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let camera = Cartesian3::new(0.0, 0.0, 10.0);
    let occ = Occluder::new(&bs, &camera);
    let occludee = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 3.0), 0.5);
    assert!(occ.is_bounding_sphere_visible(&occludee));
}

#[test]
fn compute_visibility_returns_full_when_visible() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let camera = Cartesian3::new(0.0, 0.0, 10.0);
    let occ = Occluder::new(&bs, &camera);
    let occludee = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 3.0), 0.5);
    assert_eq!(occ.compute_visibility(&occludee), Visibility::Full);
}

#[test]
fn compute_visibility_returns_none_when_occluded() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let camera = Cartesian3::new(0.0, 0.0, 10.0);
    let occ = Occluder::new(&bs, &camera);
    let occludee = BoundingSphere::new(Cartesian3::new(0.0, 0.0, -5.0), 0.5);
    assert_eq!(occ.compute_visibility(&occludee), Visibility::None);
}

#[test]
fn camera_inside_occluder_returns_no_horizon() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 10.0);
    let camera = Cartesian3::new(0.0, 0.0, 1.0); // inside the occluder
    let occ = Occluder::new(&bs, &camera);
    // Point far away should not be visible (camera inside occluder)
    let point = Cartesian3::new(100.0, 0.0, 0.0);
    assert!(!occ.is_point_visible(&point));
}
