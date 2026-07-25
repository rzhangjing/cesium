//! Scene/Vector3DTileContentSpec.js → Rust integration tests

use cesium_vector::{WktGeometry, parse_wkt, to_wkt, MvtGeometryType};
use glam::DVec2;

// === WKT parsing ===

#[test]
fn test_parse_wkt_point() {
    let geom = parse_wkt("POINT (30 10)").unwrap();
    assert!(matches!(geom, WktGeometry::Point(_)));
}

#[test]
fn test_parse_wkt_linestring() {
    let geom = parse_wkt("LINESTRING (30 10, 10 30, 40 40)").unwrap();
    assert!(matches!(geom, WktGeometry::LineString(_)));
}

#[test]
fn test_parse_wkt_polygon() {
    let geom = parse_wkt("POLYGON ((30 10, 40 40, 20 40, 10 20, 30 10))").unwrap();
    assert!(matches!(geom, WktGeometry::Polygon { .. }));
}

#[test]
fn test_parse_wkt_multipoint() {
    let geom = parse_wkt("MULTIPOINT ((10 40), (40 30))").unwrap();
    assert!(matches!(geom, WktGeometry::MultiPoint(_)));
}

// === WKT output ===

#[test]
fn test_to_wkt_point() {
    let geom = WktGeometry::Point(DVec2::new(30.0, 10.0));
    let wkt = to_wkt(&geom);
    assert!(wkt.contains("POINT"));
}

// === MvtGeometryType ===

#[test]
fn test_mvt_geometry_type() {
    assert_ne!(MvtGeometryType::Point, MvtGeometryType::LineString);
    assert_ne!(MvtGeometryType::LineString, MvtGeometryType::Polygon);
}
