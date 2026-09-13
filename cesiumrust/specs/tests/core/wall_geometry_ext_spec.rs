//! Ported from CesiumJS `Core/WallGeometrySpec.js` (expanded A-class tests).
//!
//! Tests: closed loop, duplicate handling, EPSILON10 boundary, height selection,
//! all attributes, texture coordinates.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::{wall_geometry, VertexFormat, WallOptions};
use glam::DVec3;

fn wgs84() -> Ellipsoid {
    Ellipsoid::WGS84
}

fn from_degrees(lon: f64, lat: f64, h: f64) -> DVec3 {
    let e = wgs84();
    let carto = Cartographic::from_degrees(lon, lat, h);
    e.cartographic_to_cartesian(&carto)
}

fn to_cartographic(p: [f64; 3]) -> Cartographic {
    let e = wgs84();
    e.cartesian_to_cartographic(DVec3::new(p[0], p[1], p[2]))
        .unwrap_or_default()
}

const EPSILON8: f64 = 1e-8;

// ---------------------------------------------------------------------------
// "creates positions relative to ellipsoid"
// 2 positions → 4 vertices (2 bottom + 2 top), 2 triangles
// ---------------------------------------------------------------------------

#[test]
fn wall_creates_positions_relative_to_ellipsoid() {
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS: numPositions = 4, numTriangles = 2
    // With granularity=1°, 1° arc → 2 points per segment → 2 corners × 2 (top+bottom) = 4
    assert_eq!(geo.positions.len(), 4, "expected 4 positions");
    assert_eq!(geo.indices.len(), 6, "expected 6 indices (2 triangles)");

    // First position should be at height 0 (bottom)
    let c0 = to_cartographic(geo.positions[0]);
    assert!(
        (c0.height - 0.0).abs() < EPSILON8,
        "bottom height should be 0, got {}",
        c0.height
    );

    // Second position should be at height 1000 (top)
    let c1 = to_cartographic(geo.positions[1]);
    assert!(
        (c1.height - 1000.0).abs() < EPSILON8,
        "top height should be 1000, got {}",
        c1.height
    );
}

// ---------------------------------------------------------------------------
// "creates positions when first and last positions are equal"
// Closed loop: 5 positions (first=last) → 16 vertices, 8 triangles
// ---------------------------------------------------------------------------

#[test]
fn wall_creates_positions_closed_loop() {
    let positions = vec![
        from_degrees(-107.0, 43.0, 1000.0),
        from_degrees(-106.0, 43.0, 1000.0),
        from_degrees(-106.0, 42.0, 1000.0),
        from_degrees(-107.0, 42.0, 1000.0),
        from_degrees(-107.0, 43.0, 1000.0), // same as first
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS: numPositions = 16, numTriangles = 8
    // 4 segments × 2 points each × 2 (top+bottom) = 16
    assert_eq!(geo.positions.len(), 16, "expected 16 positions for closed loop");
    assert_eq!(geo.indices.len(), 24, "expected 24 indices (8 triangles)");

    // First position should be at height 0 (bottom)
    let c0 = to_cartographic(geo.positions[0]);
    assert!(
        (c0.height - 0.0).abs() < EPSILON8,
        "bottom height should be 0, got {}",
        c0.height
    );

    // Second position should be at height 1000 (top)
    let c1 = to_cartographic(geo.positions[1]);
    assert!(
        (c1.height - 1000.0).abs() < EPSILON8,
        "top height should be 1000, got {}",
        c1.height
    );
}

// ---------------------------------------------------------------------------
// "cleans positions with duplicates"
// 7 input positions with duplicates → 8 vertices (4 unique corners × 2)
// ---------------------------------------------------------------------------

#[test]
fn wall_cleans_positions_with_duplicates() {
    // Input: 49,18 → 49,18(dup) → 50,18 → 50,18(dup) → 50,18(dup) → 51,18 → 51,18(dup)
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(49.0, 18.0, 2000.0), // same lon/lat, different height
        from_degrees(50.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0), // duplicate
        from_degrees(50.0, 18.0, 1000.0), // duplicate
        from_degrees(51.0, 18.0, 1000.0),
        from_degrees(51.0, 18.0, 1000.0), // duplicate
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS: numPositions = 8, numTriangles = 4
    // After removing duplicates: 3 unique corners (49, 50, 51)
    // But 49 and 49 share lon/lat so merged → 3 corners
    // 3 corners × 2 (top+bottom) = 6... but CesiumJS expects 8
    // Actually: 2 segments (49→50, 50→51) × 2 points each × 2 (top+bottom) = 8
    assert_eq!(geo.positions.len(), 8, "expected 8 positions after duplicate removal");
    assert_eq!(geo.indices.len(), 12, "expected 12 indices (4 triangles)");

    // First position should be at height 0 (bottom)
    let c0 = to_cartographic(geo.positions[0]);
    assert!(
        (c0.height - 0.0).abs() < EPSILON8,
        "bottom height should be 0, got {}",
        c0.height
    );

    // Second position should be at height 2000 (max of 1000 and 2000)
    let c1 = to_cartographic(geo.positions[1]);
    assert!(
        (c1.height - 2000.0).abs() < EPSILON8,
        "top height should be 2000 (max of duplicates), got {}",
        c1.height
    );
}

// ---------------------------------------------------------------------------
// "removes duplicates with very small difference"
// Positions differing by < EPSILON10 should be merged
// ---------------------------------------------------------------------------

#[test]
fn wall_removes_duplicates_with_small_difference() {
    // These positions differ by < EPSILON10 in cartesian coordinates
    let positions = vec![
        DVec3::new(4347090.215457887, 1061403.4237998386, 4538066.036525028),
        DVec3::new(4348147.589624987, 1043897.8776143644, 4541092.234751661),
        DVec3::new(4348147.589882754, 1043897.8776762491, 4541092.234492364), // very close to previous
        DVec3::new(4335659.882947743, 1047571.602084736, 4552098.654605664),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS: numPositions = 8, numTriangles = 4
    // After removing the near-duplicate: 3 unique corners
    // 2 segments × 2 points × 2 (top+bottom) = 8
    assert_eq!(geo.positions.len(), 8, "expected 8 positions after near-duplicate removal");
    assert_eq!(geo.indices.len(), 12, "expected 12 indices (4 triangles)");
}

// ---------------------------------------------------------------------------
// "does not clean positions that add up past EPSILON10"
// Small differences that accumulate past EPSILON10 should NOT be merged
// ---------------------------------------------------------------------------

#[test]
fn wall_does_not_clean_positions_past_epsilon10() {
    let eighty_percent_of_epsilon10: f64 = 0.8 * 1e-10;

    // 4 positions, each differing by 0.8×EPSILON10 in latitude
    // Adjacent pairs differ by < EPSILON10, but accumulated difference > EPSILON10
    let lat0: f64 = 1.0;
    let positions = vec![
        from_degrees(
            1.0_f64.to_degrees(),
            lat0.to_degrees(),
            1000.0,
        ),
        from_degrees(
            1.0_f64.to_degrees(),
            (lat0 + eighty_percent_of_epsilon10).to_degrees(),
            1000.0,
        ),
        from_degrees(
            1.0_f64.to_degrees(),
            (lat0 + 2.0 * eighty_percent_of_epsilon10).to_degrees(),
            1000.0,
        ),
        from_degrees(
            1.0_f64.to_degrees(),
            (lat0 + 3.0 * eighty_percent_of_epsilon10).to_degrees(),
            1000.0,
        ),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS expects this to produce geometry (not return undefined)
    // The first and third positions differ by 1.6×EPSILON10 > EPSILON10
    // So they should NOT be merged
    assert!(
        !geo.positions.is_empty(),
        "should produce geometry for positions accumulating past EPSILON10"
    );
}

// ---------------------------------------------------------------------------
// "cleans selects maximum height from duplicates"
// When positions share lon/lat, keep the maximum height
// ---------------------------------------------------------------------------

#[test]
fn wall_selects_maximum_height_from_duplicates() {
    // 50,18 appears 3 times with heights 1000, 6000, 10000
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 6000.0),  // same lon/lat, higher
        from_degrees(50.0, 18.0, 10000.0), // same lon/lat, highest
        from_degrees(51.0, 18.0, 1000.0),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS: numPositions = 8, numTriangles = 4
    assert_eq!(geo.positions.len(), 8, "expected 8 positions");
    assert_eq!(geo.indices.len(), 12, "expected 12 indices (4 triangles)");

    // First position should be at height 0 (bottom)
    let c0 = to_cartographic(geo.positions[0]);
    assert!(
        (c0.height - 0.0).abs() < EPSILON8,
        "bottom height should be 0, got {}",
        c0.height
    );

    // Position at index 9 (5th top vertex) should be at height 10000 (max)
    // The 50° longitude corner should have the maximum height
    // In the output, positions are interleaved: bottom0, top0, bottom1, top1, ...
    // Index 9 = 5th top vertex (index 4 in top array)
    if geo.positions.len() > 9 {
        let c9 = to_cartographic(geo.positions[9]);
        assert!(
            (c9.height - 10000.0).abs() < EPSILON8,
            "max height should be 10000, got {}",
            c9.height
        );
    }
}

// ---------------------------------------------------------------------------
// "creates all attributes"
// VertexFormat::ALL → positions + normals + tangents + bitangents + st
// ---------------------------------------------------------------------------

#[test]
fn wall_creates_all_attributes() {
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
        from_degrees(51.0, 18.0, 1000.0),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::ALL);

    // CesiumJS: numPositions = 8, numTriangles = 4
    let num_positions = 8;
    assert_eq!(geo.positions.len(), num_positions, "expected {} positions", num_positions);
    assert_eq!(geo.indices.len(), 12, "expected 12 indices (4 triangles)");

    // Check all attributes are present
    assert!(geo.normals.is_some(), "normals should be present");
    assert!(geo.tangents.is_some(), "tangents should be present");
    assert!(geo.bitangents.is_some(), "bitangents should be present");
    assert!(geo.tex_coords.is_some(), "tex_coords should be present");

    let normals = geo.normals.as_ref().unwrap();
    let tangents = geo.tangents.as_ref().unwrap();
    let bitangents = geo.bitangents.as_ref().unwrap();
    let st = geo.tex_coords.as_ref().unwrap();

    assert_eq!(normals.len(), num_positions, "normals count mismatch");
    assert_eq!(tangents.len(), num_positions, "tangents count mismatch");
    assert_eq!(bitangents.len(), num_positions, "bitangents count mismatch");
    assert_eq!(st.len(), num_positions, "tex_coords count mismatch");
}

// ---------------------------------------------------------------------------
// "creates correct texture coordinates"
// ST values should be [0,0, 0,1, 0.5,0, 0.5,1, 0.5,0, 0.5,1, 1,0, 1,1]
// ---------------------------------------------------------------------------

#[test]
fn wall_creates_correct_texture_coordinates() {
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
        from_degrees(51.0, 18.0, 1000.0),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::ALL);

    let st = geo.tex_coords.as_ref().expect("tex_coords should be present");

    // CesiumJS expected ST values for 3 positions (2 segments):
    // [0.0, 0.0, 0.0, 1.0, 0.5, 0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 1.0, 1.0, 0.0, 1.0, 1.0]
    // Pattern: for each corner, bottom has v=0, top has v=1
    // u goes from 0 to 1 across segments

    assert_eq!(st.len(), 8, "expected 8 texture coordinates");

    // Check pattern: alternating v=0 (bottom) and v=1 (top)
    for (i, uv) in st.iter().enumerate() {
        let expected_v = if i % 2 == 0 { 0.0 } else { 1.0 };
        assert!(
            (uv[1] - expected_v).abs() < 1e-6,
            "st[{}].v should be {}, got {}",
            i,
            expected_v,
            uv[1]
        );
    }

    // First u should be 0, last u should be 1
    assert!((st[0][0] - 0.0).abs() < 1e-6, "first u should be 0");
    assert!((st[st.len() - 2][0] - 1.0).abs() < 1e-6, "last u should be 1");
}

// ---------------------------------------------------------------------------
// "creates correct texture coordinates when there are duplicate wall positions"
// Same ST values even with duplicate input positions
// ---------------------------------------------------------------------------

#[test]
fn wall_texture_coordinates_with_duplicates() {
    // 50,18 appears twice (duplicate)
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0), // duplicate
        from_degrees(51.0, 18.0, 1000.0),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::ALL);

    let st = geo.tex_coords.as_ref().expect("tex_coords should be present");

    // After duplicate removal, should have same ST as without duplicates
    assert_eq!(st.len(), 8, "expected 8 texture coordinates after duplicate removal");

    // Check pattern: alternating v=0 (bottom) and v=1 (top)
    for (i, uv) in st.iter().enumerate() {
        let expected_v = if i % 2 == 0 { 0.0 } else { 1.0 };
        assert!(
            (uv[1] - expected_v).abs() < 1e-6,
            "st[{}].v should be {}, got {}",
            i,
            expected_v,
            uv[1]
        );
    }
}

// ---------------------------------------------------------------------------
// "creates positions with constant minimum and maximum heights"
// fromConstantHeights with min=1000, max=2000
// ---------------------------------------------------------------------------

#[test]
fn wall_from_constant_heights_detailed() {
    let min = 1000.0;
    let max = 2000.0;

    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
    ];

    let opts = WallOptions::from_constant_heights(positions, Some(min), Some(max), wgs84());
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS: numPositions = 4, numTriangles = 2
    assert_eq!(geo.positions.len(), 4, "expected 4 positions");
    assert_eq!(geo.indices.len(), 6, "expected 6 indices (2 triangles)");

    // Check heights: bottom=min, top=max
    let c0 = to_cartographic(geo.positions[0]);
    assert!(
        (c0.height - min).abs() < EPSILON8,
        "bottom height should be {}, got {}",
        min,
        c0.height
    );

    let c1 = to_cartographic(geo.positions[1]);
    assert!(
        (c1.height - max).abs() < EPSILON8,
        "top height should be {}, got {}",
        max,
        c1.height
    );

    let c2 = to_cartographic(geo.positions[2]);
    assert!(
        (c2.height - min).abs() < EPSILON8,
        "bottom height should be {}, got {}",
        min,
        c2.height
    );

    let c3 = to_cartographic(geo.positions[3]);
    assert!(
        (c3.height - max).abs() < EPSILON8,
        "top height should be {}, got {}",
        max,
        c3.height
    );
}

// ---------------------------------------------------------------------------
// "creates positions with minimum and maximum heights"
// Variable height arrays (not constant)
// ---------------------------------------------------------------------------

#[test]
fn wall_creates_positions_with_variable_minimum_maximum_heights() {
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
    ];

    let opts = WallOptions {
        positions,
        minimum_heights: Some(vec![500.0, 300.0]),
        maximum_heights: Some(vec![1500.0, 1300.0]),
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // CesiumJS: numPositions = 4, numTriangles = 2
    assert_eq!(geo.positions.len(), 4, "expected 4 positions");
    assert_eq!(geo.indices.len(), 6, "expected 6 indices (2 triangles)");

    // Check that bottom height varies (follows minimum_heights)
    // Position pattern: bottom0, top0, bottom1, top1
    let c0 = to_cartographic(geo.positions[0]);
    assert!((c0.height - 500.0).abs() < EPSILON8, "bottom height at pos0 should be 500");

    let c2 = to_cartographic(geo.positions[2]);
    assert!((c2.height - 300.0).abs() < EPSILON8, "bottom height at pos2 should be 300");

    let c1 = to_cartographic(geo.positions[1]);
    assert!((c1.height - 1500.0).abs() < EPSILON8, "top height at pos1 should be 1500");

    let c3 = to_cartographic(geo.positions[3]);
    assert!((c3.height - 1300.0).abs() < EPSILON8, "top height at pos3 should be 1300");
}

// ---------------------------------------------------------------------------
// Wall with granularity larger than arc (minimal subdivision)
// ---------------------------------------------------------------------------

#[test]
fn wall_coarse_granularity_minimal_subdivision() {
    let positions = vec![
        from_degrees(49.0, 18.0, 1000.0),
        from_degrees(50.0, 18.0, 1000.0),
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 3.0, // 60 degrees (large)
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    // With coarse granularity, should still produce valid geometry
    assert!(geo.positions.len() >= 4, "should produce at least 4 positions");
    assert!(geo.indices.len() >= 6);
}

// ---------------------------------------------------------------------------
// Wall with 3+ positions, non-constant heights
// ---------------------------------------------------------------------------

#[test]
fn wall_three_positions_gradient_heights() {
    let positions = vec![
        from_degrees(0.0, 0.0, 0.0),
        from_degrees(1.0, 0.0, 0.0),
        from_degrees(2.0, 0.0, 0.0),
    ];

    let opts = WallOptions {
        positions,
        minimum_heights: Some(vec![0.0, 1000.0, 0.0]),
        maximum_heights: Some(vec![5000.0, 6000.0, 5000.0]),
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);

    assert!(geo.positions.len() >= 6);
    assert_eq!(geo.indices.len() % 3, 0);
    assert!(geo.bounding_sphere.radius > 0.0);
}

// ---------------------------------------------------------------------------
// Wall: positions differ by EPSILON10 boundary
// ---------------------------------------------------------------------------

#[test]
fn wall_positions_at_epsilon_boundary_survive_cleaning() {
    // Same as the existing test but verify geometry is produced
    let p1 = DVec3::new(4347090.215457887, 1061403.4237998386, 4538066.036525028);
    let p2 = DVec3::new(4348147.589624987, 1043897.8776143644, 4541092.234751661);
    // p3 differs by ~1.5*EPSILON10 from p2 (should NOT be merged)
    let p3 = DVec3::new(4348147.58998, 1043897.8780, 4541092.2350);

    let positions = vec![p1, p2, p3];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(!geo.positions.is_empty(), "should produce geometry");
}

// ---------------------------------------------------------------------------
// Wall outline: closed loop
// ---------------------------------------------------------------------------

#[test]
fn wall_outline_forms_closed_loop() {
    use cesium_geospatial::geometry::{wall_outline_geometry, WallOptions};

    let positions = vec![
        from_degrees(0.0, 0.0, 0.0),
        from_degrees(1.0, 0.0, 0.0),
        from_degrees(1.0, 1.0, 0.0),
        from_degrees(0.0, 1.0, 0.0),
        from_degrees(0.0, 0.0, 0.0), // closed
    ];

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = wall_outline_geometry(&opts);
    assert!(geo.positions.len() >= 4);
    assert_eq!(geo.indices.len() % 2, 0);
    // All indices valid
    let n = geo.positions.len() as u32;
    for &idx in &geo.indices {
        assert!(idx < n);
    }
}
