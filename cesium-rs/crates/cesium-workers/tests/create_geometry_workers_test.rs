//! Behavior specs for the `create*Geometry` worker re-wiring (SEM-3).
//!
//! In CesiumJS each `create*Geometry.js` worker constructs the geometry
//! object and delegates to its `createGeometry` implementation. The Rust
//! `_unpacked` variants mirror that body; these specs assert that the
//! delegation now reaches the ported `cesium-core` geometry generators
//! and produces real geometry instead of the previous `None` stubs.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::geometry::Geometry;
use cesium_workers::create_box_geometry::create_box_geometry_unpacked;
use cesium_workers::create_box_outline_geometry::create_box_outline_geometry_unpacked;
use cesium_workers::create_corridor_geometry::create_corridor_geometry_unpacked;
use cesium_workers::create_corridor_outline_geometry::create_corridor_outline_geometry_unpacked;
use cesium_workers::create_cylinder_geometry::create_cylinder_geometry_unpacked;
use cesium_workers::create_cylinder_outline_geometry::create_cylinder_outline_geometry_unpacked;
use cesium_workers::create_ellipse_geometry::create_ellipse_geometry_unpacked;
use cesium_workers::create_ellipse_outline_geometry::create_ellipse_outline_geometry_unpacked;
use cesium_workers::create_ellipsoid_geometry::create_ellipsoid_geometry_unpacked;
use cesium_workers::create_plane_geometry::create_plane_geometry_unpacked;
use cesium_workers::create_plane_outline_geometry::create_plane_outline_geometry_unpacked;
use cesium_workers::create_polygon_geometry::create_polygon_geometry_unpacked;
use cesium_workers::create_polygon_outline_geometry::create_polygon_outline_geometry_unpacked;
use cesium_workers::create_polyline_volume_geometry::create_polyline_volume_geometry_unpacked;
use cesium_workers::create_polyline_volume_outline_geometry::create_polyline_volume_outline_geometry_unpacked;
use cesium_workers::create_sphere_geometry::create_sphere_geometry_unpacked;

/// Every real geometry carries a position attribute with vertex data.
fn assert_real_geometry(geometry: Option<Geometry>) -> Geometry {
    let geometry = geometry.expect("worker must delegate to ported createGeometry");
    let position = geometry
        .attributes
        .get("position")
        .expect("geometry must have a position attribute");
    assert!(
        !position.values.is_empty(),
        "position attribute must contain vertex data"
    );
    geometry
}

fn degree_positions() -> Vec<Cartesian3> {
    [(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)]
        .iter()
        .map(|&(lon, lat)| Cartesian3::from_degrees_new(lon, lat, None, None))
        .collect()
}

fn square_shape() -> Vec<Cartesian2> {
    vec![
        Cartesian2::new(-2.0, -2.0),
        Cartesian2::new(2.0, -2.0),
        Cartesian2::new(2.0, 2.0),
        Cartesian2::new(-2.0, 2.0),
    ]
}

#[test]
fn box_geometry_worker_produces_geometry() {
    let min = Cartesian3::new(-1.0, -1.0, -1.0);
    let max = Cartesian3::new(1.0, 1.0, 1.0);
    let geometry = assert_real_geometry(create_box_geometry_unpacked(&min, &max));
    assert!(geometry.indices.is_some());
}

#[test]
fn box_outline_geometry_worker_produces_geometry() {
    let min = Cartesian3::new(-1.0, -1.0, -1.0);
    let max = Cartesian3::new(1.0, 1.0, 1.0);
    let geometry = assert_real_geometry(create_box_outline_geometry_unpacked(&min, &max));
    assert!(geometry.indices.is_some());
}

#[test]
fn corridor_geometry_worker_produces_geometry() {
    let positions = degree_positions();
    let geometry = assert_real_geometry(create_corridor_geometry_unpacked(
        &positions,
        100_000.0,
        0.0,
        0.0,
    ));
    assert!(geometry.indices.is_some());
}

#[test]
fn corridor_outline_geometry_worker_produces_geometry() {
    let positions = degree_positions();
    assert_real_geometry(create_corridor_outline_geometry_unpacked(
        &positions,
        100_000.0,
    ));
}

#[test]
fn cylinder_geometry_worker_produces_geometry() {
    let geometry = assert_real_geometry(create_cylinder_geometry_unpacked(5.0, 3.0, 3.0, 16));
    assert!(geometry.indices.is_some());
}

#[test]
fn cylinder_outline_geometry_worker_produces_geometry() {
    assert_real_geometry(create_cylinder_outline_geometry_unpacked(5.0, 3.0, 3.0, 16));
}

#[test]
fn ellipse_geometry_worker_produces_geometry() {
    let center = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let geometry = assert_real_geometry(create_ellipse_geometry_unpacked(
        center,
        500_000.0,
        300_000.0,
        None,
    ));
    assert!(geometry.indices.is_some());
}

#[test]
fn ellipse_outline_geometry_worker_produces_geometry() {
    let center = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    assert_real_geometry(create_ellipse_outline_geometry_unpacked(
        &center,
        500_000.0,
        300_000.0,
    ));
}

#[test]
fn ellipsoid_geometry_worker_produces_geometry() {
    let radii = Cartesian3::new(1_000_000.0, 1_000_000.0, 500_000.0);
    let geometry = assert_real_geometry(create_ellipsoid_geometry_unpacked(&radii));
    assert!(geometry.indices.is_some());
}

#[test]
fn plane_geometry_worker_produces_geometry() {
    let geometry = assert_real_geometry(create_plane_geometry_unpacked(None));
    assert!(geometry.indices.is_some());
}

#[test]
fn plane_outline_geometry_worker_produces_geometry() {
    assert_real_geometry(create_plane_outline_geometry_unpacked());
}

#[test]
fn polygon_geometry_worker_produces_geometry() {
    let positions = degree_positions();
    let geometry = assert_real_geometry(create_polygon_geometry_unpacked(&positions, 0.0, 0.0));
    assert!(geometry.indices.is_some());
}

#[test]
fn polygon_outline_geometry_worker_produces_geometry() {
    let positions = degree_positions();
    assert_real_geometry(create_polygon_outline_geometry_unpacked(
        &positions, 0.0, 0.0,
    ));
}

#[test]
fn polyline_volume_geometry_worker_produces_geometry() {
    let polyline = degree_positions();
    let geometry = assert_real_geometry(create_polyline_volume_geometry_unpacked(
        &polyline,
        &square_shape(),
    ));
    assert!(geometry.indices.is_some());
}

#[test]
fn polyline_volume_outline_geometry_worker_produces_geometry() {
    let polyline = degree_positions();
    assert_real_geometry(create_polyline_volume_outline_geometry_unpacked(
        &polyline,
        &square_shape(),
    ));
}

#[test]
fn sphere_geometry_worker_produces_geometry() {
    let geometry = assert_real_geometry(create_sphere_geometry_unpacked(100.0));
    assert!(geometry.indices.is_some());
}
