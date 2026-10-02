//! 参考自 CesiumJS QuantizedMeshTerrainDataSpec 的测试
//! A 类测试：7 个（isChildAvailable 基于坐标 + interpolateHeight）
//! C 类已省略：4 个 throws + upsample（复杂的网格拆分）

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::rectangle::Rectangle;
use cesium_provider::tiling_scheme::GeographicTilingScheme;
use cesium_terrain::QuantizedMeshTerrainData;
use glam::DVec3;

fn create_test_data(child_tile_mask: u8) -> QuantizedMeshTerrainData {
    QuantizedMeshTerrainData {
        quantized_vertices: vec![
            // u 值（sw, nw, se, ne）
            0, 0, 32767, 32767,
            // v 值
            0, 32767, 0, 32767,
            // 高度值
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

// ===== isChildAvailable（基于坐标）=====

#[test]
fn is_child_available_returns_true_for_all_children_when_mask_not_specified() {
    // 参考自: "returns true for all children when child mask is not explicitly specified"
    // 默认掩码 = 15（所有子块）
    let data = create_test_data(15);

    assert!(data.is_child_available_coords(10, 20, 20, 40)); // SW
    assert!(data.is_child_available_coords(10, 20, 21, 40)); // SE
    assert!(data.is_child_available_coords(10, 20, 20, 41)); // NW
    assert!(data.is_child_available_coords(10, 20, 21, 41)); // NE
}

#[test]
fn is_child_available_works_when_only_southwest_child() {
    // 参考自: "works when only southwest child is available"
    // CesiumJS 瓦片坐标：Y 向南增大
    // relative_y=0 → 北行，relative_y=1 → 南行
    let data = create_test_data(1); // bit 0 = SW

    assert!(!data.is_child_available_coords(10, 20, 20, 40)); // NW (bit 2) → false
    assert!(!data.is_child_available_coords(10, 20, 21, 40)); // NE (bit 3) → false
    assert!(data.is_child_available_coords(10, 20, 20, 41));  // SW (bit 0) → true
    assert!(!data.is_child_available_coords(10, 20, 21, 41)); // SE (bit 1) → false
}

#[test]
fn is_child_available_works_when_only_southeast_child() {
    // 参考自: "works when only southeast child is available"
    let data = create_test_data(2); // bit 1 = SE

    assert!(!data.is_child_available_coords(10, 20, 20, 40)); // NW → false
    assert!(!data.is_child_available_coords(10, 20, 21, 40)); // NE → false
    assert!(!data.is_child_available_coords(10, 20, 20, 41)); // SW → false
    assert!(data.is_child_available_coords(10, 20, 21, 41));  // SE → true
}

#[test]
fn is_child_available_works_when_only_northwest_child() {
    // 参考自: "works when only northwest child is available"
    let data = create_test_data(4); // bit 2 = NW

    assert!(data.is_child_available_coords(10, 20, 20, 40));  // NW → true
    assert!(!data.is_child_available_coords(10, 20, 21, 40)); // NE → false
    assert!(!data.is_child_available_coords(10, 20, 20, 41)); // SW → false
    assert!(!data.is_child_available_coords(10, 20, 21, 41)); // SE → false
}

#[test]
fn is_child_available_works_when_only_northeast_child() {
    // 参考自: "works when only northeast child is available"
    let data = create_test_data(8); // bit 3 = NE

    assert!(!data.is_child_available_coords(10, 20, 20, 40)); // NW → false
    assert!(data.is_child_available_coords(10, 20, 21, 40));  // NE → true
    assert!(!data.is_child_available_coords(10, 20, 20, 41)); // SW → false
    assert!(!data.is_child_available_coords(10, 20, 21, 41)); // SE → false
}

// ===== interpolateHeight =====

#[test]
fn interpolate_height_clamps_coordinates_outside_mesh() {
    // 参考自: "clamps coordinates if given a position outside the mesh"
    // 原始使用 tilingScheme.tileXYToRectangle(7, 6, 5)
    let tiling_scheme = GeographicTilingScheme::default();
    let rectangle = tiling_scheme.tile_xy_to_rectangle(7, 6, 5);

    let data = QuantizedMeshTerrainData {
        quantized_vertices: vec![
            // u（sw, nw, se, ne）
            0, 0, 32767, 32767,
            // v
            0, 32767, 0, 32767,
            // 高度：32767/4, 2*32767/4, 3*32767/4, 32767
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

    // 位置 (0,0) 在此瓦片矩形之外 → 应钳制到最近的边
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
    // 参考自: "returns a height interpolated from the correct triangle"
    // 高度：sw=16384(→0), nw=0(→-16384), se=32767(→16383), ne=16384(→0)
    // 沿 SW-NE 对角线高度为零，NW 为负，SE 为正
    let data = create_test_data(15);

    let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);

    // 位于西北象限的位置 → 应为负
    let longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.25;
    let latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.75;
    let result = data.interpolate_height(&rectangle, longitude, latitude);
    assert!(result < 0.0, "NW quadrant height should be negative, got {}", result);

    // 位于东南象限的位置 → 应为正
    let longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.75;
    let latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.25;
    let result = data.interpolate_height(&rectangle, longitude, latitude);
    assert!(result > 0.0, "SE quadrant height should be positive, got {}", result);

    // 位于 SW-NE 对角线上的位置 → 应近似为零
    let longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.5;
    let latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.5;
    let result = data.interpolate_height(&rectangle, longitude, latitude);
    assert!(
        result.abs() < 1e-10,
        "Center diagonal height should be ~0, got {}",
        result
    );
}
