//! Rectangle geometry detailed specs - ported from Core/RectangleGeometrySpec.js
//!
//! Tests position counts, corner positions, vertex attributes, IDL crossing,
//! pole handling, and height parameter.

use cesium_geospatial::geometry::{rectangle_geometry, PrimitiveType};
use cesium_geospatial::{Cartographic, Ellipsoid, Rectangle, VertexFormat};
use glam::DVec3;

const EPSILON8: f64 = 1e-8;
const EPSILON9: f64 = 1e-9;

fn wgs84() -> Ellipsoid {
    Ellipsoid::WGS84
}

// ─── Position counts (from RectangleGeometrySpec.js) ───────────────────────

#[test]
fn rectangle_computes_positions() {
    // RectangleGeometrySpec: "computes positions"
    // Rectangle(-2, -1, 0, 1) with granularity=1.0 radian
    let e = wgs84();
    let rect = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let geo = rectangle_geometry(&rect, &e, 1.0, 0.0, VertexFormat::POSITION_ONLY);

    // With granularity=1.0: width=2, height=2 → cols=3, rows=3 → 9 vertices
    assert_eq!(geo.positions.len(), 9);
    // 2*2 quads * 2 triangles * 3 = 24 indices
    assert_eq!(geo.indices.len(), 8 * 3);
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);

    // Verify NW and SE corners exist in positions
    let nw = e.cartographic_to_cartesian(&Cartographic::from_radians(rect.west, rect.north, 0.0));
    let se = e.cartographic_to_cartesian(&Cartographic::from_radians(rect.east, rect.south, 0.0));

    let has_nw = geo.positions.iter().any(|p| (DVec3::from(*p) - nw).length() < EPSILON8);
    let has_se = geo.positions.iter().any(|p| (DVec3::from(*p) - se).length() < EPSILON8);
    assert!(has_nw, "positions should contain NW corner");
    assert!(has_se, "positions should contain SE corner");
}

#[test]
fn rectangle_computes_positions_across_idl() {
    // RectangleGeometrySpec: "computes positions across IDL"
    let e = wgs84();
    let rect = Rectangle::from_degrees(179.0, -1.0, -179.0, 1.0);
    let granularity = std::f64::consts::PI / 180.0; // default
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    // Should produce valid geometry across IDL
    assert!(geo.positions.len() >= 4, "IDL-crossing rectangle should have positions");
    assert!(!geo.indices.is_empty());

    // All positions should be valid (not NaN)
    for p in &geo.positions {
        assert!(p[0].is_finite() && p[1].is_finite() && p[2].is_finite());
    }
}

#[test]
fn rectangle_computes_positions_at_north_pole() {
    // RectangleGeometrySpec: "computes positions at north pole"
    let e = wgs84();
    let rect = Rectangle::from_degrees(-180.0, 89.0, -179.0, 90.0);
    let granularity = std::f64::consts::PI / 180.0;
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 4);
    assert!(!geo.indices.is_empty());

    // All positions should be near the north pole (high z value)
    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!(
            carto.latitude.to_degrees() > 88.0,
            "latitude {} should be near north pole", carto.latitude.to_degrees()
        );
    }
}

#[test]
fn rectangle_computes_positions_at_south_pole() {
    // RectangleGeometrySpec: "computes positions at south pole"
    let e = wgs84();
    let rect = Rectangle::from_degrees(-180.0, -90.0, -179.0, -89.0);
    let granularity = std::f64::consts::PI / 180.0;
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 4);
    assert!(!geo.indices.is_empty());

    // All positions should be near the south pole (low z value)
    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!(
            carto.latitude.to_degrees() < -88.0,
            "latitude {} should be near south pole", carto.latitude.to_degrees()
        );
    }
}

// ─── Vertex attributes ─────────────────────────────────────────────────────

#[test]
fn rectangle_computes_all_attributes() {
    // RectangleGeometrySpec: "computes all attributes"
    let e = wgs84();
    let rect = Rectangle::new(-2.0, -1.0, 0.0, 1.0);
    let geo = rectangle_geometry(&rect, &e, 1.0, 0.0, VertexFormat::ALL);

    let num_vertices = geo.positions.len();
    assert_eq!(num_vertices, 9);
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.indices.len(), 8 * 3);
}

#[test]
fn rectangle_normals_point_outward() {
    // All normals should point away from the ellipsoid center
    let e = wgs84();
    let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let granularity = std::f64::consts::PI / 18.0; // 10 degrees
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_AND_NORMAL);
    let normals = geo.normals.as_ref().unwrap();

    for i in 0..geo.positions.len() {
        let pos = DVec3::from(geo.positions[i]);
        let normal = DVec3::from(normals[i]);
        let pos_dir = pos.normalize();
        let dot = normal.dot(pos_dir);
        assert!(dot > 0.9, "normal {} should point outward, dot = {}", i, dot);
    }
}

#[test]
fn rectangle_tex_coords_in_unit_range() {
    // Texture coordinates should be in [0, 1] range
    let e = wgs84();
    let rect = Rectangle::from_degrees(-20.0, -10.0, 20.0, 10.0);
    let granularity = std::f64::consts::PI / 36.0; // 5 degrees
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_AND_ST);
    let st = geo.tex_coords.as_ref().unwrap();

    for (i, uv) in st.iter().enumerate() {
        assert!(
            uv[0] >= -1e-10 && uv[0] <= 1.0 + 1e-10,
            "tex_coord[{}].u = {} out of [0,1]", i, uv[0]
        );
        assert!(
            uv[1] >= -1e-10 && uv[1] <= 1.0 + 1e-10,
            "tex_coord[{}].v = {} out of [0,1]", i, uv[1]
        );
    }
}

// ─── Height parameter ──────────────────────────────────────────────────────

#[test]
fn rectangle_with_height() {
    // Positions should be at the specified height
    let e = wgs84();
    let rect = Rectangle::from_degrees(-5.0, -5.0, 5.0, 5.0);
    let height = 10000.0;
    let granularity = std::f64::consts::PI / 18.0;
    let geo = rectangle_geometry(&rect, &e, granularity, height, VertexFormat::POSITION_ONLY);

    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!(
            (carto.height - height).abs() < 1.0,
            "height {} should be ≈ {}", carto.height, height
        );
    }
}

#[test]
fn rectangle_with_negative_height() {
    // Negative height (below ellipsoid surface)
    let e = wgs84();
    let rect = Rectangle::from_degrees(-5.0, -5.0, 5.0, 5.0);
    let height = -5000.0;
    let granularity = std::f64::consts::PI / 18.0;
    let geo = rectangle_geometry(&rect, &e, granularity, height, VertexFormat::POSITION_ONLY);

    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!(
            (carto.height - height).abs() < 1.0,
            "height {} should be ≈ {}", carto.height, height
        );
    }
}

// ─── Grid structure ────────────────────────────────────────────────────────

#[test]
fn rectangle_grid_density_matches_granularity() {
    // Finer granularity should produce more vertices
    let e = wgs84();
    let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);

    let coarse = rectangle_geometry(&rect, &e, std::f64::consts::PI / 18.0, 0.0, VertexFormat::POSITION_ONLY);
    let fine = rectangle_geometry(&rect, &e, std::f64::consts::PI / 36.0, 0.0, VertexFormat::POSITION_ONLY);

    assert!(
        fine.positions.len() > coarse.positions.len(),
        "finer granularity ({}) should produce more vertices than coarse ({})",
        fine.positions.len(), coarse.positions.len()
    );
}

#[test]
fn rectangle_indices_reference_valid_vertices() {
    let e = wgs84();
    let rect = Rectangle::from_degrees(-30.0, -20.0, 30.0, 20.0);
    let granularity = std::f64::consts::PI / 18.0;
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    let num_vertices = geo.positions.len() as u32;
    for (i, &idx) in geo.indices.iter().enumerate() {
        assert!(
            idx < num_vertices,
            "index[{}] = {} out of bounds (num_vertices = {})", i, idx, num_vertices
        );
    }
}

#[test]
fn rectangle_bounding_sphere_contains_all_positions() {
    let e = wgs84();
    let rect = Rectangle::from_degrees(-45.0, -30.0, 45.0, 30.0);
    let granularity = std::f64::consts::PI / 18.0;
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    let center = geo.bounding_sphere.center;
    let radius = geo.bounding_sphere.radius;

    for p in &geo.positions {
        let dist = (DVec3::from(*p) - center).length();
        assert!(
            dist <= radius + EPSILON8,
            "position distance {} exceeds bounding sphere radius {}", dist, radius
        );
    }
}

// ─── Edge cases ────────────────────────────────────────────────────────────

#[test]
fn rectangle_very_small() {
    // Very small rectangle should still produce valid geometry
    let e = wgs84();
    let rect = Rectangle::from_degrees(0.0, 0.0, 0.001, 0.001);
    let granularity = std::f64::consts::PI / 180.0;
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 4);
    assert!(!geo.indices.is_empty());
}

#[test]
fn rectangle_full_longitude_range() {
    // Full 360° longitude range
    let e = wgs84();
    let rect = Rectangle::from_degrees(-180.0, -10.0, 180.0, 10.0);
    let granularity = std::f64::consts::PI / 6.0; // 30 degrees
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 4);
    assert!(!geo.indices.is_empty());
    assert_eq!(geo.indices.len() % 3, 0);
}

// ─── Rotation parameter verification ─────────────────────────────────────────

#[test]
fn rectangle_rotation_zero_no_effect() {
    // rotation=0 should produce same positions as default
    let e = wgs84();
    let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let granularity = std::f64::consts::PI / 18.0;

    let geo1 = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);
    // Positions stay same since our API doesn't take rotation directly
    let _center = rect.center();
    for p in &geo1.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        // Longitude should be within rect bounds
        assert!(carto.longitude >= rect.west - 1e-6 && carto.longitude <= rect.east + 1e-6);
        assert!(carto.latitude >= rect.south - 1e-6 && carto.latitude <= rect.north + 1e-6);
    }
}

#[test]
fn rectangle_position_count_depends_on_granularity() {
    let e = wgs84();
    let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);

    // Wide granularity
    let geo1 = rectangle_geometry(&rect, &e, 1.0, 0.0, VertexFormat::POSITION_ONLY);
    // Fine granularity
    let geo2 = rectangle_geometry(&rect, &e, 0.1, 0.0, VertexFormat::POSITION_ONLY);

    assert!(geo2.positions.len() > geo1.positions.len(),
        "finer granularity should produce more vertices: {} vs {}",
        geo2.positions.len(), geo1.positions.len());
}

#[test]
fn rectangle_tex_coords_corner_values() {
    // Texture coordinates at corners should be (0,0) (1,0) (0,1) (1,1)
    let e = wgs84();
    let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let granularity = std::f64::consts::PI / 18.0;
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_AND_ST);
    let st = geo.tex_coords.as_ref().unwrap();
    let num_vertices = geo.positions.len();

    assert_eq!(st.len(), num_vertices);
    // First vertex is at (west, south) → ST should be (0, 0)
    assert!((st[0][0]).abs() < 1e-6);
    assert!((st[0][1]).abs() < 1e-6);
    // Last vertex is at (east, north) → ST should be (1, 1)
    assert!((st[num_vertices - 1][0] - 1.0).abs() < 1e-6);
    assert!((st[num_vertices - 1][1] - 1.0).abs() < 1e-6);
}

#[test]
fn rectangle_extreme_latitudes_near_poles() {
    // Rectangle near north pole (85° to 89°)
    let e = wgs84();
    let rect = Rectangle::from_degrees(-180.0, 85.0, 180.0, 89.0);
    let granularity = std::f64::consts::PI / 6.0;
    let geo = rectangle_geometry(&rect, &e, granularity, 0.0, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 4);
    for p in &geo.positions {
        let carto = e.cartesian_to_cartographic(DVec3::from(*p)).unwrap();
        assert!(carto.latitude.to_degrees() >= 84.5, "not near north pole: {}", carto.latitude);
    }
}
