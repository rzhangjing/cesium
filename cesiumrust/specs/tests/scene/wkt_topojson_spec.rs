//! WKT + TopoJSON comprehensive specs
//! Tests cesium-vector crate: parse_wkt, to_wkt, decode_arc, resolve_linestring, etc.

use cesium_vector::{
    decode_arc, decode_arc_reversed, is_clockwise, parse_wkt, resolve_linestring, resolve_polygon,
    ring_area, to_wkt, Topology, Transform, WktError, WktGeometry,
};
use glam::DVec2;

// ═══════════════════════════════════════════════════════════════════════════════
// WKT Parsing
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn wkt_parse_point() {
    let geom = parse_wkt("POINT (30 10)").unwrap();
    assert_eq!(geom, WktGeometry::Point(DVec2::new(30.0, 10.0)));
}

#[test]
fn wkt_parse_point_negative_coords() {
    let geom = parse_wkt("POINT (-122.08 37.42)").unwrap();
    assert_eq!(geom, WktGeometry::Point(DVec2::new(-122.08, 37.42)));
}

#[test]
fn wkt_parse_point_case_insensitive() {
    let geom = parse_wkt("point (5 15)").unwrap();
    assert_eq!(geom, WktGeometry::Point(DVec2::new(5.0, 15.0)));
}

#[test]
fn wkt_parse_linestring() {
    let geom = parse_wkt("LINESTRING (30 10, 10 30, 40 40)").unwrap();
    if let WktGeometry::LineString(coords) = geom {
        assert_eq!(coords.len(), 3);
        assert_eq!(coords[0], DVec2::new(30.0, 10.0));
        assert_eq!(coords[1], DVec2::new(10.0, 30.0));
        assert_eq!(coords[2], DVec2::new(40.0, 40.0));
    } else {
        panic!("Expected LineString");
    }
}

#[test]
fn wkt_parse_polygon_no_holes() {
    let geom = parse_wkt("POLYGON ((30 10, 40 40, 20 40, 10 20, 30 10))").unwrap();
    if let WktGeometry::Polygon { exterior, interiors } = geom {
        assert_eq!(exterior.len(), 5);
        assert!(interiors.is_empty());
        assert_eq!(exterior[0], DVec2::new(30.0, 10.0));
    } else {
        panic!("Expected Polygon");
    }
}

#[test]
fn wkt_parse_polygon_with_hole() {
    let geom = parse_wkt(
        "POLYGON ((35 10, 45 45, 15 40, 10 20, 35 10), (20 30, 35 35, 30 20, 20 30))",
    )
    .unwrap();
    if let WktGeometry::Polygon { exterior, interiors } = geom {
        assert_eq!(exterior.len(), 5);
        assert_eq!(interiors.len(), 1);
        assert_eq!(interiors[0].len(), 4);
    } else {
        panic!("Expected Polygon");
    }
}

#[test]
fn wkt_parse_multipoint_with_parens() {
    let geom = parse_wkt("MULTIPOINT ((10 40), (40 30), (20 20), (30 10))").unwrap();
    if let WktGeometry::MultiPoint(points) = geom {
        assert_eq!(points.len(), 4);
        assert_eq!(points[0], DVec2::new(10.0, 40.0));
        assert_eq!(points[3], DVec2::new(30.0, 10.0));
    } else {
        panic!("Expected MultiPoint");
    }
}

#[test]
fn wkt_parse_multipoint_without_parens() {
    let geom = parse_wkt("MULTIPOINT (10 40, 40 30, 20 20)").unwrap();
    if let WktGeometry::MultiPoint(points) = geom {
        assert_eq!(points.len(), 3);
    } else {
        panic!("Expected MultiPoint");
    }
}

#[test]
fn wkt_parse_multilinestring() {
    let geom =
        parse_wkt("MULTILINESTRING ((10 10, 20 20, 10 40), (40 40, 30 30, 40 20, 30 10))").unwrap();
    if let WktGeometry::MultiLineString(lines) = geom {
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), 3);
        assert_eq!(lines[1].len(), 4);
    } else {
        panic!("Expected MultiLineString");
    }
}

#[test]
fn wkt_parse_multipolygon() {
    let geom = parse_wkt(
        "MULTIPOLYGON (((30 20, 45 40, 10 40, 30 20)), ((15 5, 40 10, 10 20, 5 10, 15 5)))",
    )
    .unwrap();
    if let WktGeometry::MultiPolygon(polys) = geom {
        assert_eq!(polys.len(), 2);
    } else {
        panic!("Expected MultiPolygon");
    }
}

#[test]
fn wkt_parse_geometry_collection() {
    let geom =
        parse_wkt("GEOMETRYCOLLECTION (POINT (4 6), LINESTRING (4 6, 7 10))").unwrap();
    if let WktGeometry::GeometryCollection(geoms) = geom {
        assert_eq!(geoms.len(), 2);
        assert!(matches!(geoms[0], WktGeometry::Point(_)));
        assert!(matches!(geoms[1], WktGeometry::LineString(_)));
    } else {
        panic!("Expected GeometryCollection");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// WKT Errors
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn wkt_error_unknown_type() {
    let result = parse_wkt("INVALID (30 10)");
    assert!(result.is_err());
    if let Err(WktError::UnknownType(_)) = result {
        // expected
    } else {
        panic!("Expected UnknownType error");
    }
}

#[test]
fn wkt_error_missing_parenthesis() {
    let result = parse_wkt("POINT 30 10");
    assert!(result.is_err());
}

#[test]
fn wkt_error_invalid_number() {
    let result = parse_wkt("POINT (abc def)");
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
// WKT Serialization (to_wkt)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn wkt_serialize_point() {
    let geom = WktGeometry::Point(DVec2::new(30.0, 10.0));
    assert_eq!(to_wkt(&geom), "POINT (30 10)");
}

#[test]
fn wkt_serialize_linestring() {
    let geom = WktGeometry::LineString(vec![
        DVec2::new(30.0, 10.0),
        DVec2::new(10.0, 30.0),
        DVec2::new(40.0, 40.0),
    ]);
    assert_eq!(to_wkt(&geom), "LINESTRING (30 10, 10 30, 40 40)");
}

#[test]
fn wkt_serialize_polygon() {
    let geom = WktGeometry::Polygon {
        exterior: vec![
            DVec2::new(30.0, 10.0),
            DVec2::new(40.0, 40.0),
            DVec2::new(20.0, 40.0),
            DVec2::new(30.0, 10.0),
        ],
        interiors: vec![],
    };
    let wkt = to_wkt(&geom);
    assert!(wkt.starts_with("POLYGON ("));
    assert!(wkt.contains("30 10, 40 40, 20 40, 30 10"));
}

#[test]
fn wkt_roundtrip_point() {
    let original = WktGeometry::Point(DVec2::new(-122.08, 37.42));
    let wkt_str = to_wkt(&original);
    let parsed = parse_wkt(&wkt_str).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn wkt_roundtrip_linestring() {
    let original = WktGeometry::LineString(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(2.0, 0.0),
    ]);
    let wkt_str = to_wkt(&original);
    let parsed = parse_wkt(&wkt_str).unwrap();
    assert_eq!(original, parsed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TopoJSON: decode_arc
// ═══════════════════════════════════════════════════════════════════════════════

fn simple_topology() -> Topology {
    Topology {
        objects: vec![],
        arcs: vec![
            vec![
                DVec2::new(0.0, 0.0),
                DVec2::new(1.0, 0.0),
                DVec2::new(1.0, 1.0),
            ],
            vec![
                DVec2::new(1.0, 1.0),
                DVec2::new(0.0, 1.0),
                DVec2::new(0.0, 0.0),
            ],
        ],
        transform: None,
        bbox: Some([0.0, 0.0, 1.0, 1.0]),
    }
}

#[test]
fn topojson_decode_arc_no_transform() {
    let topo = simple_topology();
    let arc = decode_arc(&topo, 0);
    assert_eq!(arc.len(), 3);
    assert_eq!(arc[0], DVec2::new(0.0, 0.0));
    assert_eq!(arc[1], DVec2::new(1.0, 0.0));
    assert_eq!(arc[2], DVec2::new(1.0, 1.0));
}

#[test]
fn topojson_decode_arc_reversed() {
    let topo = simple_topology();
    let arc = decode_arc_reversed(&topo, 0);
    assert_eq!(arc.len(), 3);
    assert_eq!(arc[0], DVec2::new(1.0, 1.0));
    assert_eq!(arc[2], DVec2::new(0.0, 0.0));
}

#[test]
fn topojson_decode_arc_out_of_bounds() {
    let topo = simple_topology();
    let arc = decode_arc(&topo, 99);
    assert!(arc.is_empty());
}

#[test]
fn topojson_decode_arc_with_transform() {
    let topo = Topology {
        objects: vec![],
        arcs: vec![vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1000.0, 0.0),
            DVec2::new(0.0, 1000.0),
        ]],
        transform: Some(Transform {
            scale: [0.001, 0.001],
            translate: [100.0, 50.0],
        }),
        bbox: None,
    };

    let arc = decode_arc(&topo, 0);
    assert_eq!(arc.len(), 3);
    // Delta encoding: (0,0) → (0+1000, 0+0) → (0+1000+0, 0+0+1000)
    // After transform: x*scale+translate
    assert!((arc[0].x - 100.0).abs() < 1e-10);
    assert!((arc[0].y - 50.0).abs() < 1e-10);
    assert!((arc[1].x - 101.0).abs() < 1e-10);
    assert!((arc[1].y - 50.0).abs() < 1e-10);
    assert!((arc[2].x - 101.0).abs() < 1e-10);
    assert!((arc[2].y - 51.0).abs() < 1e-10);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TopoJSON: resolve_linestring / resolve_polygon
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn topojson_resolve_linestring_single_arc() {
    let topo = simple_topology();
    let coords = resolve_linestring(&topo, &[0]);
    assert_eq!(coords.len(), 3);
    assert_eq!(coords[0], DVec2::new(0.0, 0.0));
}

#[test]
fn topojson_resolve_linestring_multiple_arcs() {
    let topo = simple_topology();
    let coords = resolve_linestring(&topo, &[0, 1]);
    // Arc 0: (0,0), (1,0), (1,1) → 3 points
    // Arc 1 (skip first): (0,1), (0,0) → 2 points
    assert_eq!(coords.len(), 5);
    assert_eq!(coords[0], DVec2::new(0.0, 0.0));
    assert_eq!(coords[4], DVec2::new(0.0, 0.0));
}

#[test]
fn topojson_resolve_polygon() {
    let topo = simple_topology();
    let rings = resolve_polygon(&topo, &[vec![0, 1]]);
    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].len(), 5);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TopoJSON: ring_area / is_clockwise
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn topojson_ring_area_ccw() {
    // Counter-clockwise unit square → positive area
    let ring = vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(0.0, 1.0),
    ];
    let area = ring_area(&ring);
    assert!((area - 1.0).abs() < 1e-10);
}

#[test]
fn topojson_ring_area_cw() {
    // Clockwise unit square → negative area
    let ring = vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(0.0, 1.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(1.0, 0.0),
    ];
    let area = ring_area(&ring);
    assert!((area + 1.0).abs() < 1e-10);
}

#[test]
fn topojson_ring_area_degenerate() {
    // Less than 3 points → 0
    let ring = vec![DVec2::new(0.0, 0.0), DVec2::new(1.0, 1.0)];
    assert_eq!(ring_area(&ring), 0.0);
}

#[test]
fn topojson_is_clockwise_true() {
    let ring = vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(0.0, 1.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(1.0, 0.0),
    ];
    assert!(is_clockwise(&ring));
}

#[test]
fn topojson_is_clockwise_false() {
    let ring = vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(0.0, 1.0),
    ];
    assert!(!is_clockwise(&ring));
}

// ═══════════════════════════════════════════════════════════════════════════════
// TopoJSON: Transform
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn topojson_transform_apply() {
    let transform = Transform {
        scale: [0.001, 0.002],
        translate: [100.0, 50.0],
    };
    let result = transform.apply(1000.0, 500.0);
    assert!((result.x - 101.0).abs() < 1e-10);
    assert!((result.y - 51.0).abs() < 1e-10);
}

#[test]
fn topojson_transform_identity() {
    let transform = Transform {
        scale: [1.0, 1.0],
        translate: [0.0, 0.0],
    };
    let result = transform.apply(42.0, 99.0);
    assert!((result.x - 42.0).abs() < 1e-10);
    assert!((result.y - 99.0).abs() < 1e-10);
}
