use cesium_core::barycentric_coordinates::barycentric_coordinates_3d;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;

#[test]
fn evaluates_to_p0() {
    let p0 = Cartesian3::new(-1.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 1.0);
    let result = barycentric_coordinates_3d(&p0, &p0, &p1, &p2).unwrap();
    assert_eq!(result, Cartesian3::UNIT_X);
}

#[test]
fn evaluates_to_p1() {
    let p0 = Cartesian3::new(-1.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 1.0);
    let result = barycentric_coordinates_3d(&p1, &p0, &p1, &p2).unwrap();
    assert_eq!(result, Cartesian3::UNIT_Y);
}

#[test]
fn evaluates_to_p2() {
    let p0 = Cartesian3::new(-1.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 1.0);
    let result = barycentric_coordinates_3d(&p2, &p0, &p1, &p2).unwrap();
    assert_eq!(result, Cartesian3::UNIT_Z);
}

#[test]
fn evaluates_on_edge_p0_p1() {
    let p0 = Cartesian3::new(-1.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 1.0);
    let midpoint = Cartesian3::multiply_by_scalar_new(
        &Cartesian3::add_new(&p0, &p1),
        0.5,
    );
    let result = barycentric_coordinates_3d(&midpoint, &p0, &p1, &p2).unwrap();
    assert!((result.x - 0.5).abs() < CesiumMath::EPSILON14);
    assert!((result.y - 0.5).abs() < CesiumMath::EPSILON14);
    assert!((result.z - 0.0).abs() < CesiumMath::EPSILON14);
}

#[test]
fn evaluates_on_interior() {
    let p0 = Cartesian3::new(-1.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 1.0);
    let scalar = 1.0 / 3.0;
    let sum = Cartesian3::multiply_by_scalar_new(
        &Cartesian3::add_new(&Cartesian3::add_new(&p0, &p1), &p2),
        scalar,
    );
    let result = barycentric_coordinates_3d(&sum, &p0, &p1, &p2).unwrap();
    assert!((result.x - scalar).abs() < CesiumMath::EPSILON14);
    assert!((result.y - scalar).abs() < CesiumMath::EPSILON14);
    assert!((result.z - scalar).abs() < CesiumMath::EPSILON14);
}

#[test]
fn returns_none_for_colinear_points() {
    let p0 = Cartesian3::new(-1.0, -1.0, 0.0);
    let p1 = Cartesian3::new(0.0, 0.0, 0.0);
    let p2 = Cartesian3::new(1.0, 1.0, 0.0);
    let point = Cartesian3::new(0.5, 0.5, 0.0);
    assert!(barycentric_coordinates_3d(&point, &p0, &p1, &p2).is_none());
}
