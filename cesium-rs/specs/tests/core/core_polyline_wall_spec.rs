//! Tests for PolylineGeometry, PolylineVolumeGeometry,
//! WallGeometryLibrary, DecodeVectorPolylinePositions.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::decode_vector_polyline_positions::decode_vector_polyline_positions;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::polyline_geometry::PolylineGeometry;
use cesium_core::polyline_volume_geometry::{Cartesian2Stub, PolylineVolumeGeometry};
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
    let colors = Some(vec![[1.0, 0.0, 0.0, 1.0]]);
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
        Cartesian2Stub { x: -0.5, y: -0.5 },
        Cartesian2Stub { x: 0.5, y: -0.5 },
        Cartesian2Stub { x: 0.5, y: 0.5 },
        Cartesian2Stub { x: -0.5, y: 0.5 },
    ];
    let geom = PolylineVolumeGeometry::new(positions, shape, None, None);
    let _ = geom;
}

#[test]
fn polyline_volume_geometry_custom_corner_type() {
    let positions = vec![Cartesian3::new(0.0, 0.0, 6378137.0)];
    let shape = vec![Cartesian2Stub { x: 0.0, y: 0.0 }];
    // corner_type: 0=Rounded, 1=Mitered, 2=Beveled
    let geom = PolylineVolumeGeometry::new(positions, shape, Some(2), Some(0.01));
    let _ = geom;
}

#[test]
fn cartesian2_stub_copy() {
    let a = Cartesian2Stub { x: 1.0, y: 2.0 };
    let b = a; // Copy
    assert_eq!(b.x, 1.0);
    assert_eq!(b.y, 2.0);
}

// --- WallGeometryLibrary ---
#[test]
fn wall_compute_positions_returns_none() {
    // TODO implementation → always returns None
    let ellipsoid = Ellipsoid::WGS84;
    let positions = vec![
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(0.0, 1.0, 0.0),
    ];
    let result = compute_positions(&ellipsoid, &positions, None, None, 0.01, false);
    assert!(result.is_none());
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
