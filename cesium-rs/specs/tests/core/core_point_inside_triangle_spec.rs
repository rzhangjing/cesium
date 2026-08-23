//! Tests for `cesium_core::point_inside_triangle`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::point_inside_triangle::point_inside_triangle;

#[test]
fn centroid_is_inside() {
    let p0 = Cartesian3::new(0.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 0.0);
    let point = Cartesian3::new(0.25, 0.25, 0.0);
    assert!(point_inside_triangle(&point, &p0, &p1, &p2));
}

#[test]
fn point_outside_triangle() {
    let p0 = Cartesian3::new(0.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 0.0);
    let point = Cartesian3::new(2.0, 2.0, 0.0);
    assert!(!point_inside_triangle(&point, &p0, &p1, &p2));
}

#[test]
fn vertex_is_not_strictly_inside() {
    let p0 = Cartesian3::new(0.0, 0.0, 0.0);
    let p1 = Cartesian3::new(1.0, 0.0, 0.0);
    let p2 = Cartesian3::new(0.0, 1.0, 0.0);
    // A vertex has barycentric coords (1,0,0) → not strictly > 0 for all
    assert!(!point_inside_triangle(&p0, &p0, &p1, &p2));
}
