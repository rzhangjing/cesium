use cesium_core::cartesian3::Cartesian3;
use cesium_core::intersect::Intersect;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::oriented_bounding_box::OrientedBoundingBox;
use cesium_core::plane::Plane;

#[test]
fn default_constructor() {
    let obb = OrientedBoundingBox::default();
    assert_eq!(obb.center, Cartesian3::ZERO);
    assert_eq!(obb.half_axes, Matrix3::ZERO);
}

#[test]
fn constructor_with_parameters() {
    let center = Cartesian3::new(1.0, 2.0, 3.0);
    let half_axes = Matrix3::IDENTITY;
    let obb = OrientedBoundingBox::new(Some(&center), Some(&half_axes));
    assert_eq!(obb.center, center);
    assert_eq!(obb.half_axes, half_axes);
}

#[test]
fn constructor_defaults_to_zero() {
    let obb = OrientedBoundingBox::new(None, None);
    assert_eq!(obb.center, Cartesian3::ZERO);
}

#[test]
fn equals_works() {
    let center = Cartesian3::new(1.0, 2.0, 3.0);
    let half_axes = Matrix3::IDENTITY;
    let obb1 = OrientedBoundingBox::new(Some(&center), Some(&half_axes));
    let obb2 = OrientedBoundingBox::new(Some(&center), Some(&half_axes));
    assert!(obb1.equals(Some(&obb2)));

    let obb3 = OrientedBoundingBox::new(Some(&Cartesian3::ZERO), Some(&half_axes));
    assert!(!obb1.equals(Some(&obb3)));
}

#[test]
fn intersect_plane_splits() {
    // An OBB centered at origin with unit half-axes
    let center = Cartesian3::ZERO;
    let half_axes = Matrix3::new(
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    );
    let obb = OrientedBoundingBox::new(Some(&center), Some(&half_axes));

    // Plane at x=0.5 should split the box
    let plane = Plane::new(&Cartesian3::UNIT_X, -0.5);
    let result = OrientedBoundingBox::intersect_plane(&obb, &plane);
    assert_eq!(result, Intersect::Intersecting);
}

#[test]
fn intersect_plane_in_front() {
    let center = Cartesian3::ZERO;
    let half_axes = Matrix3::new(
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    );
    let obb = OrientedBoundingBox::new(Some(&center), Some(&half_axes));

    // Plane at x=5 should be in front of the box
    let plane = Plane::new(&Cartesian3::UNIT_X, -5.0);
    let result = OrientedBoundingBox::intersect_plane(&obb, &plane);
    assert_eq!(result, Intersect::Outside);
}
