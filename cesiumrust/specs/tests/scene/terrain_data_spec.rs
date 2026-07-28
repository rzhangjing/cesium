//! Core/HeightmapTerrainData + QuantizedMeshTerrainData → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Core/HeightmapTerrainData.js
//! - Core/QuantizedMeshTerrainData.js
//!
//! A-class tests: heightmap get/interpolate/create_mesh/child_mask,
//! quantized mesh vertex accessors/create_mesh/skirts/child_mask.
//! C-class omitted: Worker creation, ArrayBuffer transfer, upsampling (needs full pipeline).

use cesium_terrain::{HeightmapTerrainData, QuantizedMeshTerrainData, TerrainMesh, MAX_SHORT};
use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;

// === HeightmapTerrainData ===

fn make_heightmap() -> HeightmapTerrainData {
    // 4x3 heightmap (width=4, height=3)
    let heights = vec![
        0.0, 100.0, 200.0, 300.0,   // row 0 (south)
        50.0, 150.0, 250.0, 350.0,  // row 1 (middle)
        100.0, 200.0, 300.0, 400.0, // row 2 (north)
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
    // (0,0) = SW corner = heights[0] = 0.0
    assert!((data.interpolate_height(0.0, 0.0) - 0.0).abs() < 0.01);
    // (1,0) = SE corner = heights[3] = 300.0
    assert!((data.interpolate_height(1.0, 0.0) - 300.0).abs() < 0.01);
    // (0,1) = NW corner = heights[8] = 100.0
    assert!((data.interpolate_height(0.0, 1.0) - 100.0).abs() < 0.01);
    // (1,1) = NE corner = heights[11] = 400.0
    assert!((data.interpolate_height(1.0, 1.0) - 400.0).abs() < 0.01);
}

#[test]
fn heightmap_interpolate_midpoint() {
    let data = make_heightmap();
    // Center of the grid: bilinear interpolation
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

    // 4*3 = 12 vertices
    assert_eq!(mesh.positions.len(), 12);
    // (4-1)*(3-1)*2*3 = 3*2*6 = 36 indices
    assert_eq!(mesh.indices.len(), 36);
    assert!(mesh.normals.is_some());
    assert!(mesh.tex_coords.is_some());
}

#[test]
fn heightmap_child_mask() {
    let data = make_heightmap();
    // Default child_tile_mask = 15 (all 4 children)
    assert!(data.is_child_available(0));
    assert!(data.is_child_available(1));
    assert!(data.is_child_available(2));
    assert!(data.is_child_available(3));
}

#[test]
fn heightmap_partial_child_mask() {
    let mut data = make_heightmap();
    data.child_tile_mask = 0b0101; // Only children 0 and 2
    assert!(data.is_child_available(0));
    assert!(!data.is_child_available(1));
    assert!(data.is_child_available(2));
    assert!(!data.is_child_available(3));
}

// === QuantizedMeshTerrainData ===

fn make_quantized_mesh() -> QuantizedMeshTerrainData {
    // 4 vertices forming a quad
    QuantizedMeshTerrainData {
        quantized_vertices: vec![
            // u values: SW=0, SE=MAX, NW=0, NE=MAX
            0, MAX_SHORT, 0, MAX_SHORT,
            // v values: SW=0, SE=0, NW=MAX, NE=MAX
            0, 0, MAX_SHORT, MAX_SHORT,
            // height values: all mid-range
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
    data.child_tile_mask = 0b1010; // Children 1 and 3
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

    // All positions should be near the ellipsoid surface
    // Geocentric radius varies from ~6357km (pole) to ~6378km (equator)
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

    // More vertices due to skirts (4 base + skirt vertices)
    assert!(mesh.positions.len() > 4);
    // More indices due to skirt triangles
    assert!(mesh.indices.len() > 6);
}

#[test]
fn quantized_mesh_uv_coordinates() {
    let data = make_quantized_mesh();
    let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let ellipsoid = Ellipsoid::WGS84;
    let mesh = data.create_mesh(&rectangle, &ellipsoid, 1.0);

    let uvs = mesh.tex_coords.unwrap();
    // Vertex 0: u=0/MAX=0, v=0/MAX=0
    assert!((uvs[0][0] - 0.0).abs() < 1e-4);
    assert!((uvs[0][1] - 0.0).abs() < 1e-4);
    // Vertex 1: u=MAX/MAX=1, v=0/MAX=0
    assert!((uvs[1][0] - 1.0).abs() < 1e-4);
    assert!((uvs[1][1] - 0.0).abs() < 1e-4);
    // Vertex 2: u=0/MAX=0, v=MAX/MAX=1
    assert!((uvs[2][0] - 0.0).abs() < 1e-4);
    assert!((uvs[2][1] - 1.0).abs() < 1e-4);
}

#[test]
fn quantized_mesh_max_short_constant() {
    assert_eq!(MAX_SHORT, 32767);
}
