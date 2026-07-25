//! GeometryUpdater specs - ported from DataSources/*GeometryUpdaterSpec.js
//! Covers: update_box/cylinder/ellipse/corridor/rectangle/wall/ellipsoid/polyline_graphics

use cesium_datasource::geometry_updater::{
    cartographic_to_cartesian, positions_to_cartesian, update_box_graphics,
    update_corridor_graphics, update_cylinder_graphics, update_ellipse_graphics,
    update_ellipsoid_graphics, update_polyline_graphics, update_rectangle_graphics,
    update_wall_graphics,
};
use cesium_datasource::{
    BoxGraphics, CorridorGraphics, CylinderGraphics, EllipseGraphics, EllipsoidGraphics,
    Entity, PolylineGraphics, RectangleGraphics, WallGraphics, NumberProperty, Property,
};
use cesium_geospatial::Ellipsoid;

fn wgs84() -> Ellipsoid {
    Ellipsoid::WGS84
}

fn entity_with_position() -> Entity {
    let mut e = Entity::new("test-entity");
    e.position = Property::Constant([0.0, 0.0, 0.0]); // lon, lat, height in radians/meters
    e
}

// ─── Box ────────────────────────────────────────────────────────────────────

#[test]
fn update_box_produces_fill() {
    let entity = entity_with_position();
    let mut g = BoxGraphics::default();
    g.dimensions = Property::Constant([100.0, 200.0, 300.0]);
    let result = update_box_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty(), "box should produce geometry");
    assert!(!result.fill_instances.is_empty());
}

#[test]
fn update_box_empty_dimensions() {
    let entity = entity_with_position();
    let g = BoxGraphics::default(); // dimensions = [0,0,0]
    let result = update_box_graphics(&entity, &g, 0.0, &wgs84());
    assert!(result.is_empty(), "zero-size box should produce no geometry");
}

// ─── Cylinder ───────────────────────────────────────────────────────────────

#[test]
fn update_cylinder_produces_fill() {
    let entity = entity_with_position();
    let mut g = CylinderGraphics::default();
    g.length = NumberProperty::Constant(100.0);
    g.top_radius = NumberProperty::Constant(50.0);
    g.bottom_radius = NumberProperty::Constant(50.0);
    let result = update_cylinder_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty());
    assert!(!result.fill_instances.is_empty());
}

// ─── Ellipse ────────────────────────────────────────────────────────────────

#[test]
fn update_ellipse_produces_fill() {
    let entity = entity_with_position();
    let mut g = EllipseGraphics::default();
    g.semi_major_axis = NumberProperty::Constant(100_000.0);
    g.semi_minor_axis = NumberProperty::Constant(50_000.0);
    let result = update_ellipse_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty());
    assert!(!result.fill_instances.is_empty());
    let inst = &result.fill_instances[0];
    assert!(!inst.geometry.positions.is_empty());
}

// ─── Corridor ───────────────────────────────────────────────────────────────

#[test]
fn update_corridor_produces_fill() {
    let entity = entity_with_position();
    let mut g = CorridorGraphics::default();
    g.positions = Property::Constant(vec![
        [0.0, 0.0, 0.0],
        [0.01, 0.0, 0.0],
        [0.01, 0.01, 0.0],
    ]);
    g.width = NumberProperty::Constant(1000.0);
    let result = update_corridor_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty());
}

// ─── Rectangle ──────────────────────────────────────────────────────────────

#[test]
fn update_rectangle_produces_fill() {
    let entity = entity_with_position();
    let mut g = RectangleGraphics::default();
    g.coordinates = Property::Constant([0.0, 0.0, 0.1, 0.1]); // west, south, east, north
    let result = update_rectangle_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty());
}

// ─── Wall ───────────────────────────────────────────────────────────────────

#[test]
fn update_wall_produces_fill() {
    let entity = entity_with_position();
    let mut g = WallGraphics::default();
    g.positions = Property::Constant(vec![
        [0.0, 0.0, 0.0],
        [0.01, 0.0, 0.0],
        [0.01, 0.01, 0.0],
    ]);
    let result = update_wall_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty());
}

// ─── Ellipsoid ──────────────────────────────────────────────────────────────

#[test]
fn update_ellipsoid_produces_fill() {
    let entity = entity_with_position();
    let mut g = EllipsoidGraphics::default();
    g.radii = Property::Constant([100.0, 100.0, 100.0]);
    let result = update_ellipsoid_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty());
}

// ─── Polyline ───────────────────────────────────────────────────────────────

#[test]
fn update_polyline_produces_fill() {
    let entity = entity_with_position();
    let mut g = PolylineGraphics::default();
    g.positions = Property::Constant(vec![
        [0.0, 0.0, 0.0],
        [0.01, 0.01, 0.0],
    ]);
    let result = update_polyline_graphics(&entity, &g, 0.0, &wgs84());
    assert!(!result.is_empty());
}

// ─── EntityGeometry helpers ─────────────────────────────────────────────────

#[test]
fn entity_geometry_instance_count() {
    let entity = entity_with_position();
    let mut g = BoxGraphics::default();
    g.dimensions = Property::Constant([10.0, 10.0, 10.0]);
    let result = update_box_graphics(&entity, &g, 0.0, &wgs84());
    assert_eq!(result.instance_count(), result.fill_instances.len() + result.outline_instances.len());
}

// ─── Coordinate conversion ──────────────────────────────────────────────────

#[test]
fn cartographic_to_cartesian_origin() {
    let e = wgs84();
    let pos = cartographic_to_cartesian(&[0.0, 0.0, 0.0], &e);
    // At lon=0, lat=0, height=0 → on equator at prime meridian
    assert!((pos.x - e.radii().x).abs() < 1.0);
    assert!(pos.y.abs() < 1.0);
    assert!(pos.z.abs() < 1.0);
}

#[test]
fn positions_to_cartesian_multiple() {
    let e = wgs84();
    let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let result = positions_to_cartesian(&positions, &e);
    assert_eq!(result.len(), 2);
    assert!((result[0].x - result[1].x).abs() > 1.0); // different positions
}
