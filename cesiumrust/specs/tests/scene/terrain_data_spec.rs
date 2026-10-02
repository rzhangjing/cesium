//! Core/HeightmapTerrainData + QuantizedMeshTerrainData → Rust 集成测试。
//!
//! 参考 CesiumJS：
//! - Core/HeightmapTerrainData
//! - Core/QuantizedMeshTerrainData
//!
//! A 类测试：heightmap get/interpolate/create_mesh/child_mask、
//! quantized mesh 顶点访问器/create_mesh/skirts/child_mask。
//! 省略的 C 类：Worker 创建、ArrayBuffer 转移、upsampling（需要完整流水线）。

use cesium_terrain::{HeightmapTerrainData, QuantizedMeshTerrainData, MAX_SHORT};
use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;

// === HeightmapTerrainData ===

fn make_heightmap() -> HeightmapTerrainData {
    // 4x3 高度图（width=4, height=3）
    let heights = vec![
        0.0, 100.0, 200.0, 300.0,   // 行 0（南）
        50.0, 150.0, 250.0, 350.0,  // 行 1（中）
        100.0, 200.0, 300.0, 400.0, // 行 2（北）
    ];
    HeightmapTerrainData::new(heights, 4, 3, 0.0, 400.0)
}

#[test]
fn heightmap_creation() {
    let data = make_heightmap();
    assert_eq!(data.width, 4);
    assert_eq!(data.height, 3);
    assert_eq!(data.minimum_height, 0.0);
    assert_eq!(data.maximum_height, 400.0);
    assert_eq!(data.heights.len(), 12);
}

#[test]
fn heightmap_get_height() {
    let data = make_heightmap();
    assert_eq!(data.get_height(0, 0), Some(0.0));
    assert_eq!(data.get_height(3, 0), Some(300.0));
    assert_eq!(data.get_height(1, 1), Some(150.0));
    assert_eq!(data.get_height(3, 2), Some(400.0));
}

#[test]
fn heightmap_get_height_out_of_bounds() {
    let data = make_heightmap();
    assert_eq!(data.get_height(4, 0), None);
    assert_eq!(data.get_height(0, 3), None);
    assert_eq!(data.get_height(100, 100), None);
}

#[test]
fn heightmap_interpolate_corners() {
    let data = make_heightmap();
    // (0,0) = SW 角 = heights[0] = 0.0
    assert!((data.interpolate_height(0.0, 0.0) - 0.0).abs() < 0.01);
    // (1,0) = SE 角 = heights[3] = 300.0
    assert!((data.interpolate_height(1.0, 0.0) - 300.0).abs() < 0.01);
    // (0,1) = NW 角 = heights[8] = 100.0
    assert!((data.interpolate_height(0.0, 1.0) - 100.0).abs() < 0.01);
    // (1,1) = NE 角 = heights[11] = 400.0
    assert!((data.interpolate_height(1.0, 1.0) - 400.0).abs() < 0.01);
}

#[test]
fn heightmap_interpolate_midpoint() {
    let data = make_heightmap();
    // 网格中心：双线性插值
    // u=0.5 → col_f=1.5, v=0.5 → row_f=1.0
    // row0=1, row1=1 (since row_f=1.0 exactly → row0=1, row1=min(2, 2)=2? no, floor(1.0)=1)
    // Actually row_f = 0.5 * (3-1) = 1.0, so row0=1, row1=min(2,2)=2, dv=0.0
    // col_f = 0.5 * (4-1) = 1.5, col0=1, col1=2, du=0.5
    // h00 = heights[1*4+1] = 150, h10 = heights[1*4+2] = 250
    // h01 = heights[2*4+1] = 200, h11 = heights[2*4+2] = 300
    // h0 = lerp(150, 250, 0.5) = 200
    // h1 = lerp(200, 300, 0.0) = 200 (dv=0)
    // result = lerp(200, 200, 0.0) = 200
    let mid = data.interpolate_height(0.5, 0.5);
    assert!((mid - 200.0).abs() < 0.01);
}

#[test]
fn heightmap_create_mesh() {
    let data = make_heightmap();
    let rectangle = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
    let ellipsoid = Ellipsoid::WGS84;
    let mesh = data.create_mesh(&rectangle, &ellipsoid);

    // 4*3 = 12 个顶点
    assert_eq!(mesh.positions.len(), 12);
    // (4-1)*(3-1)*2*3 = 3*2*6 = 36 个索引
    assert_eq!(mesh.indices.len(), 36);
    assert!(mesh.normals.is_some());
    assert!(mesh.tex_coords.is_some());
}

#[test]
fn heightmap_child_mask() {
    let data = make_heightmap();
    // 默认 child_tile_mask = 15（全部 4 个子块）
    assert!(data.is_child_available(0));
    assert!(data.is_child_available(1));
    assert!(data.is_child_available(2));
    assert!(data.is_child_available(3));
}

#[test]
fn heightmap_partial_child_mask() {
    let mut data = make_heightmap();
    data.child_tile_mask = 0b0101; // 仅子块 0 和 2
    assert!(data.is_child_available(0));
    assert!(!data.is_child_available(1));
    assert!(data.is_child_available(2));
    assert!(!data.is_child_available(3));
}

// === QuantizedMeshTerrainData ===

fn make_quantized_mesh() -> QuantizedMeshTerrainData {
    // 4 个顶点构成一个四边形
    QuantizedMeshTerrainData {
        quantized_vertices: vec![
            // u 值：SW=0, SE=MAX, NW=0, NE=MAX
            0, MAX_SHORT, 0, MAX_SHORT,
            // v 值：SW=0, SE=0, NW=MAX, NE=MAX
            0, 0, MAX_SHORT, MAX_SHORT,
            // height 值：全部居中
            16384, 16384, 16384, 16384,
        ],
        indices: vec![0, 1, 2, 1, 3, 2],
        minimum_height: -50.0,
        maximum_height: 500.0,
        bounding_sphere: BoundingSphere::new(DVec3::new(1000.0, 2000.0, 3000.0), 50000.0),
        horizon_occlusion_point: DVec3::new(1000.0, 2000.0, 3000.0),
        west_indices: vec![0, 2],
        south_indices: vec![0, 1],
        east_indices: vec![1, 3],
        north_indices: vec![2, 3],
        west_skirt_height: 200.0,
        south_skirt_height: 200.0,
        east_skirt_height: 200.0,
        north_skirt_height: 200.0,
        child_tile_mask: 15,
        created_by_upsampling: false,
        encoded_normals: None,
        water_mask: None,
    }
}

#[test]
fn quantized_mesh_vertex_count() {
    let data = make_quantized_mesh();
    assert_eq!(data.vertex_count(), 4);
}

#[test]
fn quantized_mesh_u_values() {
    let data = make_quantized_mesh();
    assert_eq!(data.u_values(), &[0, MAX_SHORT, 0, MAX_SHORT]);
}

#[test]
fn quantized_mesh_v_values() {
    let data = make_quantized_mesh();
    assert_eq!(data.v_values(), &[0, 0, MAX_SHORT, MAX_SHORT]);
}

#[test]
fn quantized_mesh_height_values() {
    let data = make_quantized_mesh();
    assert_eq!(data.height_values(), &[16384, 16384, 16384, 16384]);
}

#[test]
fn quantized_mesh_child_availability() {
    let data = make_quantized_mesh();
    assert!(data.is_child_available(0));
    assert!(data.is_child_available(1));
    assert!(data.is_child_available(2));
    assert!(data.is_child_available(3));
}

#[test]
fn quantized_mesh_partial_child_mask() {
    let mut data = make_quantized_mesh();
    data.child_tile_mask = 0b1010; // 子块 1 和 3
    assert!(!data.is_child_available(0));
    assert!(data.is_child_available(1));
    assert!(!data.is_child_available(2));
    assert!(data.is_child_available(3));
}

#[test]
fn quantized_mesh_create_mesh() {
    let data = make_quantized_mesh();
    let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let ellipsoid = Ellipsoid::WGS84;
    let mesh = data.create_mesh(&rectangle, &ellipsoid, 1.0);

    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
    assert!(mesh.tex_coords.is_some());
    assert_eq!(mesh.minimum_height, -50.0);
    assert_eq!(mesh.maximum_height, 500.0);
}

#[test]
fn quantized_mesh_positions_on_ellipsoid() {
    let data = make_quantized_mesh();
    let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let ellipsoid = Ellipsoid::WGS84;
    let mesh = data.create_mesh(&rectangle, &ellipsoid, 1.0);

    // 所有位置应接近椭球表面
    // 地心半径从约 6357km（极点）变化到约 6378km（赤道）
    for pos in &mesh.positions {
        let r = DVec3::new(pos[0], pos[1], pos[2]).length();
        assert!(r > 6350000.0, "radius {} too small", r);
        assert!(r < 6400000.0, "radius {} too large", r);
    }
}

#[test]
fn quantized_mesh_create_mesh_with_skirts() {
    let data = make_quantized_mesh();
    let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let ellipsoid = Ellipsoid::WGS84;
    let mesh = data.create_mesh_with_skirts(&rectangle, &ellipsoid, 1.0);

    // 由于 skirts 顶点更多（4 个基础 + skirt 顶点）
    assert!(mesh.positions.len() > 4);
    // 由于 skirt 三角形索引更多
    assert!(mesh.indices.len() > 6);
}

#[test]
fn quantized_mesh_uv_coordinates() {
    let data = make_quantized_mesh();
    let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let ellipsoid = Ellipsoid::WGS84;
    let mesh = data.create_mesh(&rectangle, &ellipsoid, 1.0);

    let uvs = mesh.tex_coords.unwrap();
    // 顶点 0：u=0/MAX=0, v=0/MAX=0
    assert!((uvs[0][0] - 0.0).abs() < 1e-4);
    assert!((uvs[0][1] - 0.0).abs() < 1e-4);
    // 顶点 1：u=MAX/MAX=1, v=0/MAX=0
    assert!((uvs[1][0] - 1.0).abs() < 1e-4);
    assert!((uvs[1][1] - 0.0).abs() < 1e-4);
    // 顶点 2：u=0/MAX=0, v=MAX/MAX=1
    assert!((uvs[2][0] - 0.0).abs() < 1e-4);
    assert!((uvs[2][1] - 1.0).abs() < 1e-4);
}

#[test]
fn quantized_mesh_max_short_constant() {
    assert_eq!(MAX_SHORT, 32767);
}

// === HeightmapTerrainData.upsample ===

#[test]
fn heightmap_upsample_southwest_child() {
    // 3x3 均匀梯度：heights[row][col] = row*10 + col
    let heights = vec![
        0.0, 1.0, 2.0,
        10.0, 11.0, 12.0,
        20.0, 21.0, 22.0,
    ];
    let parent = HeightmapTerrainData::new(heights, 3, 3, 0.0, 22.0);

    // 从父块 (x=0, y=0, level=0) 上采样到 SW 子块 (x=0, y=0, level=1)
    let child = parent.upsample(0, 0, 0, 0, 0, 1);

    // SW 子块覆盖父块的 [0, 0.5] x [0, 0.5]
    // 在 3x3 网格下：u=0.5 映射到 col_f=1.0（精确网格点）
    // 子角点 (0,0) = 父 (0,0) = 0.0
    assert!((child.get_height(0, 0).unwrap() - 0.0).abs() < 1e-10);
    // 子角点 (2,0) = 父 (0.5, 0) → col_f=1, row_f=0 → h=1.0
    assert!((child.get_height(2, 0).unwrap() - 1.0).abs() < 1e-10);
    // 子角点 (0,2) = 父 (0, 0.5) → col_f=0, row_f=1 → h=10.0
    assert!((child.get_height(0, 2).unwrap() - 10.0).abs() < 1e-10);
    // 子角点 (2,2) = 父 (0.5, 0.5) → col_f=1, row_f=1 → h=11.0
    assert!((child.get_height(2, 2).unwrap() - 11.0).abs() < 1e-10);
    assert!(child.created_by_upsampling);
}

#[test]
fn heightmap_upsample_eastern_child() {
    let heights = vec![
        0.0, 1.0, 2.0,
        10.0, 11.0, 12.0,
        20.0, 21.0, 22.0,
    ];
    let parent = HeightmapTerrainData::new(heights, 3, 3, 0.0, 22.0);

    // SE 子块 (x=1, y=0)
    let child = parent.upsample(0, 0, 0, 1, 0, 1);

    // SE 子块覆盖父块的 [0.5, 1.0] x [0, 0.5]
    // 子角点 (0,0) = 父 (0.5, 0) → h=1.0
    assert!((child.get_height(0, 0).unwrap() - 1.0).abs() < 1e-10);
    // 子角点 (2,0) = 父 (1.0, 0) → h=2.0
    assert!((child.get_height(2, 0).unwrap() - 2.0).abs() < 1e-10);
    // 子角点 (0,2) = 父 (0.5, 0.5) → h=11.0
    assert!((child.get_height(0, 2).unwrap() - 11.0).abs() < 1e-10);
    // 子角点 (2,2) = 父 (1.0, 0.5) → h=12.0
    assert!((child.get_height(2, 2).unwrap() - 12.0).abs() < 1e-10);
}

#[test]
fn heightmap_upsample_northwest_child() {
    let heights = vec![
        0.0, 1.0, 2.0,
        10.0, 11.0, 12.0,
        20.0, 21.0, 22.0,
    ];
    let parent = HeightmapTerrainData::new(heights, 3, 3, 0.0, 22.0);

    // NW 子块 (x=0, y=1)
    let child = parent.upsample(0, 0, 0, 0, 1, 1);

    // NW 子块覆盖父块的 [0, 0.5] x [0.5, 1.0]
    // 子角点 (0,0) = 父 (0, 0.5) → col_f=0, row_f=1 → h=10.0
    assert!((child.get_height(0, 0).unwrap() - 10.0).abs() < 1e-10);
    // 子角点 (2,0) = 父 (0.5, 0.5) → col_f=1, row_f=1 → h=11.0
    assert!((child.get_height(2, 0).unwrap() - 11.0).abs() < 1e-10);
    // 子角点 (0,2) = 父 (0, 1.0) → col_f=0, row_f=2 → h=20.0
    assert!((child.get_height(0, 2).unwrap() - 20.0).abs() < 1e-10);
    // 子角点 (2,2) = 父 (0.5, 1.0) → col_f=1, row_f=2 → h=21.0
    assert!((child.get_height(2, 2).unwrap() - 21.0).abs() < 1e-10);
}

#[test]
fn heightmap_upsample_northeast_child() {
    let heights = vec![
        0.0, 1.0, 2.0,
        10.0, 11.0, 12.0,
        20.0, 21.0, 22.0,
    ];
    let parent = HeightmapTerrainData::new(heights, 3, 3, 0.0, 22.0);

    // NE 子块 (x=1, y=1)
    let child = parent.upsample(0, 0, 0, 1, 1, 1);

    // NE 子块覆盖父块的 [0.5, 1.0] x [0.5, 1.0]
    // 子角点 (0,0) = 父 (0.5, 0.5) → col_f=1, row_f=1 → h=11.0
    assert!((child.get_height(0, 0).unwrap() - 11.0).abs() < 1e-10);
    // 子角点 (2,0) = 父 (1.0, 0.5) → col_f=2, row_f=1 → h=12.0
    assert!((child.get_height(2, 0).unwrap() - 12.0).abs() < 1e-10);
    // 子角点 (0,2) = 父 (0.5, 1.0) → col_f=1, row_f=2 → h=21.0
    assert!((child.get_height(0, 2).unwrap() - 21.0).abs() < 1e-10);
    // 子角点 (2,2) = 父 (1.0, 1.0) → col_f=2, row_f=2 → h=22.0
    assert!((child.get_height(2, 2).unwrap() - 22.0).abs() < 1e-10);
}

#[test]
fn heightmap_upsample_preserves_dimensions() {
    let heights = vec![0.0; 12]; // 4x3
    let parent = HeightmapTerrainData::new(heights, 4, 3, 0.0, 0.0);
    let child = parent.upsample(0, 0, 0, 0, 0, 1);
    assert_eq!(child.width, 4);
    assert_eq!(child.height, 3);
    assert_eq!(child.heights.len(), 12);
}

#[test]
#[should_panic(expected = "upsample can only cross one level")]
fn heightmap_upsample_rejects_multi_level() {
    let heights = vec![0.0; 4];
    let parent = HeightmapTerrainData::new(heights, 2, 2, 0.0, 0.0);
    parent.upsample(0, 0, 0, 0, 0, 2); // 层差 = 2
}
