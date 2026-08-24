//! Tests for small geometry constructors: Wall, Sphere, Circle, Ellipse, Rectangle, Polygon, Polyline, Corridor, Frustum.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::circle_geometry::CircleGeometry;
use cesium_core::corridor_geometry::CorridorGeometry;
use cesium_core::ellipse_geometry::EllipseGeometry;
use cesium_core::frustum_geometry::FrustumGeometry;
use cesium_core::polygon_geometry::PolygonGeometry;
use cesium_core::polyline_geometry::PolylineGeometry;
use cesium_core::rectangle::Rectangle;
use cesium_core::rectangle_geometry::RectangleGeometry;
use cesium_core::sphere_geometry::SphereGeometry;
use cesium_core::wall_geometry::WallGeometry;

// --- WallGeometry ---
#[test]
fn wall_geometry_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
    ];
    let wall = WallGeometry::new(positions, None, None, None, None, None);
    // Just verify construction doesn't panic
    let _ = wall;
}

#[test]
fn wall_geometry_from_constant_heights() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
    ];
    let wall = WallGeometry::from_constant_heights(positions, Some(0.0), Some(100.0), None, None);
    let _ = wall;
}

// --- SphereGeometry ---
#[test]
fn sphere_geometry_default_radius() {
    let sphere = SphereGeometry::new(None, None, None, None);
    let _ = sphere;
}

#[test]
fn sphere_geometry_custom_radius() {
    let sphere = SphereGeometry::new(Some(5.0), Some(8), Some(8), None);
    let _ = sphere;
}

// --- CircleGeometry ---
#[test]
fn circle_geometry_new() {
    let center = Cartesian3::new(1.0, 2.0, 3.0);
    let circle = CircleGeometry::new(center, 100.0, None, None, None, None, None, None);
    assert_eq!(circle.radius(), 100.0);
}

// --- EllipseGeometry ---
#[test]
fn ellipse_geometry_new() {
    let center = Cartesian3::new(0.0, 0.0, 0.0);
    let ellipse = EllipseGeometry::new(center, 1000.0, 500.0, None, None, None, None, None, None, None, None, None);
    let _ = ellipse;
}

// --- RectangleGeometry ---
#[test]
fn rectangle_geometry_new() {
    let rect = Rectangle::from_radians(-1.0, -0.5, 1.0, 0.5);
    let rg = RectangleGeometry::new(rect, None, None, None, None);
    let _ = rg;
}

// --- PolygonGeometry ---
#[test]
fn polygon_geometry_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(0.5, 1.0, 0.0),
    ];
    let poly = PolygonGeometry::new(positions, None, None, None, None, None, None, None, None, None, None, None);
    let _ = poly;
}

// --- PolylineGeometry ---
#[test]
fn polyline_geometry_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
    ];
    let pl = PolylineGeometry::new(positions, None, None, None, None, None, None);
    let _ = pl;
}

// --- CorridorGeometry ---
#[test]
fn corridor_geometry_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(2.0, 0.0, 0.0),
    ];
    let corridor = CorridorGeometry::new(positions, 10.0, None, None, None, None, None, None, None, None);
    let _ = corridor;
}

// --- FrustumGeometry ---
#[test]
fn frustum_geometry_new() {
    let origin = Cartesian3::new(0.0, 0.0, 0.0);
    let orientation = Cartesian4::new(0.0, 0.0, 0.0, 1.0);
    let fg = FrustumGeometry::new(origin, orientation, 1.0, 1000.0, 1.0, 1.0);
    let _ = fg;
}
