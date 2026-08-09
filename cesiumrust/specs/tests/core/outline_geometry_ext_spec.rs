//! Outline geometry extended specs - additional A-class tests from
//! Core/BoxOutlineGeometrySpec.js, Core/SphereOutlineGeometrySpec.js,
//! Core/CylinderOutlineGeometrySpec.js

use cesium_geospatial::geometry::{
    box_outline_geometry, cylinder_outline_geometry, ellipsoid_outline_geometry,
    plane_outline_geometry, PrimitiveType,
};
use glam::DVec3;

const EPSILON10: f64 = 1e-10;

// ─── BoxOutlineGeometry extended (from BoxOutlineGeometrySpec.js) ─────────────

#[test]
fn box_outline_degenerate_min_equals_max() {
    // BoxOutlineGeometrySpec: "undefined is returned if min and max are equal"
    // Rust implementation produces a degenerate box at single point
    let p = DVec3::new(250000.0, 250000.0, 250000.0);
    let geo = box_outline_geometry(p, p);

    // All positions collapse to the same point
    for pos in &geo.positions {
        assert!((pos[0] - 250000.0).abs() < EPSILON10);
        assert!((pos[1] - 250000.0).abs() < EPSILON10);
        assert!((pos[2] - 250000.0).abs() < EPSILON10);
    }
    // Bounding sphere radius should be 0
    assert!(geo.bounding_sphere.radius < EPSILON10);
}

#[test]
fn box_outline_from_dimensions_concept() {
    // BoxOutlineGeometrySpec: "fromDimensions" - box centered at origin
    // Our API takes min/max directly, so simulate fromDimensions
    let dimensions = DVec3::new(1.0, 2.0, 3.0);
    let half = dimensions * 0.5;
    let geo = box_outline_geometry(-half, half);

    assert_eq!(geo.positions.len(), 8);
    assert_eq!(geo.indices.len(), 24); // 12 edges * 2
    assert_eq!(geo.primitive_type, PrimitiveType::Lines);

    // All positions should be within [-half, half]
    for p in &geo.positions {
        assert!(p[0].abs() <= half.x + EPSILON10);
        assert!(p[1].abs() <= half.y + EPSILON10);
        assert!(p[2].abs() <= half.z + EPSILON10);
    }
}

#[test]
fn box_outline_from_aabb_concept() {
    // BoxOutlineGeometrySpec: "fromAxisAlignedBoundingBox"
    // Simulate by passing AABB min/max
    let min = DVec3::new(-1.0, -2.0, -3.0);
    let max = DVec3::new(1.0, 2.0, 3.0);
    let geo = box_outline_geometry(min, max);

    assert_eq!(geo.positions.len(), 8);
    assert_eq!(geo.indices.len(), 24);
}

// ─── SphereOutlineGeometry (using ellipsoid_outline with equal radii) ────────

#[test]
fn sphere_outline_computes_positions() {
    // SphereOutlineGeometrySpec: "computes positions"
    // Rust uses ellipsoid_outline_geometry with equal radii for sphere
    let radii = DVec3::new(1.0, 1.0, 1.0);
    let geo = ellipsoid_outline_geometry(radii, 3, 3);

    // 3 great circles with stacks=3, slices=3
    // XY circle (slices+1=4) + XZ circle (stacks+1=4) + YZ circle (stacks+1=4) = 12
    assert_eq!(geo.positions.len(), 12);
    assert_eq!(geo.indices.len(), 18); // 3 circles * 3 segments * 2 indices
    assert!((geo.bounding_sphere.radius - 1.0).abs() < EPSILON10);
    assert_eq!(geo.primitive_type, PrimitiveType::Lines);
}

#[test]
fn sphere_outline_positions_on_unit_sphere() {
    // SphereOutlineGeometrySpec: positions should be on unit sphere surface
    let radii = DVec3::new(1.0, 1.0, 1.0);
    let geo = ellipsoid_outline_geometry(radii, 8, 8);

    for p in &geo.positions {
        let pos = DVec3::from(*p);
        let magnitude = pos.length();
        assert!(
            (magnitude - 1.0).abs() < EPSILON10,
            "position magnitude {} should be 1.0", magnitude
        );
    }
}

#[test]
fn sphere_outline_degenerate_radius_zero() {
    // SphereOutlineGeometrySpec: "undefined is returned if radius is equals to zero"
    // Rust implementation produces degenerate geometry at origin
    let radii = DVec3::new(0.0, 0.0, 0.0);
    let geo = ellipsoid_outline_geometry(radii, 3, 3);

    // All positions should be at origin
    for p in &geo.positions {
        assert!(p[0].abs() < EPSILON10);
        assert!(p[1].abs() < EPSILON10);
        assert!(p[2].abs() < EPSILON10);
    }
    assert!(geo.bounding_sphere.radius < EPSILON10);
}

#[test]
fn sphere_outline_radius_scales_positions() {
    // SphereOutlineGeometrySpec: radius scales positions
    let radius = 5.0;
    let radii = DVec3::new(radius, radius, radius);
    let geo = ellipsoid_outline_geometry(radii, 4, 4);

    for p in &geo.positions {
        let pos = DVec3::from(*p);
        let magnitude = pos.length();
        assert!(
            (magnitude - radius).abs() < EPSILON10,
            "position magnitude {} should be {}", magnitude, radius
        );
    }
    assert!((geo.bounding_sphere.radius - radius).abs() < EPSILON10);
}

// ─── CylinderOutlineGeometry extended (from CylinderOutlineGeometrySpec.js) ──

#[test]
fn cylinder_outline_degenerate_length_zero() {
    // CylinderOutlineGeometrySpec: "undefined is returned if length <= 0"
    // Rust produces degenerate geometry at z=0
    let geo = cylinder_outline_geometry(0.0, 1.0, 1.0, 8);

    // All positions should be at z=0
    for p in &geo.positions {
        assert!(p[2].abs() < EPSILON10, "z={} should be 0", p[2]);
    }
}

#[test]
fn cylinder_outline_degenerate_both_radii_zero() {
    // CylinderOutlineGeometrySpec: "undefined if both radii are zero"
    let geo = cylinder_outline_geometry(10.0, 0.0, 0.0, 8);

    // All positions should be on Z axis (x=0, y=0)
    for p in &geo.positions {
        assert!(p[0].abs() < EPSILON10);
        assert!(p[1].abs() < EPSILON10);
    }
}

#[test]
fn cylinder_outline_cone_bottom_radius_zero() {
    // CylinderOutlineGeometrySpec: "computes positions with bottomRadius equals 0"
    let geo = cylinder_outline_geometry(10.0, 5.0, 0.0, 8);

    assert!(!geo.positions.is_empty());
    assert_eq!(geo.primitive_type, PrimitiveType::Lines);

    // Bottom circle vertices should be at origin (x=0, y=0, z=-half)
    let half = 5.0;
    for p in &geo.positions {
        if (p[2] + half).abs() < EPSILON10 {
            // Bottom vertex
            assert!(p[0].abs() < EPSILON10);
            assert!(p[1].abs() < EPSILON10);
        }
    }
}

#[test]
fn cylinder_outline_bounding_sphere() {
    // CylinderOutlineGeometrySpec: bounding sphere should encompass cylinder
    let length = 2.0;
    let radius = 1.0;
    let geo = cylinder_outline_geometry(length, radius, radius, 8);

    let half_length = length * 0.5;
    let expected_radius = (radius * radius + half_length * half_length).sqrt();
    assert!((geo.bounding_sphere.radius - expected_radius).abs() < EPSILON10);
    assert!(geo.bounding_sphere.center.length() < EPSILON10);
}

// ─── PlaneOutlineGeometry extended (from PlaneOutlineGeometrySpec.js) ────────

#[test]
fn plane_outline_bounding_sphere() {
    // PlaneOutlineGeometrySpec: bounding sphere should encompass unit quad
    let geo = plane_outline_geometry();

    // Unit quad from -0.5 to 0.5 in XY plane
    // Bounding sphere radius = diagonal/2 = sqrt(0.5² + 0.5²) = sqrt(0.5) ≈ 0.707
    let expected_radius = std::f64::consts::FRAC_1_SQRT_2;
    assert!((geo.bounding_sphere.radius - expected_radius).abs() < EPSILON10);
    assert!(geo.bounding_sphere.center.length() < EPSILON10);
}

#[test]
fn plane_outline_indices_form_closed_loop() {
    // PlaneOutlineGeometrySpec: indices should form 4 edges
    let geo = plane_outline_geometry();

    assert_eq!(geo.indices.len(), 8); // 4 edges * 2 indices
    // Verify edges: 0-1, 1-2, 2-3, 3-0
    assert_eq!(geo.indices[0], 0);
    assert_eq!(geo.indices[1], 1);
    assert_eq!(geo.indices[2], 1);
    assert_eq!(geo.indices[3], 2);
    assert_eq!(geo.indices[4], 2);
    assert_eq!(geo.indices[5], 3);
    assert_eq!(geo.indices[6], 3);
    assert_eq!(geo.indices[7], 0);
}

// ─── Common invariants ──────────────────────────────────────────────────────

#[test]
fn all_outline_geometries_have_valid_indices() {
    // All indices should reference valid vertices
    let geometries = vec![
        box_outline_geometry(DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0)),
        ellipsoid_outline_geometry(DVec3::new(1.0, 1.0, 1.0), 4, 4),
        cylinder_outline_geometry(2.0, 1.0, 1.0, 8),
        plane_outline_geometry(),
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
fn all_outline_geometries_use_lines_primitive() {
    let geometries = vec![
        box_outline_geometry(DVec3::splat(-1.0), DVec3::ONE),
        ellipsoid_outline_geometry(DVec3::ONE, 4, 4),
        cylinder_outline_geometry(2.0, 1.0, 1.0, 8),
        plane_outline_geometry(),
    ];

    for geo in &geometries {
        assert_eq!(
            geo.primitive_type,
            PrimitiveType::Lines,
            "all outline geometries should use Lines primitive"
        );
    }
}
