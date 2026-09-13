//! Tests ported from CesiumJS QuantizedMeshTerrainDataSpec.js
//! A-class tests: 7 (isChildAvailable coordinate-based + interpolateHeight)
//! C-class omitted: 4 throws + upsample (complex mesh splitting)

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::rectangle::Rectangle;
use cesium_provider::tiling_scheme::{GeographicTilingScheme, TilingScheme};
use cesium_terrain::QuantizedMeshTerrainData;
use glam::DVec3;

fn create_test_data(child_tile_mask: u8) -> QuantizedMeshTerrainData {
    QuantizedMeshTerrainData {
        quantized_vertices: vec![
            // u values (sw, nw, se, ne)
            0, 0, 32767, 32767,
            // v values
            0, 32767, 0, 32767,
            // height values
            16384, 0, 32767, 16384,
        ],
        indices: vec![0, 3, 1, 0, 2, 3],
        minimum_height: -16384.0,
        maximum_height: 16383.0,
        bounding_sphere: BoundingSphere::new(DVec3::ZERO, 1.0),
        horizon_occlusion_point: DVec3::ZERO,
        west_indices: vec![0, 1],
        south_indices: vec![0, 1],
        east_indices: vec![2, 3],
        north_indices: vec![1, 3],
        west_skirt_height: 1.0,
        south_skirt_height: 1.0,
        east_skirt_height: 1.0,
        north_skirt_height: 1.0,
        child_tile_mask,
        created_by_upsampling: false,
        encoded_normals: None,
        water_mask: None,
    }
}

// ===== isChildAvailable (coordinate-based) =====

#[test]
fn is_child_available_returns_true_for_all_children_when_mask_not_specified() {
    // Ported from: "returns true for all children when child mask is not explicitly specified"
    // Default mask = 15 (all children)
    let data = create_test_data(15);

    assert!(data.is_child_available_coords(10, 20, 20, 40)); // SW
    assert!(data.is_child_available_coords(10, 20, 21, 40)); // SE
    assert!(data.is_child_available_coords(10, 20, 20, 41)); // NW
    assert!(data.is_child_available_coords(10, 20, 21, 41)); // NE
}

#[test]
fn is_child_available_works_when_only_southwest_child() {
    // Ported from: "works when only southwest child is available"
    // CesiumJS tile coords: Y increases southward
    // relative_y=0 → north row, relative_y=1 → south row
    let data = create_test_data(1); // bit 0 = SW

    assert!(!data.is_child_available_coords(10, 20, 20, 40)); // NW (bit 2) → false
    assert!(!data.is_child_available_coords(10, 20, 21, 40)); // NE (bit 3) → false
    assert!(data.is_child_available_coords(10, 20, 20, 41));  // SW (bit 0) → true
    assert!(!data.is_child_available_coords(10, 20, 21, 41)); // SE (bit 1) → false
}

#[test]
fn is_child_available_works_when_only_southeast_child() {
    // Ported from: "works when only southeast child is available"
    let data = create_test_data(2); // bit 1 = SE

    assert!(!data.is_child_available_coords(10, 20, 20, 40)); // NW → false
    assert!(!data.is_child_available_coords(10, 20, 21, 40)); // NE → false
    assert!(!data.is_child_available_coords(10, 20, 20, 41)); // SW → false
    assert!(data.is_child_available_coords(10, 20, 21, 41));  // SE → true
}

#[test]
fn is_child_available_works_when_only_northwest_child() {
    // Ported from: "works when only northwest child is available"
    let data = create_test_data(4); // bit 2 = NW

    assert!(data.is_child_available_coords(10, 20, 20, 40));  // NW → true
    assert!(!data.is_child_available_coords(10, 20, 21, 40)); // NE → false
    assert!(!data.is_child_available_coords(10, 20, 20, 41)); // SW → false
    assert!(!data.is_child_available_coords(10, 20, 21, 41)); // SE → false
}

#[test]
fn is_child_available_works_when_only_northeast_child() {
    // Ported from: "works when only northeast child is available"
    let data = create_test_data(8); // bit 3 = NE

    assert!(!data.is_child_available_coords(10, 20, 20, 40)); // NW → false
    assert!(data.is_child_available_coords(10, 20, 21, 40));  // NE → true
    assert!(!data.is_child_available_coords(10, 20, 20, 41)); // SW → false
    assert!(!data.is_child_available_coords(10, 20, 21, 41)); // SE → false
}

// ===== interpolateHeight =====

#[test]
fn interpolate_height_clamps_coordinates_outside_mesh() {
    // Ported from: "clamps coordinates if given a position outside the mesh"
    // Original uses tilingScheme.tileXYToRectangle(7, 6, 5)
    let tiling_scheme = GeographicTilingScheme::default();
    let rectangle = tiling_scheme.tile_xy_to_rectangle(7, 6, 5);

    let data = QuantizedMeshTerrainData {
        quantized_vertices: vec![
            // u (sw, nw, se, ne)
            0, 0, 32767, 32767,
            // v
            0, 32767, 0, 32767,
            // heights: 32767/4, 2*32767/4, 3*32767/4, 32767
            8191, 16383, 24575, 32767,
        ],
        indices: vec![0, 3, 1, 0, 2, 3],
        minimum_height: 0.0,
        maximum_height: 4.0,
        bounding_sphere: BoundingSphere::new(DVec3::ZERO, 1.0),
        horizon_occlusion_point: DVec3::ZERO,
        west_indices: vec![0, 1],
        south_indices: vec![0, 1],
        east_indices: vec![2, 3],
        north_indices: vec![1, 3],
        west_skirt_height: 1.0,
        south_skirt_height: 1.0,
        east_skirt_height: 1.0,
        north_skirt_height: 1.0,
        child_tile_mask: 15,
        created_by_upsampling: false,
        encoded_normals: None,
        water_mask: None,
    };

    // Position (0,0) is outside this tile rectangle → should clamp to nearest edge
    let h_outside = data.interpolate_height(&rectangle, 0.0, 0.0);
    let h_corner = data.interpolate_height(&rectangle, rectangle.east, rectangle.south);
    assert!(
        (h_outside - h_corner).abs() < 1e-10,
        "h_outside={} should equal h_corner={}",
        h_outside,
        h_corner
    );
}

#[test]
fn interpolate_height_returns_correct_triangle_interpolation() {
    // Ported from: "returns a height interpolated from the correct triangle"
    // Heights: sw=16384(→0), nw=0(→-16384), se=32767(→16383), ne=16384(→0)
    // Zero height along SW-NE diagonal, negative in NW, positive in SE
    let data = create_test_data(15);

    let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);

    // Position in northwest quadrant → should be negative
    let longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.25;
    let latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.75;
    let result = data.interpolate_height(&rectangle, longitude, latitude);
    assert!(result < 0.0, "NW quadrant height should be negative, got {}", result);

    // Position in southeast quadrant → should be positive
    let longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.75;
    let latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.25;
    let result = data.interpolate_height(&rectangle, longitude, latitude);
    assert!(result > 0.0, "SE quadrant height should be positive, got {}", result);

    // Position on SW-NE diagonal → should be approximately zero
    let longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.5;
    let latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.5;
    let result = data.interpolate_height(&rectangle, longitude, latitude);
    assert!(
        result.abs() < 1e-10,
        "Center diagonal height should be ~0, got {}",
        result
    );
}
