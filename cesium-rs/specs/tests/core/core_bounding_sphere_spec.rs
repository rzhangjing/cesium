//! Specs for `BoundingSphere` — mirrors `Specs/Core/BoundingSphereSpec.js`.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::intersect::Intersect;
use cesium_core::plane::Plane;

#[test]
fn default_constructor() {
    let bs = BoundingSphere::default();
    assert_eq!(bs.center, Cartesian3::ZERO);
    assert_eq!(bs.radius, 0.0);
}

#[test]
fn constructor_with_values() {
    let bs = BoundingSphere::new(Cartesian3::new(1.0, 2.0, 3.0), 5.0);
    assert_eq!(bs.center.x, 1.0);
    assert_eq!(bs.center.y, 2.0);
    assert_eq!(bs.center.z, 3.0);
    assert_eq!(bs.radius, 5.0);
}

#[test]
fn from_points_empty() {
    let bs = BoundingSphere::from_points(&[], None);
    assert_eq!(bs.center, Cartesian3::ZERO);
    assert_eq!(bs.radius, 0.0);
}

#[test]
fn from_points_single() {
    let pts = vec![Cartesian3::new(1.0, 2.0, 3.0)];
    let bs = BoundingSphere::from_points(&pts, None);
    assert_eq!(bs.radius, 0.0);
}

#[test]
fn from_points_encloses_all() {
    let pts = vec![
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(-1.0, 0.0, 0.0),
        Cartesian3::new(0.0, 1.0, 0.0),
        Cartesian3::new(0.0, -1.0, 0.0),
    ];
    let bs = BoundingSphere::from_points(&pts, None);
    for p in &pts {
        let dist = Cartesian3::distance(&bs.center, p);
        assert!(dist <= bs.radius + 1e-10, "point not enclosed");
    }
}

#[test]
fn from_vertices_basic() {
    let verts = vec![
        1.0, 0.0, 0.0,
        -1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
    ];
    let bs = BoundingSphere::from_vertices(&verts, None, None, None);
    assert!(bs.radius > 0.0);
}

#[test]
fn from_corner_points() {
    let corner = Cartesian3::new(-1.0, -1.0, -1.0);
    let opposite = Cartesian3::new(1.0, 1.0, 1.0);
    let bs = BoundingSphere::from_corner_points(&corner, &opposite, None);
    assert_eq!(bs.center, Cartesian3::ZERO);
    let expected_radius = 3.0f64.sqrt();
    assert!((bs.radius - expected_radius).abs() < 1e-10);
}

#[test]
fn from_ellipsoid_wgs84() {
    let bs = BoundingSphere::from_ellipsoid(&Ellipsoid::WGS84, None);
    assert_eq!(bs.center, Cartesian3::ZERO);
    assert!((bs.radius - 6378137.0).abs() < 1e-6);
}

#[test]
fn union_containing() {
    let a = BoundingSphere::new(Cartesian3::ZERO, 10.0);
    let b = BoundingSphere::new(Cartesian3::new(1.0, 0.0, 0.0), 1.0);
    let u = BoundingSphere::union(&a, &b, None);
    assert_eq!(u.radius, 10.0);
}

#[test]
fn union_disjoint() {
    let a = BoundingSphere::new(Cartesian3::new(-5.0, 0.0, 0.0), 1.0);
    let b = BoundingSphere::new(Cartesian3::new(5.0, 0.0, 0.0), 1.0);
    let u = BoundingSphere::union(&a, &b, None);
    assert!(u.radius >= 6.0);
}

#[test]
fn intersect_plane_inside() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 1.0), 0.5);
    let plane = Plane::new(&Cartesian3::UNIT_Z, -0.0);
    assert_eq!(BoundingSphere::intersect_plane(&bs, &plane), Intersect::Inside);
}

#[test]
fn intersect_plane_outside() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, -5.0), 0.5);
    let plane = Plane::new(&Cartesian3::UNIT_Z, 0.0);
    assert_eq!(BoundingSphere::intersect_plane(&bs, &plane), Intersect::Outside);
}

#[test]
fn intersect_plane_intersecting() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.3), 0.5);
    let plane = Plane::new(&Cartesian3::UNIT_Z, 0.0);
    assert_eq!(BoundingSphere::intersect_plane(&bs, &plane), Intersect::Intersecting);
}

#[test]
fn distance_squared_to_outside() {
    let bs = BoundingSphere::new(Cartesian3::ZERO, 1.0);
    let pt = Cartesian3::new(3.0, 0.0, 0.0);
    let d2 = BoundingSphere::distance_squared_to(&bs, &pt);
    assert!((d2 - 4.0).abs() < 1e-10); // (3 - 1)^2 = 4
}

#[test]
fn distance_squared_to_inside() {
    let bs = BoundingSphere::new(Cartesian3::ZERO, 5.0);
    let pt = Cartesian3::new(1.0, 0.0, 0.0);
    let d2 = BoundingSphere::distance_squared_to(&bs, &pt);
    assert_eq!(d2, 0.0);
}

#[test]
fn pack_and_unpack() {
    let bs = BoundingSphere::new(Cartesian3::new(1.0, 2.0, 3.0), 4.0);
    let mut array = [0.0f64; 4];
    bs.pack(&mut array, 0);
    let unpacked = BoundingSphere::unpack(&array, 0, None);
    assert!(BoundingSphere::equals(&bs, &unpacked));
}

#[test]
fn equals_same() {
    let a = BoundingSphere::new(Cartesian3::new(1.0, 2.0, 3.0), 4.0);
    let b = BoundingSphere::new(Cartesian3::new(1.0, 2.0, 3.0), 4.0);
    assert!(BoundingSphere::equals(&a, &b));
}

#[test]
fn equals_different() {
    let a = BoundingSphere::new(Cartesian3::new(1.0, 2.0, 3.0), 4.0);
    let b = BoundingSphere::new(Cartesian3::new(1.0, 2.0, 3.0), 5.0);
    assert!(!BoundingSphere::equals(&a, &b));
}

#[test]
fn volume() {
    let bs = BoundingSphere::new(Cartesian3::ZERO, 1.0);
    let expected = (4.0 / 3.0) * std::f64::consts::PI;
    assert!((bs.volume() - expected).abs() < 1e-10);
}
