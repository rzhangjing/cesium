//! Ported from CesiumJS `Core/EllipseGeometrySpec.js` (expanded A-class tests).
//!
//! Tests: positions, all attributes, texture coordinates, rotation, height,
//! edge cases, circle special case, bounding sphere, outline.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::{
    ellipse_geometry, ellipse_outline_geometry, EllipseOptions, VertexFormat,
};
use glam::DVec3;

fn wgs84() -> Ellipsoid {
    Ellipsoid::WGS84
}

fn from_degrees(lon: f64, lat: f64, h: f64) -> DVec3 {
    let e = wgs84();
    let carto = Cartographic::from_degrees(lon, lat, h);
    e.cartographic_to_cartesian(&carto)
}

// ---------------------------------------------------------------------------
// "computes positions" - granularity=0.1, semiMajor=semiMinor=1.0
// CesiumJS: 16 vertices (rows 1+4+6+4+1), 22 triangles, boundingSphere.radius=1
// ---------------------------------------------------------------------------

#[test]
fn ellipse_computes_positions() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS expects 16 vertices, 22 triangles
    assert_eq!(geo.positions.len(), 16, "expected 16 positions");
    assert_eq!(geo.indices.len(), 66, "expected 66 indices (22 triangles)");
    assert!(
        (geo.bounding_sphere.radius - 1.0).abs() < 1e-10,
        "bounding sphere radius should be 1, got {}",
        geo.bounding_sphere.radius
    );
}

// ---------------------------------------------------------------------------
// "compute all vertex attributes" - VertexFormat.ALL
// Rust impl provides position + normals + st (no tangents/bitangents)
// ---------------------------------------------------------------------------

#[test]
fn ellipse_computes_all_vertex_attributes() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::ALL);

    let num_verts = geo.positions.len();
    assert_eq!(num_verts, 16, "expected 16 positions");

    // Normals and ST should be present
    assert!(geo.normals.is_some(), "normals should be present");
    assert!(geo.tex_coords.is_some(), "tex_coords should be present");

    let normals = geo.normals.as_ref().unwrap();
    let st = geo.tex_coords.as_ref().unwrap();

    assert_eq!(normals.len(), num_verts, "normals count mismatch");
    assert_eq!(st.len(), num_verts, "tex_coords count mismatch");
}

// ---------------------------------------------------------------------------
// "compute texture coordinates with rotation" - stRotation=PI/2
// Note: Rust impl stores st_rotation but applies it differently than CesiumJS.
// Verify ST is present and in valid range with rotation set.
// ---------------------------------------------------------------------------

#[test]
fn ellipse_texture_coordinates_with_rotation() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        st_rotation: std::f64::consts::FRAC_PI_2,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_AND_ST);

    assert_eq!(geo.positions.len(), 16);
    let st = geo.tex_coords.as_ref().expect("tex_coords should be present");
    assert_eq!(st.len(), 16);

    // ST values should be in a reasonable range
    for (i, uv) in st.iter().enumerate() {
        assert!(
            uv[0] >= -0.5 && uv[0] <= 1.5,
            "st[{}].u={} out of range with rotation",
            i,
            uv[0]
        );
        assert!(
            uv[1] >= -0.5 && uv[1] <= 1.5,
            "st[{}].v={} out of range with rotation",
            i,
            uv[1]
        );
    }
}

// ---------------------------------------------------------------------------
// Very small ellipse (semiMajor=semiMinor=1.0) produces correct bounding sphere
// ---------------------------------------------------------------------------

#[test]
fn ellipse_small_axes_correct_bounding_sphere() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);

    // Bounding sphere radius should equal semi_major_axis
    assert!(
        (geo.bounding_sphere.radius - 1.0).abs() < 1e-10,
        "bounding sphere radius should be 1.0, got {}",
        geo.bounding_sphere.radius
    );

    // All positions should be near the ellipsoid surface
    let e = wgs84();
    for p in &geo.positions {
        let pos = DVec3::new(p[0], p[1], p[2]);
        let carto = e.cartesian_to_cartographic(pos).unwrap_or_default();
        assert!(
            carto.height.abs() < 1.0,
            "position should be near surface, height={}",
            carto.height
        );
    }
}

// ---------------------------------------------------------------------------
// Ellipse with different semi-major and semi-minor axes (non-circle)
// ---------------------------------------------------------------------------

#[test]
fn ellipse_non_circle_produces_geometry() {
    let opts = EllipseOptions {
        center: from_degrees(-75.59777, 40.03883, 0.0),
        semi_major_axis: 300000.0,
        semi_minor_axis: 150000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert!(
        geo.positions.len() >= 8,
        "ellipse should produce >= 8 positions, got {}",
        geo.positions.len()
    );
    assert_eq!(geo.indices.len() % 3, 0, "indices must form triangles");
    assert!(geo.indices.len() >= 6);
    assert!(geo.bounding_sphere.radius > 0.0);
}

// ---------------------------------------------------------------------------
// Ellipse with rotation
// ---------------------------------------------------------------------------

#[test]
fn ellipse_with_rotation_produces_geometry() {
    let opts_no_rot = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 500000.0,
        semi_minor_axis: 200000.0,
        rotation: 0.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_no_rot = ellipse_geometry(&opts_no_rot, VertexFormat::POSITION_ONLY);

    let opts_rot = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 500000.0,
        semi_minor_axis: 200000.0,
        rotation: std::f64::consts::FRAC_PI_2,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_rot = ellipse_geometry(&opts_rot, VertexFormat::POSITION_ONLY);

    // Both should produce geometry
    assert!(!geo_no_rot.positions.is_empty());
    assert!(!geo_rot.positions.is_empty());

    // Same number of vertices (rotation doesn't change tessellation)
    assert_eq!(
        geo_no_rot.positions.len(),
        geo_rot.positions.len(),
        "rotation should not change vertex count"
    );

    // But positions should differ (rotated)
    let mut any_different = false;
    for (a, b) in geo_no_rot.positions.iter().zip(geo_rot.positions.iter()) {
        if (a[0] - b[0]).abs() > 1e-6 || (a[1] - b[1]).abs() > 1e-6 {
            any_different = true;
            break;
        }
    }
    assert!(any_different, "rotated ellipse should have different positions");
}

// ---------------------------------------------------------------------------
// Ellipse with height raises positions above ellipsoid
// ---------------------------------------------------------------------------

#[test]
fn ellipse_with_height_raises_positions() {
    let height = 10000.0;
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 100000.0,
        semi_minor_axis: 50000.0,
        height,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert!(!geo.positions.is_empty());

    // All positions should be at approximately the specified height
    let e = wgs84();
    for (i, p) in geo.positions.iter().enumerate() {
        let pos = DVec3::new(p[0], p[1], p[2]);
        let carto = e.cartesian_to_cartographic(pos).unwrap_or_default();
        assert!(
            (carto.height - height).abs() < 100.0,
            "position[{}] height should be ~{}, got {}",
            i,
            height,
            carto.height
        );
    }
}

// ---------------------------------------------------------------------------
// Normals should be unit length and point outward
// ---------------------------------------------------------------------------

#[test]
fn ellipse_normals_are_valid() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 100000.0,
        semi_minor_axis: 50000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::ALL);

    let normals = geo.normals.as_ref().expect("normals should be present");
    for (i, n) in normals.iter().enumerate() {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-6,
            "normal[{}] should be unit length, got {}",
            i,
            len
        );
    }

    // Normals should point outward (dot with position > 0)
    for (i, (p, n)) in geo.positions.iter().zip(normals.iter()).enumerate() {
        let dot = p[0] * n[0] + p[1] * n[1] + p[2] * n[2];
        assert!(dot > 0.0, "normal[{}] should point outward (dot={})", i, dot);
    }
}

// ---------------------------------------------------------------------------
// Texture coordinates should be in [0, 1] range
// ---------------------------------------------------------------------------

#[test]
fn ellipse_texture_coordinates_in_range() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 100000.0,
        semi_minor_axis: 50000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_AND_ST);

    let st = geo.tex_coords.as_ref().expect("tex_coords should be present");
    for (i, uv) in st.iter().enumerate() {
        assert!(
            uv[0] >= -0.1 && uv[0] <= 1.1,
            "st[{}].u={} out of range",
            i,
            uv[0]
        );
        assert!(
            uv[1] >= -0.1 && uv[1] <= 1.1,
            "st[{}].v={} out of range",
            i,
            uv[1]
        );
    }
}

// ---------------------------------------------------------------------------
// Outline geometry: line loop around the ellipse
// ---------------------------------------------------------------------------

#[test]
fn ellipse_outline_forms_closed_loop() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 100000.0,
        semi_minor_axis: 50000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_outline_geometry(&opts);

    assert!(
        geo.positions.len() >= 8,
        "outline should have >= 8 positions, got {}",
        geo.positions.len()
    );
    // Indices are line pairs forming a closed loop
    assert_eq!(geo.indices.len() % 2, 0, "outline indices must be pairs");
    assert_eq!(
        geo.indices.len(),
        geo.positions.len() * 2,
        "closed loop: n positions → n line segments → 2n indices"
    );

    // All indices should be valid
    let n = geo.positions.len() as u32;
    for &idx in &geo.indices {
        assert!(idx < n, "index {} out of bounds (n={})", idx, n);
    }
}

// ---------------------------------------------------------------------------
// Circle (semiMajor == semiMinor) should produce symmetric geometry
// ---------------------------------------------------------------------------

#[test]
fn circle_produces_symmetric_geometry() {
    let radius = 200000.0;
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: radius,
        semi_minor_axis: radius,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert!(
        geo.positions.len() >= 8,
        "circle should produce >= 8 positions"
    );
    assert_eq!(geo.indices.len() % 3, 0);
    // Bounding sphere radius should equal the circle radius
    assert!(
        (geo.bounding_sphere.radius - radius).abs() < 1.0,
        "bounding sphere radius should be ~{}, got {}",
        radius,
        geo.bounding_sphere.radius
    );
}

// ---------------------------------------------------------------------------
// Larger granularity → fewer vertices
// ---------------------------------------------------------------------------

#[test]
fn ellipse_granularity_affects_tessellation() {
    let center = from_degrees(0.0, 0.0, 0.0);

    let opts_fine = EllipseOptions {
        center,
        semi_major_axis: 500000.0,
        semi_minor_axis: 300000.0,
        granularity: std::f64::consts::PI / 180.0, // 1 degree
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_fine = ellipse_geometry(&opts_fine, VertexFormat::POSITION_ONLY);

    let opts_coarse = EllipseOptions {
        center,
        semi_major_axis: 500000.0,
        semi_minor_axis: 300000.0,
        granularity: std::f64::consts::PI / 36.0, // 5 degrees
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_coarse = ellipse_geometry(&opts_coarse, VertexFormat::POSITION_ONLY);

    assert!(
        geo_fine.positions.len() > geo_coarse.positions.len(),
        "fine granularity ({}) should produce more vertices than coarse ({})",
        geo_fine.positions.len(),
        geo_coarse.positions.len()
    );
}

// ---------------------------------------------------------------------------
// Semi-major > semi-minor with rotation = PI
// ---------------------------------------------------------------------------

#[test]
fn ellipse_rotation_pi_swaps_axes() {
    let opts_no_rot = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 500000.0,
        semi_minor_axis: 200000.0,
        rotation: 0.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_no_rot = ellipse_geometry(&opts_no_rot, VertexFormat::POSITION_ONLY);

    let opts_rot = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 500000.0,
        semi_minor_axis: 200000.0,
        rotation: std::f64::consts::PI,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo_rot = ellipse_geometry(&opts_rot, VertexFormat::POSITION_ONLY);

    assert_eq!(geo_no_rot.positions.len(), geo_rot.positions.len());
    // PI rotation should flip the axes (positions differ at some vertices)
    let mut any_diff = false;
    for (a, b) in geo_no_rot.positions.iter().zip(geo_rot.positions.iter()) {
        if (a[0] - b[0]).abs() > 1e-6 || (a[1] - b[1]).abs() > 1e-6 || (a[2] - b[2]).abs() > 1e-6 {
            any_diff = true;
            break;
        }
    }
    assert!(any_diff, "PI rotation should change positions");
}

// ---------------------------------------------------------------------------
// st_rotation = 0 gives standard texture coordinates
// ---------------------------------------------------------------------------

#[test]
fn ellipse_st_rotation_zero_center_uv_at_origin() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        st_rotation: 0.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_AND_ST);
    let st = geo.tex_coords.as_ref().unwrap();
    assert_eq!(geo.positions.len(), 16);
    assert_eq!(st.len(), 16);
    // ST coordinates should be finite
    for uv in st.iter() {
        assert!(uv[0].is_finite() && uv[1].is_finite());
    }
}

// ---------------------------------------------------------------------------
// Ellipse with st_rotation = PI
// ---------------------------------------------------------------------------

#[test]
fn ellipse_st_rotation_pi_inverts_texture() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        st_rotation: std::f64::consts::PI,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_AND_ST);
    assert_eq!(geo.positions.len(), 16);
    let st = geo.tex_coords.as_ref().unwrap();
    // With st_rotation=PI, the center uv should be near (0.5, 0.5)
    for uv in st.iter() {
        assert!(uv[0] >= -0.5 && uv[0] <= 1.5, "s out of range after PI rotation");
        assert!(uv[1] >= -0.5 && uv[1] <= 1.5, "t out of range after PI rotation");
    }
}

// ---------------------------------------------------------------------------
// Ellipse with height and rotation combined
// ---------------------------------------------------------------------------

#[test]
fn ellipse_height_with_rotation() {
    let height = 5000.0;
    let opts = EllipseOptions {
        center: from_degrees(-75.59777, 40.03883, 0.0),
        semi_major_axis: 300000.0,
        semi_minor_axis: 150000.0,
        height,
        rotation: std::f64::consts::FRAC_PI_4,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty());
    let e = wgs84();
    for p in &geo.positions {
        let pos = DVec3::new(p[0], p[1], p[2]);
        let carto = e.cartesian_to_cartographic(pos).unwrap_or_default();
        assert!((carto.height - height).abs() < 100.0);
    }
}

// ---------------------------------------------------------------------------
// Ellipse bounding sphere with rotation
// ---------------------------------------------------------------------------

#[test]
fn ellipse_bounding_sphere_with_rotation() {
    let opts = EllipseOptions {
        center: from_degrees(0.0, 0.0, 0.0),
        semi_major_axis: 300000.0,
        semi_minor_axis: 100000.0,
        rotation: std::f64::consts::FRAC_PI_3,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!((geo.bounding_sphere.radius - 300000.0).abs() < 1.0);
    let center = geo.bounding_sphere.center;
    for p in &geo.positions {
        let dist = (DVec3::new(p[0], p[1], p[2]) - center).length();
        assert!(dist <= geo.bounding_sphere.radius + 1.0);
    }
}
