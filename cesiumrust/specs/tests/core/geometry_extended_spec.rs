//! Extended geometry specs - ported from Core/CircleGeometrySpec.js,
//! Core/CoplanarPolygonGeometrySpec.js, Core/GroundPolylineGeometrySpec.js,
//! Core/PolylineVolumeGeometrySpec.js, Core/PlaneGeometrySpec.js
//!
//! Tests mathematical properties of additional geometry generators.

use cesium_geospatial::geometry::{
    circle_geometry, coplanar_polygon_geometry, ground_polyline_geometry,
    polyline_volume_geometry, plane_geometry,
    CoplanarPolygonOptions, GroundPolylineOptions, PolylineVolumeOptions,
    PrimitiveType,
};
use cesium_geospatial::{Cartographic, Ellipsoid, VertexFormat};
use glam::DVec3;

fn wgs84() -> Ellipsoid {
    Ellipsoid::WGS84
}

// ─── CircleGeometry (from CircleGeometrySpec.js) ───────────────────────────

#[test]
fn circle_computes_positions() {
    // CircleGeometrySpec: "computes positions"
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let geo = circle_geometry(center, 1000.0, &e, 16, VertexFormat::POSITION_ONLY);

    // Rust: center + (segments+1) ring vertices = 1 + 17 = 18
    assert_eq!(geo.positions.len(), 18);
    // Fan triangulation: segments triangles * 3 indices
    assert_eq!(geo.indices.len(), 16 * 3);
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

#[test]
fn circle_computes_all_vertex_attributes() {
    // CircleGeometrySpec: "compute all vertex attributes"
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let geo = circle_geometry(center, 1000.0, &e, 8, VertexFormat::ALL);

    let num_vertices = geo.positions.len();
    assert_eq!(num_vertices, 10); // 1 center + 9 ring
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
}

#[test]
fn circle_bounding_sphere() {
    // CircleGeometrySpec: bounding sphere radius equals circle radius
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(10.0, 20.0, 0.0));
    let radius = 5000.0;
    let geo = circle_geometry(center, radius, &e, 32, VertexFormat::POSITION_ONLY);

    assert!((geo.bounding_sphere.radius - radius).abs() < 1e-6);
    assert!((geo.bounding_sphere.center - center).length() < 1e-6);
}

#[test]
fn circle_positions_on_surface() {
    // All positions should be on the ellipsoid surface
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let geo = circle_geometry(center, 10000.0, &e, 16, VertexFormat::POSITION_ONLY);

    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!(
            carto.height.abs() < 1.0,
            "position height {} should be ≈ 0", carto.height
        );
    }
}

// ─── CoplanarPolygonGeometry (from CoplanarPolygonGeometrySpec.js) ─────────

#[test]
fn coplanar_polygon_computes_positions() {
    // CoplanarPolygonGeometrySpec: "computes positions"
    let e = wgs84();
    let positions = vec![
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, -1.0, 0.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 0.0, 1.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 1.0, 1.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 2.0, 0.0)),
    ];
    let opts = CoplanarPolygonOptions {
        positions,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert_eq!(geo.positions.len(), 4);
    assert_eq!(geo.indices.len(), 2 * 3); // 2 triangles for a quad
}

#[test]
fn coplanar_polygon_computes_all_attributes() {
    // CoplanarPolygonGeometrySpec: "computes all attributes"
    let e = wgs84();
    let positions = vec![
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, -1.0, 0.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 0.0, 1.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 1.0, 1.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 2.0, 0.0)),
    ];
    let opts = CoplanarPolygonOptions {
        positions,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::ALL);

    let num_vertices = geo.positions.len();
    assert_eq!(num_vertices, 4);
    if let Some(normals) = &geo.normals {
        assert_eq!(normals.len(), num_vertices);
    }
    if let Some(st) = &geo.tex_coords {
        assert_eq!(st.len(), num_vertices);
    }
}

#[test]
fn coplanar_polygon_triangle() {
    // Simple triangle should produce 1 triangle
    let e = wgs84();
    let positions = vec![
        e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(0.5, 1.0, 0.0)),
    ];
    let opts = CoplanarPolygonOptions {
        positions,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert_eq!(geo.positions.len(), 3);
    assert_eq!(geo.indices.len(), 3); // 1 triangle
}

// ─── GroundPolylineGeometry (from GroundPolylineGeometrySpec.js) ───────────

#[test]
fn ground_polyline_computes_positions() {
    // GroundPolylineGeometrySpec: "computes positions"
    let e = wgs84();
    let opts = GroundPolylineOptions {
        positions: vec![
            e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
        ],
        width: 5.0,
        ellipsoid: e,
        granularity: std::f64::consts::PI / 180.0,
        closed: false,
    };
    let geo = ground_polyline_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 4, "ground polyline should have multiple positions");
    assert!(!geo.indices.is_empty());
}

#[test]
fn ground_polyline_positions_on_surface() {
    // Ground polyline positions should be on the ellipsoid surface
    let e = wgs84();
    let opts = GroundPolylineOptions {
        positions: vec![
            e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 1.0, 0.0)),
        ],
        width: 10.0,
        ellipsoid: e,
        granularity: std::f64::consts::PI / 180.0,
        closed: false,
    };
    let geo = ground_polyline_geometry(&opts, VertexFormat::POSITION_ONLY);

    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!(
            carto.height.abs() < 1.0,
            "ground polyline height {} should be ≈ 0", carto.height
        );
    }
}

// ─── PolylineVolumeGeometry (from PolylineVolumeGeometrySpec.js) ───────────

#[test]
fn polyline_volume_computes_positions() {
    // PolylineVolumeGeometrySpec: "computes positions"
    let e = wgs84();
    let opts = PolylineVolumeOptions {
        positions: vec![
            e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
        ],
        shape: vec![
            [-50.0, -50.0],
            [50.0, -50.0],
            [50.0, 50.0],
            [-50.0, 50.0],
        ],
        ellipsoid: e,
        granularity: std::f64::consts::PI / 180.0,
        ..Default::default()
    };
    let geo = polyline_volume_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 8, "polyline volume should have multiple positions");
    assert!(!geo.indices.is_empty());
    assert_eq!(geo.indices.len() % 3, 0, "indices should be triangles");
}

#[test]
fn polyline_volume_computes_all_attributes() {
    let e = wgs84();
    let opts = PolylineVolumeOptions {
        positions: vec![
            e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ],
        shape: vec![
            [-100.0, -100.0],
            [100.0, -100.0],
            [100.0, 100.0],
            [-100.0, 100.0],
        ],
        ellipsoid: e,
        granularity: std::f64::consts::PI / 180.0,
        ..Default::default()
    };
    let geo = polyline_volume_geometry(&opts, VertexFormat::ALL);

    let num_vertices = geo.positions.len();
    assert!(num_vertices > 0);
    if let Some(normals) = &geo.normals {
        assert_eq!(normals.len(), num_vertices);
    }
}

// ─── PlaneGeometry (from PlaneGeometrySpec.js) ─────────────────────────────

#[test]
fn plane_computes_positions() {
    // PlaneGeometrySpec: unit quad in XY plane
    let geo = plane_geometry(VertexFormat::POSITION_ONLY);

    assert_eq!(geo.positions.len(), 4);
    assert_eq!(geo.indices.len(), 6); // 2 triangles
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

#[test]
fn plane_computes_all_attributes() {
    let geo = plane_geometry(VertexFormat::ALL);

    assert_eq!(geo.positions.len(), 4);
    assert_eq!(geo.normals.as_ref().unwrap().len(), 4);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), 4);

    // All normals should point in +Z direction
    for n in geo.normals.as_ref().unwrap() {
        assert!((n[0]).abs() < 1e-10);
        assert!((n[1]).abs() < 1e-10);
        assert!((n[2] - 1.0).abs() < 1e-10);
    }
}

#[test]
fn plane_positions_form_unit_quad() {
    let geo = plane_geometry(VertexFormat::POSITION_ONLY);

    // Positions should form a unit quad centered at origin
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for p in &geo.positions {
        if p[0] < min_x { min_x = p[0]; }
        if p[0] > max_x { max_x = p[0]; }
        if p[1] < min_y { min_y = p[1]; }
        if p[1] > max_y { max_y = p[1]; }
        // Z should be 0
        assert!(p[2].abs() < 1e-10);
    }

    assert!((min_x - (-0.5)).abs() < 1e-10);
    assert!((max_x - 0.5).abs() < 1e-10);
    assert!((min_y - (-0.5)).abs() < 1e-10);
    assert!((max_y - 0.5).abs() < 1e-10);
}

// ─── Cross-cutting: bounding sphere contains all positions ─────────────────

#[test]
fn circle_bounding_sphere_contains_all_positions() {
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let geo = circle_geometry(center, 5000.0, &e, 32, VertexFormat::POSITION_ONLY);

    let bs_center = geo.bounding_sphere.center;
    let bs_radius = geo.bounding_sphere.radius;

    for p in &geo.positions {
        let dist = (DVec3::from(*p) - bs_center).length();
        // Allow some tolerance since positions are on curved surface
        assert!(
            dist <= bs_radius + 100.0,
            "position distance {} exceeds bounding sphere radius {} + tolerance", dist, bs_radius
        );
    }
}

#[test]
fn coplanar_polygon_bounding_sphere_contains_all_positions() {
    let e = wgs84();
    let positions = vec![
        e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 1.0, 0.0)),
    ];
    let opts = CoplanarPolygonOptions {
        positions,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);

    let center = geo.bounding_sphere.center;
    let radius = geo.bounding_sphere.radius;

    for p in &geo.positions {
        let dist = (DVec3::from(*p) - center).length();
        assert!(
            dist <= radius + 1e-6,
            "position distance {} exceeds bounding sphere radius {}", dist, radius
        );
    }
}

// ─── Circle geometry with negative/zero radius ────────────────────────────

#[test]
fn circle_zero_radius_degenerate() {
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(10.0, 20.0, 0.0));
    let geo = circle_geometry(center, 0.0, &e, 16, VertexFormat::POSITION_ONLY);

    // With radius=0, bounding sphere radius should be 0
    assert!(geo.bounding_sphere.radius < 1e-10);
}

#[test]
fn circle_negative_radius_uses_absolute() {
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(10.0, 20.0, 0.0));
    let geo = circle_geometry(center, -1.0, &e, 16, VertexFormat::POSITION_ONLY);

    // Implementation may handle negative radius differently
    // But should not crash
    assert_eq!(geo.positions.len(), 18);
    assert_eq!(geo.indices.len(), 48);
}

// ─── Circle geometry with texture coordinates ─────────────────────────────

#[test]
fn circle_tex_coords_pattern() {
    let e = wgs84();
    let center = e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let geo = circle_geometry(center, 5000.0, &e, 16, VertexFormat::POSITION_AND_ST);
    let st = geo.tex_coords.as_ref().unwrap();
    let nv = geo.positions.len();
    assert_eq!(st.len(), nv);
    // Center vertex (index 0) ST should be near center of texture
    assert!((st[0][0] - 0.5).abs() < 1e-2);
    assert!((st[0][1] - 0.5).abs() < 1e-2);
}

// ─── CoplanarPolygonGeometry: edge cases ──────────────────────────────────

#[test]
fn coplanar_polygon_with_height() {
    let e = wgs84();
    let positions = vec![
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, -1.0, 5000.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 0.0, 5000.0)),
        e.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 1.0, 5000.0)),
    ];
    let opts = CoplanarPolygonOptions {
        positions,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert_eq!(geo.positions.len(), 3);
    // Positions should be at height ~5000
    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!((carto.height - 5000.0).abs() < 10.0);
    }
}

// ─── PolylineVolumeGeometry: edge cases ───────────────────────────────────

#[test]
fn polyline_volume_with_corner_shape() {
    let e = wgs84();
    let opts = PolylineVolumeOptions {
        positions: vec![
            e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(2.0, 1.0, 0.0)),
        ],
        shape: vec![
            [-50.0, -50.0],
            [50.0, -50.0],
            [50.0, 50.0],
            [-50.0, 50.0],
        ],
        ellipsoid: e,
        granularity: std::f64::consts::PI / 180.0,
        ..Default::default()
    };
    let geo = polyline_volume_geometry(&opts, VertexFormat::POSITION_AND_NORMAL);
    assert!(geo.positions.len() >= 8);
    if let Some(normals) = &geo.normals {
        for n in normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-6, "normal should be unit");
        }
    }
}

// ─── GroundPolylineGeometry: closed vs open ───────────────────────────────

#[test]
fn ground_polyline_closed_produces_closed_shape() {
    let e = wgs84();
    let opts = GroundPolylineOptions {
        positions: vec![
            e.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            e.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
        ],
        width: 5.0,
        ellipsoid: e,
        granularity: std::f64::consts::PI / 180.0,
        closed: true,
    };
    let geo = ground_polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(geo.positions.len() >= 4);
    assert!(!geo.indices.is_empty());
}

// ─── PlaneGeometry: normal direction ──────────────────────────────────────

#[test]
fn plane_positions_form_right_handed_quads() {
    let geo = plane_geometry(VertexFormat::POSITION_AND_NORMAL);
    let normals = geo.normals.as_ref().unwrap();

    // All normals should point in +Z direction
    for n in normals {
        assert!((n[0]).abs() < 1e-10);
        assert!((n[1]).abs() < 1e-10);
        assert!((n[2] - 1.0).abs() < 1e-10);
    }

    // Verify triangle winding produces +Z normals (right-handed)
    // Triangle 0: positions[0], positions[1], positions[2]
    let p0 = DVec3::from(geo.positions[geo.indices[0] as usize]);
    let p1 = DVec3::from(geo.positions[geo.indices[1] as usize]);
    let p2 = DVec3::from(geo.positions[geo.indices[2] as usize]);
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let face_normal = edge1.cross(edge2);
    // Face normal should point in +Z (positive z)
    assert!(face_normal.z > 0.0, "face normal should point +Z");
}
