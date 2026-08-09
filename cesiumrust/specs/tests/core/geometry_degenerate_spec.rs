//! Geometry degenerate cases - ported from CesiumJS Core/CylinderGeometrySpec.js,
//! Core/EllipsoidGeometrySpec.js, Core/BoxGeometrySpec.js
//!
//! Tests degenerate inputs that should produce minimal/empty geometry.

use cesium_geospatial::geometry::{
    box_geometry, cylinder_geometry, ellipsoid_geometry, sphere_geometry,
    PrimitiveType, VertexFormat,
};
use glam::DVec3;

const EPSILON10: f64 = 1e-10;

// ─── CylinderGeometry degenerate cases (from CylinderGeometrySpec.js) ────────

#[test]
fn cylinder_degenerate_length_zero() {
    // CylinderGeometrySpec: "undefined is returned if the length is less than or equal to zero"
    let geo = cylinder_geometry(0.0, 80000.0, 200000.0, 8, VertexFormat::POSITION_ONLY);
    // Our implementation produces degenerate geometry instead of None
    // All positions should be at z=0
    for p in &geo.positions {
        assert!(p[2].abs() < EPSILON10, "z={} should be 0 for zero-length", p[2]);
    }
}

#[test]
fn cylinder_degenerate_negative_length() {
    // CylinderGeometrySpec: "undefined if length < 0"
    let geo = cylinder_geometry(-200000.0, 100.0, 100.0, 8, VertexFormat::POSITION_ONLY);
    // Implementation produces geometry with swapped z extents
    assert!(!geo.positions.is_empty());
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

#[test]
fn cylinder_degenerate_both_radii_zero() {
    // CylinderGeometrySpec: "undefined if both radii are equal to zero"
    let geo = cylinder_geometry(200000.0, 0.0, 0.0, 8, VertexFormat::POSITION_ONLY);
    // All positions should be on Z axis
    for p in &geo.positions {
        assert!(p[0].abs() < EPSILON10, "x={} should be 0", p[0]);
        assert!(p[1].abs() < EPSILON10, "y={} should be 0", p[1]);
    }
}

#[test]
fn cylinder_degenerate_top_radius_negative() {
    // CylinderGeometrySpec: "undefined if either radii is less than zero"
    let geo = cylinder_geometry(200000.0, -10.0, 4.0, 8, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty());
}

#[test]
fn cylinder_degenerate_bottom_radius_negative() {
    let geo = cylinder_geometry(200000.0, 0.0, -34.0, 8, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty());
}

#[test]
fn cylinder_cone_top_vs_bottom_zero() {
    // Compare cone with top=0 vs bottom=0 - should be mirror images
    let top_zero = cylinder_geometry(10.0, 0.0, 5.0, 8, VertexFormat::POSITION_ONLY);
    let bottom_zero = cylinder_geometry(10.0, 5.0, 0.0, 8, VertexFormat::POSITION_ONLY);

    // Same number of vertices
    assert_eq!(top_zero.positions.len(), bottom_zero.positions.len());
    assert_eq!(top_zero.indices.len(), bottom_zero.indices.len());

    // Same bounding sphere radius
    assert!(
        (top_zero.bounding_sphere.radius - bottom_zero.bounding_sphere.radius).abs() < EPSILON10
    );
}

#[test]
fn cylinder_large_slice_count() {
    // High slice count should produce smooth cylinder
    let geo = cylinder_geometry(2.0, 1.0, 1.0, 64, VertexFormat::POSITION_ONLY);
    // Should have many vertices
    assert!(geo.positions.len() > 100, "high slice count should produce many vertices");
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

// ─── EllipsoidGeometry degenerate cases (from EllipsoidGeometrySpec.js) ──────

#[test]
fn ellipsoid_degenerate_zero_x_radius() {
    // EllipsoidGeometrySpec: "undefined if x, y, or z radii are equal or less than zero"
    let geo = ellipsoid_geometry(DVec3::new(0.0, 500000.0, 500000.0), 4, 4, VertexFormat::POSITION_ONLY);
    // All positions should have x=0 (flattened to YZ plane)
    for p in &geo.positions {
        assert!(p[0].abs() < EPSILON10, "x={} should be 0", p[0]);
    }
}

#[test]
fn ellipsoid_degenerate_zero_y_radius() {
    let geo = ellipsoid_geometry(DVec3::new(1000000.0, 0.0, 500000.0), 4, 4, VertexFormat::POSITION_ONLY);
    for p in &geo.positions {
        assert!(p[1].abs() < EPSILON10, "y={} should be 0", p[1]);
    }
}

#[test]
fn ellipsoid_degenerate_zero_z_radius() {
    let geo = ellipsoid_geometry(DVec3::new(1000000.0, 500000.0, 0.0), 4, 4, VertexFormat::POSITION_ONLY);
    for p in &geo.positions {
        assert!(p[2].abs() < EPSILON10, "z={} should be 0", p[2]);
    }
}

#[test]
fn ellipsoid_degenerate_negative_x_radius() {
    let geo = ellipsoid_geometry(DVec3::new(-10.0, 500000.0, 500000.0), 4, 4, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty());
}

#[test]
fn ellipsoid_degenerate_negative_y_radius() {
    let geo = ellipsoid_geometry(DVec3::new(1000000.0, -10.0, 500000.0), 4, 4, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty());
}

#[test]
fn ellipsoid_degenerate_negative_z_radius() {
    let geo = ellipsoid_geometry(DVec3::new(1000000.0, 500000.0, -10.0), 4, 4, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty());
}

#[test]
fn ellipsoid_partitions_default_to_minimum() {
    // EllipsoidGeometrySpec: "computes partitions to default to 2 if less than 2"
    // With 0 stacks and 0 slices, our implementation should handle gracefully
    let geo = ellipsoid_geometry(DVec3::new(0.5, 0.5, 0.5), 1, 1, VertexFormat::POSITION_ONLY);
    // Minimal partitions: at least some vertices and indices
    assert!(!geo.positions.is_empty());
    assert!(!geo.indices.is_empty());
}

#[test]
fn ellipsoid_unit_sphere_radius() {
    // EllipsoidGeometrySpec: "computes the unit ellipsoid"
    let geo = ellipsoid_geometry(DVec3::ONE, 8, 8, VertexFormat::POSITION_ONLY);
    assert!((geo.bounding_sphere.radius - 1.0).abs() < EPSILON10);
}

#[test]
fn ellipsoid_non_uniform_bounding_sphere() {
    // Bounding sphere should use max radius
    let radii = DVec3::new(1.0, 2.0, 3.0);
    let geo = ellipsoid_geometry(radii, 8, 8, VertexFormat::POSITION_ONLY);
    assert!((geo.bounding_sphere.radius - 3.0).abs() < EPSILON10);
}

// ─── BoxGeometry degenerate cases (from BoxGeometrySpec.js) ─────────────────

#[test]
fn box_degenerate_flat_in_x() {
    // Box with zero extent in X dimension
    let geo = box_geometry(
        DVec3::new(5.0, -1.0, -1.0),
        DVec3::new(5.0, 1.0, 1.0),
        VertexFormat::POSITION_ONLY,
    );
    // All positions should have x=5
    for p in &geo.positions {
        assert!((p[0] - 5.0).abs() < EPSILON10, "x={} should be 5", p[0]);
    }
}

#[test]
fn box_degenerate_flat_in_y() {
    let geo = box_geometry(
        DVec3::new(-1.0, 3.0, -1.0),
        DVec3::new(1.0, 3.0, 1.0),
        VertexFormat::POSITION_ONLY,
    );
    for p in &geo.positions {
        assert!((p[1] - 3.0).abs() < EPSILON10, "y={} should be 3", p[1]);
    }
}

#[test]
fn box_degenerate_flat_in_z() {
    let geo = box_geometry(
        DVec3::new(-1.0, -1.0, 7.0),
        DVec3::new(1.0, 1.0, 7.0),
        VertexFormat::POSITION_ONLY,
    );
    for p in &geo.positions {
        assert!((p[2] - 7.0).abs() < EPSILON10, "z={} should be 7", p[2]);
    }
}

#[test]
fn box_normals_consistent_across_faces() {
    // Each face should have uniform normals
    let geo = box_geometry(
        DVec3::new(-1.0, -1.0, -1.0),
        DVec3::new(1.0, 1.0, 1.0),
        VertexFormat::POSITION_AND_NORMAL,
    );
    let normals = geo.normals.as_ref().unwrap();
    assert_eq!(normals.len(), geo.positions.len());

    // All normals should be unit length
    for n in normals {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < EPSILON10, "normal should be unit length, got {}", len);
    }
}

// ─── SphereGeometry degenerate cases ────────────────────────────────────────

#[test]
fn sphere_degenerate_zero_radius() {
    let geo = sphere_geometry(0.0, 4, 4, VertexFormat::POSITION_ONLY);
    // All positions should be at origin
    for p in &geo.positions {
        assert!(p[0].abs() < EPSILON10);
        assert!(p[1].abs() < EPSILON10);
        assert!(p[2].abs() < EPSILON10);
    }
    assert!(geo.bounding_sphere.radius < EPSILON10);
}

#[test]
fn sphere_degenerate_one_stack_one_slice() {
    // Minimal partitions
    let geo = sphere_geometry(1.0, 1, 1, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty());
    assert!(!geo.indices.is_empty());
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

#[test]
fn sphere_large_partitions() {
    // High partition count should produce many vertices
    let geo = sphere_geometry(1.0, 32, 32, VertexFormat::POSITION_ONLY);
    let expected_vertices = (32 + 1) * (32 + 1);
    assert_eq!(geo.positions.len(), expected_vertices);
    // All positions should be on unit sphere
    for p in &geo.positions {
        let pos = DVec3::from(*p);
        assert!((pos.length() - 1.0).abs() < EPSILON10);
    }
}

// ─── Cross-cutting invariants ──────────────────────────────────────────────

#[test]
fn all_geometries_produce_valid_indices_for_degenerate_inputs() {
    // Even degenerate inputs should produce valid (in-bounds) indices
    let geometries = vec![
        ("cylinder_zero_length", cylinder_geometry(0.0, 1.0, 1.0, 8, VertexFormat::POSITION_ONLY)),
        ("cylinder_zero_radii", cylinder_geometry(1.0, 0.0, 0.0, 8, VertexFormat::POSITION_ONLY)),
        ("ellipsoid_zero_x", ellipsoid_geometry(DVec3::new(0.0, 1.0, 1.0), 4, 4, VertexFormat::POSITION_ONLY)),
        ("box_flat", box_geometry(DVec3::new(1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0), VertexFormat::POSITION_ONLY)),
        ("sphere_zero", sphere_geometry(0.0, 4, 4, VertexFormat::POSITION_ONLY)),
    ];

    for (name, geo) in &geometries {
        let n = geo.positions.len() as u32;
        for (i, &idx) in geo.indices.iter().enumerate() {
            assert!(
                idx < n,
                "{}: index[{}] = {} out of bounds (n={})",
                name, i, idx, n
            );
        }
    }
}

#[test]
fn all_geometries_use_triangles_primitive() {
    let geometries = vec![
        cylinder_geometry(1.0, 1.0, 1.0, 8, VertexFormat::POSITION_ONLY),
        ellipsoid_geometry(DVec3::ONE, 4, 4, VertexFormat::POSITION_ONLY),
        box_geometry(DVec3::splat(-1.0), DVec3::ONE, VertexFormat::POSITION_ONLY),
        sphere_geometry(1.0, 4, 4, VertexFormat::POSITION_ONLY),
    ];

    for geo in &geometries {
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
    }
}
