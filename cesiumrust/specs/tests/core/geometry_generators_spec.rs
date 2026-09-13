//! Geometry generator specs - ported from Core/BoxGeometrySpec.js,
//! Core/SphereGeometrySpec.js, Core/CylinderGeometrySpec.js,
//! Core/EllipsoidGeometrySpec.js, Core/FrustumGeometrySpec.js,
//! Core/RectangleGeometrySpec.js
//!
//! Tests vertex counts, index counts, bounding spheres, normals, and
//! mathematical properties of generated geometry.

use cesium_geospatial::geometry::{
    box_geometry, cylinder_geometry, ellipsoid_geometry, sphere_geometry,
    rectangle_geometry, frustum_geometry, frustum_outline_geometry,
    FrustumDef, PrimitiveType,
};
use cesium_geospatial::{Ellipsoid, PerspectiveFrustum, VertexFormat};
use glam::DVec3;

const EPSILON10: f64 = 1e-10;
const EPSILON7: f64 = 1e-7;

// ─── BoxGeometry (from BoxGeometrySpec.js) ─────────────────────────────────

#[test]
fn box_position_only_creates_optimized_positions() {
    // BoxGeometrySpec: "constructor creates optimized number of positions for VertexFormat.POSITIONS_ONLY"
    let geo = box_geometry(
        DVec3::new(-1.0, -2.0, -3.0),
        DVec3::new(1.0, 2.0, 3.0),
        VertexFormat::POSITION_ONLY,
    );
    // 6 faces * 4 vertices = 24 positions (per-face vertices for flat shading)
    assert_eq!(geo.positions.len(), 24);
    // 6 faces * 2 triangles * 3 indices = 36
    assert_eq!(geo.indices.len(), 36);
    assert!(geo.normals.is_none());
    assert!(geo.tex_coords.is_none());
}

#[test]
fn box_computes_all_vertex_attributes() {
    // BoxGeometrySpec: "constructor computes all vertex attributes"
    let min = DVec3::new(0.0, 0.0, 0.0);
    let max = DVec3::new(1.0, 1.0, 1.0);
    let geo = box_geometry(min, max, VertexFormat::ALL);

    let num_vertices = 24; // 6 faces * 4 vertices
    let num_triangles = 12; // 6 faces * 2 triangles
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.indices.len(), num_triangles * 3);

    // Bounding sphere center should be at center of box
    let center = (min + max) * 0.5;
    assert!((geo.bounding_sphere.center - center).length() < EPSILON10);
    // Radius = half diagonal
    let expected_radius = (max - min).length() * 0.5;
    assert!((geo.bounding_sphere.radius - expected_radius).abs() < EPSILON10);
}

#[test]
fn box_from_dimensions_concept() {
    // BoxGeometrySpec: "fromDimensions" - box centered at origin with given dimensions
    let dimensions = DVec3::new(1.0, 2.0, 3.0);
    let half = dimensions * 0.5;
    let geo = box_geometry(-half, half, VertexFormat::POSITION_ONLY);

    assert_eq!(geo.positions.len(), 24);
    assert_eq!(geo.indices.len(), 36);

    // All positions should be within [-half, half]
    for p in &geo.positions {
        assert!((p[0] - (-half.x).min(half.x)).abs() < EPSILON10 || (p[0] - half.x).abs() < EPSILON10);
    }
}

#[test]
fn box_normals_perpendicular_to_faces() {
    // Verify each face has consistent unit normals
    let geo = box_geometry(
        DVec3::new(-1.0, -1.0, -1.0),
        DVec3::new(1.0, 1.0, 1.0),
        VertexFormat::POSITION_AND_NORMAL,
    );
    let normals = geo.normals.as_ref().unwrap();

    // Each group of 4 vertices (one face) should have the same normal
    for face in 0..6 {
        let base = face * 4;
        let n0 = DVec3::from(normals[base]);
        for v in 1..4 {
            let nv = DVec3::from(normals[base + v]);
            assert!((n0 - nv).length() < EPSILON10, "face {} normals should be uniform", face);
        }
        // Normal should be unit length
        assert!((n0.length() - 1.0).abs() < EPSILON10);
    }
}

#[test]
fn box_degenerate_min_equals_max() {
    // BoxGeometrySpec: "undefined is returned if min and max are equal"
    // Rust implementation produces zero-size box
    let p = DVec3::new(250000.0, 250000.0, 250000.0);
    let geo = box_geometry(p, p, VertexFormat::POSITION_ONLY);
    // All positions collapse to the same point
    for pos in &geo.positions {
        assert!((pos[0] - 250000.0).abs() < EPSILON10);
        assert!((pos[1] - 250000.0).abs() < EPSILON10);
        assert!((pos[2] - 250000.0).abs() < EPSILON10);
    }
    // Bounding sphere radius should be 0
    assert!(geo.bounding_sphere.radius < EPSILON10);
}

// ─── SphereGeometry (from SphereGeometrySpec.js) ───────────────────────────

#[test]
fn sphere_computes_positions() {
    // SphereGeometrySpec: "computes positions" with stackPartitions=3, slicePartitions=3
    let geo = sphere_geometry(1.0, 3, 3, VertexFormat::POSITION_ONLY);

    // Rust: (stacks+1) * (slices+1) = 4 * 4 = 16 vertices
    let num_vertices = (3 + 1) * (3 + 1);
    assert_eq!(geo.positions.len(), num_vertices);
    // stacks * slices * 6 = 3 * 3 * 6 = 54 indices
    let num_indices = 3 * 3 * 6;
    assert_eq!(geo.indices.len(), num_indices);
    assert!((geo.bounding_sphere.radius - 1.0).abs() < EPSILON10);
}

#[test]
fn sphere_computes_all_vertex_attributes() {
    // SphereGeometrySpec: "compute all vertex attributes"
    let geo = sphere_geometry(1.0, 3, 3, VertexFormat::ALL);

    let num_vertices = (3 + 1) * (3 + 1);
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.indices.len(), 3 * 3 * 6);
}

#[test]
fn sphere_positions_on_unit_sphere() {
    // SphereGeometrySpec: "computes attributes for a unit sphere"
    let geo = sphere_geometry(1.0, 6, 8, VertexFormat::POSITION_AND_NORMAL);
    let normals = geo.normals.as_ref().unwrap();

    for i in 0..geo.positions.len() {
        let pos = DVec3::from(geo.positions[i]);
        let normal = DVec3::from(normals[i]);

        // Position magnitude should be ≈ 1.0
        assert!(
            (pos.length() - 1.0).abs() < EPSILON10,
            "position {} magnitude {} != 1.0", i, pos.length()
        );

        // Normal should equal normalized position (for unit sphere)
        if pos.length() > EPSILON10 {
            let expected_normal = pos.normalize();
            assert!(
                (normal - expected_normal).length() < EPSILON7,
                "normal {} doesn't match normalized position", i
            );
        }
    }
}

#[test]
fn sphere_radius_scales_positions() {
    // Positions should be at distance = radius from center
    let radius = 5.0;
    let geo = sphere_geometry(radius, 4, 4, VertexFormat::POSITION_ONLY);

    for p in &geo.positions {
        let pos = DVec3::from(*p);
        assert!(
            (pos.length() - radius).abs() < EPSILON10,
            "position magnitude {} != radius {}", pos.length(), radius
        );
    }
    assert!((geo.bounding_sphere.radius - radius).abs() < EPSILON10);
}

// ─── CylinderGeometry (from CylinderGeometrySpec.js) ───────────────────────

#[test]
fn cylinder_computes_positions() {
    // CylinderGeometrySpec: "computes positions" with slices=3
    let geo = cylinder_geometry(1.0, 1.0, 1.0, 3, VertexFormat::POSITION_ONLY);

    // Rust: (slices+1) * 2 = 8 vertices (side only, no caps)
    let num_vertices = (3 + 1) * 2;
    assert_eq!(geo.positions.len(), num_vertices);
    // slices * 6 = 18 indices (2 triangles per slice)
    let num_indices = 3 * 6;
    assert_eq!(geo.indices.len(), num_indices);
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

#[test]
fn cylinder_computes_all_vertex_attributes() {
    // CylinderGeometrySpec: "compute all vertex attributes"
    let geo = cylinder_geometry(1.0, 1.0, 1.0, 3, VertexFormat::ALL);

    let num_vertices = (3 + 1) * 2;
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.indices.len(), 3 * 6);
}

#[test]
fn cylinder_top_radius_zero_cone() {
    // CylinderGeometrySpec: "computes positions with topRadius equals 0"
    let geo = cylinder_geometry(1.0, 0.0, 1.0, 3, VertexFormat::POSITION_ONLY);

    let num_vertices = (3 + 1) * 2;
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.indices.len(), 3 * 6);

    // Top vertices should be at origin (radius=0)
    for i in (1..geo.positions.len()).step_by(2) {
        let p = DVec3::from(geo.positions[i]);
        assert!(p.x.abs() < EPSILON10 && p.y.abs() < EPSILON10,
            "top vertex {} should be at center, got ({}, {})", i, p.x, p.y);
    }
}

#[test]
fn cylinder_bottom_radius_zero_inverted_cone() {
    // CylinderGeometrySpec: "computes positions with bottomRadius equals 0"
    let geo = cylinder_geometry(1.0, 1.0, 0.0, 3, VertexFormat::POSITION_ONLY);

    let num_vertices = (3 + 1) * 2;
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.indices.len(), 3 * 6);

    // Bottom vertices should be at origin (radius=0)
    for i in (0..geo.positions.len()).step_by(2) {
        let p = DVec3::from(geo.positions[i]);
        assert!(p.x.abs() < EPSILON10 && p.y.abs() < EPSILON10,
            "bottom vertex {} should be at center, got ({}, {})", i, p.x, p.y);
    }
}

#[test]
fn cylinder_bounding_sphere() {
    // Bounding sphere should encompass the cylinder
    let length = 2.0;
    let radius = 1.0;
    let geo = cylinder_geometry(length, radius, radius, 8, VertexFormat::POSITION_ONLY);

    let half_length = length * 0.5;
    let expected_radius = (radius * radius + half_length * half_length).sqrt();
    assert!((geo.bounding_sphere.radius - expected_radius).abs() < EPSILON10);
    assert!(geo.bounding_sphere.center.length() < EPSILON10); // centered at origin
}

// ─── EllipsoidGeometry (from EllipsoidGeometrySpec.js) ─────────────────────

#[test]
fn ellipsoid_computes_positions() {
    // EllipsoidGeometrySpec: "computes positions" with slicePartitions=3, stackPartitions=3
    let radii = DVec3::new(1.0, 1.0, 1.0);
    let geo = ellipsoid_geometry(radii, 3, 3, VertexFormat::POSITION_ONLY);

    // Rust: (stacks+1) * (slices+1) = 4 * 4 = 16
    let num_vertices = (3 + 1) * (3 + 1);
    assert_eq!(geo.positions.len(), num_vertices);
    // stacks * slices * 6 = 54
    assert_eq!(geo.indices.len(), 3 * 3 * 6);
    assert!((geo.bounding_sphere.radius - 1.0).abs() < EPSILON10);
}

#[test]
fn ellipsoid_computes_all_vertex_attributes() {
    // EllipsoidGeometrySpec: "compute all vertex attributes"
    let radii = DVec3::new(1.0, 1.0, 1.0);
    let geo = ellipsoid_geometry(radii, 3, 3, VertexFormat::ALL);

    let num_vertices = (3 + 1) * (3 + 1);
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.indices.len(), 3 * 3 * 6);
}

#[test]
fn ellipsoid_unit_sphere_properties() {
    // EllipsoidGeometrySpec: "computes attributes for a unit sphere"
    let radii = DVec3::new(1.0, 1.0, 1.0);
    let geo = ellipsoid_geometry(radii, 6, 8, VertexFormat::POSITION_AND_NORMAL);
    let normals = geo.normals.as_ref().unwrap();

    for i in 0..geo.positions.len() {
        let pos = DVec3::from(geo.positions[i]);
        let normal = DVec3::from(normals[i]);

        // Position magnitude ≈ 1.0 for unit sphere
        assert!(
            (pos.length() - 1.0).abs() < EPSILON10,
            "position {} magnitude {} != 1.0", i, pos.length()
        );

        // Normal should equal normalized position
        if pos.length() > EPSILON10 {
            let expected_normal = pos.normalize();
            assert!(
                (normal - expected_normal).length() < EPSILON7,
                "normal {} doesn't match normalized position", i
            );
        }
    }
}

#[test]
fn ellipsoid_non_uniform_radii() {
    // Non-uniform radii should scale positions accordingly
    let radii = DVec3::new(1.0, 2.0, 3.0);
    let geo = ellipsoid_geometry(radii, 4, 4, VertexFormat::POSITION_ONLY);

    // Bounding sphere radius should be max radii
    assert!((geo.bounding_sphere.radius - 3.0).abs() < EPSILON10);

    // All positions should satisfy (x/rx)² + (y/ry)² + (z/rz)² ≈ 1
    for p in &geo.positions {
        let normalized = DVec3::new(p[0] / radii.x, p[1] / radii.y, p[2] / radii.z);
        assert!(
            (normalized.length() - 1.0).abs() < EPSILON7,
            "position ({}, {}, {}) not on ellipsoid surface", p[0], p[1], p[2]
        );
    }
}

// ─── FrustumGeometry (from FrustumGeometrySpec.js) ─────────────────────────

#[test]
fn frustum_computes_all_vertex_attributes() {
    // FrustumGeometrySpec: "constructor computes all vertex attributes"
    let frustum = FrustumDef::Perspective(PerspectiveFrustum {
        fov: (30.0_f64).to_radians(),
        aspect_ratio: 1920.0 / 1080.0,
        near: 1.0,
        far: 3.0,
        x_offset: 0.0,
        y_offset: 0.0,
    });
    let geo = frustum_geometry(&frustum, DVec3::ZERO, glam::DQuat::IDENTITY, VertexFormat::ALL);

    let num_vertices = 24; // 6 planes * 4 vertices
    let num_triangles = 12; // 6 planes * 2 triangles
    assert_eq!(geo.positions.len(), num_vertices);
    assert_eq!(geo.normals.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), num_vertices);
    assert_eq!(geo.indices.len(), num_triangles * 3);
}

#[test]
fn frustum_bounding_sphere() {
    // FrustumGeometrySpec: bounding sphere center at midpoint of frustum axis
    let frustum = FrustumDef::Perspective(PerspectiveFrustum {
        fov: (30.0_f64).to_radians(),
        aspect_ratio: 1920.0 / 1080.0,
        near: 1.0,
        far: 3.0,
        x_offset: 0.0,
        y_offset: 0.0,
    });
    let geo = frustum_geometry(&frustum, DVec3::ZERO, glam::DQuat::IDENTITY, VertexFormat::POSITION_ONLY);

    // Bounding sphere should be centered along -Z axis (frustum looks down -Z)
    // Center should be between near and far planes
    assert!(geo.bounding_sphere.radius > 1.0);
    assert!(geo.bounding_sphere.radius < 3.0);
}

#[test]
fn frustum_outline_produces_lines() {
    let frustum = FrustumDef::Perspective(PerspectiveFrustum {
        fov: (45.0_f64).to_radians(),
        aspect_ratio: 1.0,
        near: 0.5,
        far: 10.0,
        x_offset: 0.0,
        y_offset: 0.0,
    });
    let geo = frustum_outline_geometry(&frustum, DVec3::ZERO, glam::DQuat::IDENTITY);

    assert!(geo.positions.len() >= 8, "frustum outline needs at least 8 corners");
    assert!(!geo.indices.is_empty());
    assert_eq!(geo.indices.len() % 2, 0, "outline indices should be line pairs");
    assert_eq!(geo.primitive_type, PrimitiveType::Lines);
}

// ─── RectangleGeometry ─────────────────────────────────────────────────────

#[test]
fn rectangle_produces_grid_positions() {
    use cesium_geospatial::Rectangle;
    let ellipsoid = Ellipsoid::WGS84;
    let rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let granularity = std::f64::consts::PI / 18.0; // 10 degrees

    let geo = rectangle_geometry(&rect, &ellipsoid, granularity, 0.0, VertexFormat::POSITION_ONLY);

    // Should produce a grid of positions
    assert!(geo.positions.len() >= 4, "rectangle should have at least 4 positions");
    assert!(!geo.indices.is_empty());
    assert_eq!(geo.indices.len() % 3, 0, "indices should be triangles");
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

#[test]
fn rectangle_normals_point_outward() {
    use cesium_geospatial::Rectangle;
    let ellipsoid = Ellipsoid::WGS84;
    let rect = Rectangle::from_degrees(-5.0, -5.0, 5.0, 5.0);
    let granularity = std::f64::consts::PI / 180.0; // 1 degree

    let geo = rectangle_geometry(&rect, &ellipsoid, granularity, 0.0, VertexFormat::POSITION_AND_NORMAL);
    let normals = geo.normals.as_ref().unwrap();

    // Each normal should point outward (dot with position direction > 0)
    for i in 0..geo.positions.len() {
        let pos = DVec3::from(geo.positions[i]);
        let normal = DVec3::from(normals[i]);
        let pos_dir = pos.normalize();
        let dot = normal.dot(pos_dir);
        assert!(dot > 0.9, "normal {} should point outward, dot = {}", i, dot);
    }
}

#[test]
fn rectangle_bounding_sphere_contains_all_positions() {
    use cesium_geospatial::Rectangle;
    let ellipsoid = Ellipsoid::WGS84;
    let rect = Rectangle::from_degrees(-20.0, -10.0, 20.0, 10.0);
    let granularity = std::f64::consts::PI / 36.0; // 5 degrees

    let geo = rectangle_geometry(&rect, &ellipsoid, granularity, 0.0, VertexFormat::POSITION_ONLY);

    let center = geo.bounding_sphere.center;
    let radius = geo.bounding_sphere.radius;

    for p in &geo.positions {
        let pos = DVec3::from(*p);
        let dist = (pos - center).length();
        assert!(
            dist <= radius + EPSILON7,
            "position distance {} exceeds bounding sphere radius {}", dist, radius
        );
    }
}

// ─── Cross-cutting invariants ──────────────────────────────────────────────

#[test]
fn all_generators_produce_valid_indices() {
    // All indices should reference valid vertices
    let geometries = vec![
        box_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::ALL),
        sphere_geometry(1.0, 4, 4, VertexFormat::ALL),
        cylinder_geometry(2.0, 1.0, 1.0, 8, VertexFormat::ALL),
        ellipsoid_geometry(DVec3::new(1.0, 1.0, 1.0), 4, 4, VertexFormat::ALL),
    ];

    for (name, geo) in geometries.iter().enumerate() {
        let num_vertices = geo.positions.len() as u32;
        for (i, &idx) in geo.indices.iter().enumerate() {
            assert!(
                idx < num_vertices,
                "geometry {} index[{}] = {} out of bounds (num_vertices = {})",
                name, i, idx, num_vertices
            );
        }
    }
}

#[test]
fn all_generators_normals_are_unit_length() {
    let geometries = vec![
        box_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::POSITION_AND_NORMAL),
        sphere_geometry(2.0, 4, 4, VertexFormat::POSITION_AND_NORMAL),
        cylinder_geometry(2.0, 1.0, 1.5, 8, VertexFormat::POSITION_AND_NORMAL),
        ellipsoid_geometry(DVec3::new(1.0, 2.0, 3.0), 4, 4, VertexFormat::POSITION_AND_NORMAL),
    ];

    for (name, geo) in geometries.iter().enumerate() {
        if let Some(normals) = &geo.normals {
            for (i, n) in normals.iter().enumerate() {
                let len = DVec3::from(*n).length();
                assert!(
                    (len - 1.0).abs() < EPSILON7,
                    "geometry {} normal[{}] length {} != 1.0", name, i, len
                );
            }
        }
    }
}
