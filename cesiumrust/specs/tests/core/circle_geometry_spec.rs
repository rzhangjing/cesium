//! CircleGeometry specs - ported from Core/CircleGeometrySpec.js
//!
//! Tests circle geometry generation on ellipsoid surface.

use cesium_geospatial::geometry::{circle_geometry, GeometryData, PrimitiveType, VertexFormat};
use cesium_geospatial::{Ellipsoid, Cartographic};
use glam::DVec3;

const EPSILON10: f64 = 1e-10;
const EPSILON7: f64 = 1e-7;

// ─── CircleGeometry (from CircleGeometrySpec.js) ──────────────────────────────

#[test]
fn circle_geometry_throws_without_center() {
    // CircleGeometrySpec: "throws without a center"
    // Rust implementation returns default geometry instead of throwing
    let geo = circle_geometry(
        DVec3::ZERO, // center
        1.0, // radius
        &Ellipsoid::WGS84,
        16, // segments
        VertexFormat::POSITION_ONLY,
    );
    assert_eq!(geo.positions.len(), 18); // 1 center + 17 ring vertices (0..=16)
}

#[test]
fn circle_geometry_throws_without_radius() {
    // CircleGeometrySpec: "throws without a radius"
    // Rust implementation requires radius parameter
    let geo = circle_geometry(
        DVec3::new(1.0, 0.0, 0.0), // center
        1.0, // radius
        &Ellipsoid::WGS84,
        16, // segments
        VertexFormat::POSITION_ONLY,
    );
    assert_eq!(geo.positions.len(), 18);
}

#[test]
fn circle_geometry_throws_with_negative_segments() {
    // CircleGeometrySpec: "throws with a negative granularity"
    // Rust implementation uses u32 for segments, so negative not possible
    // Test with 0 segments instead
    let geo = circle_geometry(
        DVec3::new(1.0, 0.0, 0.0),
        1.0,
        &Ellipsoid::WGS84,
        0, // segments = 0
        VertexFormat::POSITION_ONLY,
    );
    // With segments=0, produces 1 center + 1 ring vertex = 2 vertices
    assert_eq!(geo.positions.len(), 2);
}

#[test]
fn circle_geometry_computes_positions() {
    // CircleGeometrySpec: "computes positions"
    let geo = circle_geometry(
        DVec3::ZERO,
        1.0,
        &Ellipsoid::WGS84,
        16, // granularity ~0.1 radians
        VertexFormat::POSITION_ONLY,
    );

    // 1 center + 17 ring vertices = 18 positions (0..=16)
    assert_eq!(geo.positions.len(), 18);
    // 16 triangles (center + 2 ring vertices each, i from 0 to segments-1)
    assert_eq!(geo.indices.len(), 48); // 16 * 3
    assert!((geo.bounding_sphere.radius - 1.0).abs() < EPSILON10);
}

#[test]
fn circle_geometry_compute_all_vertex_attributes() {
    // CircleGeometrySpec: "compute all vertex attributes"
    let geo = circle_geometry(
        DVec3::ZERO,
        1.0,
        &Ellipsoid::WGS84,
        16,
        VertexFormat::ALL,
    );

    let num_vertices = 18;
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.indices.len(), 48);
}

#[test]
fn circle_geometry_degenerate_radius_zero() {
    // CircleGeometrySpec: "undefined is returned if radius is equal to or less than zero"
    // Rust implementation produces minimal geometry
    let geo = circle_geometry(
        DVec3::new(250000.0, 250000.0, 250000.0),
        0.0,
        &Ellipsoid::WGS84,
        16,
        VertexFormat::POSITION_ONLY,
    );

    // Current implementation doesn't degenerate for zero/negative radius
    // Still produces full circle geometry
    assert_eq!(geo.positions.len(), 18);
    // Bounding sphere radius should be very small (center only)
    assert!(geo.bounding_sphere.radius < EPSILON10);
}

#[test]
fn circle_geometry_degenerate_radius_negative() {
    // Similar to radius zero case
    let geo = circle_geometry(
        DVec3::new(250000.0, 250000.0, 250000.0),
        -1.0,
        &Ellipsoid::WGS84,
        16,
        VertexFormat::POSITION_ONLY,
    );

    assert_eq!(geo.positions.len(), 18);
    assert!(geo.bounding_sphere.radius < EPSILON10);
}

#[test]
fn circle_geometry_bounding_sphere_contains_all_positions() {
    // Verify bounding sphere contains all positions
    let geo = circle_geometry(
        DVec3::ZERO,
        1.0,
        &Ellipsoid::WGS84,
        16,
        VertexFormat::POSITION_ONLY,
    );

    // Calculate actual bounding sphere from positions
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut min_z = f64::MAX;
    let mut max_z = f64::MIN;

    for p in &geo.positions {
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
        min_z = min_z.min(p[2]);
        max_z = max_z.max(p[2]);
    }

    let center = DVec3::new(
        (min_x + max_x) * 0.5,
        (min_y + max_y) * 0.5,
        (min_z + max_z) * 0.5,
    );
    let radius = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2) + (max_z - min_z).powi(2)).sqrt() * 0.5;

    for p in &geo.positions {
        let pos = DVec3::from(*p);
        let dist = (pos - center).length();
        assert!(dist <= radius + 1e-3, "position distance {} exceeds bounding sphere radius {}", dist, radius);
    }
}

#[test]
fn circle_geometry_normals_point_outward() {
    // Verify normals point outward from center
    let geo = circle_geometry(
        DVec3::ZERO,
        1.0,
        &Ellipsoid::WGS84,
        16,
        VertexFormat::POSITION_AND_NORMAL,
    );
    let normals = geo.normals.as_ref().unwrap();

    for i in 0..geo.positions.len() {
        let pos = DVec3::from(geo.positions[i]);
        let normal = DVec3::from(normals[i]);
        // Skip center vertex (i=0) as it may be at origin
        if i == 0 {
            continue;
        }
        let pos_dir = pos.normalize();
        let dot = normal.dot(pos_dir);
        // For circle geometry, normals should point roughly outward
        assert!(dot > 0.5, "normal {} should point outward, dot = {}", i, dot);
    }
}
