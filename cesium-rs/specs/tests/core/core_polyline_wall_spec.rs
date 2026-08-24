//! Tests for PolylineGeometry, PolylineVolumeGeometry,
//! WallGeometryLibrary, DecodeVectorPolylinePositions.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::decode_vector_polyline_positions::decode_vector_polyline_positions;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::polyline_geometry::PolylineGeometry;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::corner_type::CornerType;
use cesium_core::polyline_volume_geometry::PolylineVolumeGeometry;
use cesium_core::rectangle::Rectangle;
use cesium_core::simple_polyline_geometry::ArcType;
use cesium_core::wall_geometry_library::compute_positions;

// --- PolylineGeometry ---
#[test]
fn polyline_geometry_new_defaults() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
    ];
    let geom = PolylineGeometry::new(positions, None, None, None, None, None, None);
    let _ = geom;
}

#[test]
fn polyline_geometry_new_custom() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 6378137.0),
        Cartesian3::new(1.0, 0.0, 6378137.0),
    ];
    // JS debug check: colors must have the same length as positions.
    let colors = Some(vec![[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]]);
    let geom = PolylineGeometry::new(
        positions,
        Some(5.0),
        colors,
        Some(true),
        Some(ArcType::Rhumb),
        Some(CesiumMath::RADIANS_PER_DEGREE),
        Some(Ellipsoid::WGS84.clone()),
    );
    let _ = geom;
}

// --- PolylineVolumeGeometry ---
#[test]
fn polyline_volume_geometry_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 6378137.0),
        Cartesian3::new(1.0, 0.0, 6378137.0),
    ];
    let shape = vec![
        Cartesian2 { x: -0.5, y: -0.5 },
        Cartesian2 { x: 0.5, y: -0.5 },
        Cartesian2 { x: 0.5, y: 0.5 },
        Cartesian2 { x: -0.5, y: 0.5 },
    ];
    let geom = PolylineVolumeGeometry::new(positions, shape, None, None, None, None);
    let _ = geom;
}

#[test]
fn polyline_volume_geometry_custom_corner_type() {
    let positions = vec![Cartesian3::new(0.0, 0.0, 6378137.0)];
    let shape = vec![Cartesian2 { x: 0.0, y: 0.0 }];
    // corner_type: 0=Rounded, 1=Mitered, 2=Beveled
    let geom = PolylineVolumeGeometry::new(positions, shape, None, Some(CornerType::Beveled), None, Some(0.01));
    let _ = geom;
}

#[test]
fn cartesian2_stub_copy() {
    let a = Cartesian2 { x: 1.0, y: 2.0 };
    let b = a; // Copy
    assert_eq!(b.x, 1.0);
    assert_eq!(b.y, 2.0);
}

// --- WallGeometryLibrary ---
#[test]
fn wall_compute_positions_returns_none() {
    // Mirrors JS behavior: with all top/bottom heights zero the cleaned
    // positions are degenerate and computePositions returns undefined.
    let ellipsoid = Ellipsoid::WGS84;
    let positions = Cartesian3::from_degrees_array(&[0.0, 0.0, 1.0, 1.0], None, None);
    let zero_heights = [0.0, 0.0];
    let result = compute_positions(
        &ellipsoid,
        &positions,
        Some(&zero_heights),
        Some(&zero_heights),
        0.01,
        false,
    );
    assert!(result.is_none());

    // Fewer than 2 positions also yields None.
    let single = Cartesian3::from_degrees_array(&[0.0, 0.0], None, None);
    let result = compute_positions(&ellipsoid, &single, None, None, 0.01, false);
    assert!(result.is_none());
}

#[test]
fn wall_compute_positions_computes_top_and_bottom() {
    // Mirrors WallGeometryLibrarySpec.js "computePositions" happy path:
    // distinct positions with non-zero top heights produce top/bottom
    // position buffers.
    let ellipsoid = Ellipsoid::WGS84;
    let positions = Cartesian3::from_degrees_array(
        &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0],
        None,
        None,
    );
    let max_heights = [1000.0, 2000.0, 3000.0];
    let min_heights = [0.0, 0.0, 0.0];
    let result = compute_positions(
        &ellipsoid,
        &positions,
        Some(&max_heights),
        Some(&min_heights),
        0.01,
        false,
    );
    assert!(result.is_some());
    let result = result.unwrap();
    // 3 distinct positions => numCorners = length - 2
    assert_eq!(result.num_corners, 1);
    assert!(!result.top_positions.is_empty());
    assert!(!result.bottom_positions.is_empty());
    assert_eq!(result.top_positions.len(), result.bottom_positions.len());
}

// --- DecodeVectorPolylinePositions ---
#[test]
fn decode_vector_polyline_positions_output_length() {
    // 3 positions → input has 9 elements (3 * 3)
    let num_positions = 3;
    let positions = vec![0.0; num_positions * 3]; // u, v, h buffers concatenated
    let rectangle = Rectangle::new(-1.0, -0.5, 1.0, 0.5);
    let ellipsoid = Ellipsoid::WGS84;
    let decoded = decode_vector_polyline_positions(
        &positions,
        &rectangle,
        0.0,
        1000.0,
        &ellipsoid,
    );
    assert_eq!(decoded.len(), num_positions * 3);
}

#[test]
fn decode_vector_polyline_positions_produces_cartesian() {
    let num_positions = 2;
    // Simple encoded data: all zeros → should decode to west/south/min_height
    let positions = vec![0.0; num_positions * 3];
    let rectangle = Rectangle::new(-1.0, -0.5, 1.0, 0.5);
    let ellipsoid = Ellipsoid::WGS84;
    let decoded = decode_vector_polyline_positions(
        &positions,
        &rectangle,
        0.0,
        1000.0,
        &ellipsoid,
    );
    // At least one component should be non-zero (Cartesian from ellipsoid)
    let has_nonzero = decoded.iter().any(|&v| v.abs() > 1e-10);
    assert!(has_nonzero);
}
