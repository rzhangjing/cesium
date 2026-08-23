use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::intersection_tests::IntersectionTests;
use cesium_core::math::CesiumMath;
use cesium_core::plane::Plane;
use cesium_core::ray::Ray;

#[test]
fn ray_plane_intersects() {
    let ray = Ray::new(
        Some(&Cartesian3::new(2.0, 0.0, 0.0)),
        Some(&Cartesian3::new(-1.0, 0.0, 0.0)),
    );
    let plane = Plane::new(&Cartesian3::UNIT_X, -1.0);
    let pt = IntersectionTests::ray_plane(&ray, &plane).unwrap();
    assert!((pt.x - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((pt.y).abs() < CesiumMath::EPSILON14);
    assert!((pt.z).abs() < CesiumMath::EPSILON14);
}

#[test]
fn ray_plane_misses() {
    let ray = Ray::new(
        Some(&Cartesian3::new(2.0, 0.0, 0.0)),
        Some(&Cartesian3::new(1.0, 0.0, 0.0)),
    );
    let plane = Plane::new(&Cartesian3::UNIT_X, -1.0);
    assert!(IntersectionTests::ray_plane(&ray, &plane).is_none());
}

#[test]
fn ray_plane_parallel_misses() {
    let ray = Ray::new(
        Some(&Cartesian3::new(2.0, 0.0, 0.0)),
        Some(&Cartesian3::new(0.0, 1.0, 0.0)),
    );
    let plane = Plane::new(&Cartesian3::UNIT_X, -1.0);
    assert!(IntersectionTests::ray_plane(&ray, &plane).is_none());
}

#[test]
fn ray_triangle_front_face() {
    let p0 = Cartesian3::new(-1.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 0.0);
    let ray = Ray::new(Some(&Cartesian3::UNIT_Z), Some(&Cartesian3::new(0.0, 0.0, -1.0)));
    let pt = IntersectionTests::ray_triangle(&ray, &p0, &p1, &p2, false).unwrap();
    assert!((pt.x).abs() < CesiumMath::EPSILON14);
    assert!((pt.y).abs() < CesiumMath::EPSILON14);
    assert!((pt.z).abs() < CesiumMath::EPSILON14);
}

#[test]
fn ray_triangle_misses() {
    let p0 = Cartesian3::new(-1.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 0.0);
    let ray = Ray::new(
        Some(&Cartesian3::new(5.0, 5.0, 1.0)),
        Some(&Cartesian3::new(0.0, 0.0, -1.0)),
    );
    assert!(IntersectionTests::ray_triangle(&ray, &p0, &p1, &p2, false).is_none());
}

#[test]
fn ray_sphere_intersects() {
    let ray = Ray::new(
        Some(&Cartesian3::new(-5.0, 0.0, 0.0)),
        Some(&Cartesian3::new(1.0, 0.0, 0.0)),
    );
    let sphere = BoundingSphere::new(Cartesian3::ZERO, 1.0);
    let interval = IntersectionTests::ray_sphere(&ray, &sphere).unwrap();
    assert!(interval.start > 0.0);
    assert!(interval.stop > interval.start);
}

#[test]
fn ray_sphere_misses() {
    let ray = Ray::new(
        Some(&Cartesian3::new(-5.0, 5.0, 0.0)),
        Some(&Cartesian3::new(1.0, 0.0, 0.0)),
    );
    let sphere = BoundingSphere::new(Cartesian3::ZERO, 1.0);
    assert!(IntersectionTests::ray_sphere(&ray, &sphere).is_none());
}

#[test]
fn ray_ellipsoid_intersects() {
    let ray = Ray::new(
        Some(&Cartesian3::new(-2.0e6, 0.0, 0.0)),
        Some(&Cartesian3::new(1.0, 0.0, 0.0)),
    );
    let interval = IntersectionTests::ray_ellipsoid(&ray, &Ellipsoid::WGS84).unwrap();
    // Ray origin (-2e6, 0, 0) is inside the ellipsoid (radius ~6.4e6)
    // so interval.start == 0.0 and interval.stop > 0.0
    assert_eq!(interval.start, 0.0);
    assert!(interval.stop > 0.0);
}

#[test]
fn ray_ellipsoid_misses() {
    let ray = Ray::new(
        Some(&Cartesian3::new(0.0, 0.0, -2.0e6)),
        Some(&Cartesian3::new(0.0, 0.0, 1.0)),
    );
    // Ray going straight up from south pole should miss (goes away from ellipsoid)
    // Actually it depends on direction. Let's use a ray that clearly misses:
    let ray2 = Ray::new(
        Some(&Cartesian3::new(1.0e12, 0.0, 0.0)),
        Some(&Cartesian3::new(1.0, 0.0, 0.0)),
    );
    assert!(IntersectionTests::ray_ellipsoid(&ray2, &Ellipsoid::WGS84).is_none());
}
