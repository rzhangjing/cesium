//! Terrain height sampling specs - ported from Core/sampleTerrain*.js
//!
//! Tests bilinear interpolation and quantized mesh height sampling.

use cesium_provider::terrain_provider::{
    sample_height_bilinear, sample_height_quantized, HeightmapSampleParams, QuantizedSampleParams,
};

fn make_heightmap_params<'a>(
    heightmap: &'a [f64],
    grid_width: usize,
    grid_height: usize,
) -> HeightmapSampleParams<'a> {
    HeightmapSampleParams {
        heightmap,
        grid_width,
        grid_height,
        tile_west: 0.0,
        tile_south: 0.0,
        tile_east: 1.0,
        tile_north: 1.0,
        min_height: -1000.0,
        max_height: 9000.0,
    }
}

// ─── sample_height_bilinear ────────────────────────────────────────────────

#[test]
fn bilinear_uniform_grid() {
    // All heights = 100.0
    let data = vec![100.0; 9]; // 3x3
    let params = make_heightmap_params(&data, 3, 3);
    let h = sample_height_bilinear(&params, 0.5, 0.5).unwrap();
    assert!((h - 100.0).abs() < 1e-10);
}

#[test]
fn bilinear_center_of_4_corners() {
    // 2x2 grid: [0, 100; 100, 200]
    let data = vec![0.0, 100.0, 100.0, 200.0];
    let params = make_heightmap_params(&data, 2, 2);
    // Center (0.5, 0.5) → average of all 4 = 100
    let h = sample_height_bilinear(&params, 0.5, 0.5).unwrap();
    assert!((h - 100.0).abs() < 1e-10);
}

#[test]
fn bilinear_at_grid_corner_nw() {
    // 2x2 grid: [10, 20; 30, 40]
    let data = vec![10.0, 20.0, 30.0, 40.0];
    let params = make_heightmap_params(&data, 2, 2);
    // NW corner: longitude=0 (west), latitude=1 (north) → row 0, col 0
    let h = sample_height_bilinear(&params, 0.0, 1.0).unwrap();
    assert!((h - 10.0).abs() < 1e-10);
}

#[test]
fn bilinear_at_grid_corner_ne() {
    let data = vec![10.0, 20.0, 30.0, 40.0];
    let params = make_heightmap_params(&data, 2, 2);
    // NE corner: longitude=1 (east), latitude=1 (north) → row 0, col 1
    let h = sample_height_bilinear(&params, 1.0, 1.0).unwrap();
    assert!((h - 20.0).abs() < 1e-10);
}

#[test]
fn bilinear_at_grid_corner_sw() {
    let data = vec![10.0, 20.0, 30.0, 40.0];
    let params = make_heightmap_params(&data, 2, 2);
    // SW corner: longitude=0 (west), latitude=0 (south) → row 1, col 0
    let h = sample_height_bilinear(&params, 0.0, 0.0).unwrap();
    assert!((h - 30.0).abs() < 1e-10);
}

#[test]
fn bilinear_at_grid_corner_se() {
    let data = vec![10.0, 20.0, 30.0, 40.0];
    let params = make_heightmap_params(&data, 2, 2);
    // SE corner: longitude=1 (east), latitude=0 (south) → row 1, col 1
    let h = sample_height_bilinear(&params, 1.0, 0.0).unwrap();
    assert!((h - 40.0).abs() < 1e-10);
}

#[test]
fn bilinear_midpoint_east_edge() {
    // 2x2: [0, 100; 0, 100] → midpoint of east edge = 100
    let data = vec![0.0, 100.0, 0.0, 100.0];
    let params = make_heightmap_params(&data, 2, 2);
    let h = sample_height_bilinear(&params, 1.0, 0.5).unwrap();
    assert!((h - 100.0).abs() < 1e-10);
}

#[test]
fn bilinear_quarter_position() {
    // 2x2: [0, 100; 0, 100] → x=0.25 from west → 25% interpolation
    let data = vec![0.0, 100.0, 0.0, 100.0];
    let params = make_heightmap_params(&data, 2, 2);
    let h = sample_height_bilinear(&params, 0.25, 0.5).unwrap();
    assert!((h - 25.0).abs() < 1e-10);
}

#[test]
fn bilinear_out_of_bounds_west() {
    let data = vec![100.0; 4];
    let params = make_heightmap_params(&data, 2, 2);
    assert!(sample_height_bilinear(&params, -0.1, 0.5).is_none());
}

#[test]
fn bilinear_out_of_bounds_east() {
    let data = vec![100.0; 4];
    let params = make_heightmap_params(&data, 2, 2);
    assert!(sample_height_bilinear(&params, 1.1, 0.5).is_none());
}

#[test]
fn bilinear_out_of_bounds_south() {
    let data = vec![100.0; 4];
    let params = make_heightmap_params(&data, 2, 2);
    assert!(sample_height_bilinear(&params, 0.5, -0.1).is_none());
}

#[test]
fn bilinear_out_of_bounds_north() {
    let data = vec![100.0; 4];
    let params = make_heightmap_params(&data, 2, 2);
    assert!(sample_height_bilinear(&params, 0.5, 1.1).is_none());
}

#[test]
fn bilinear_clamps_to_min_max() {
    // Heights exceed max_height
    let data = vec![15000.0, 15000.0, 15000.0, 15000.0];
    let params = make_heightmap_params(&data, 2, 2);
    let h = sample_height_bilinear(&params, 0.5, 0.5).unwrap();
    assert!((h - 9000.0).abs() < 1e-10); // clamped to max_height
}

#[test]
fn bilinear_clamps_to_min() {
    let data = vec![-5000.0, -5000.0, -5000.0, -5000.0];
    let params = make_heightmap_params(&data, 2, 2);
    let h = sample_height_bilinear(&params, 0.5, 0.5).unwrap();
    assert!((h - (-1000.0)).abs() < 1e-10); // clamped to min_height
}

#[test]
fn bilinear_3x3_center() {
    // 3x3 grid with center = 500
    #[rustfmt::skip]
    let data = vec![
        100.0, 200.0, 300.0,
        400.0, 500.0, 600.0,
        700.0, 800.0, 900.0,
    ];
    let params = make_heightmap_params(&data, 3, 3);
    // Center of tile = center of grid
    let h = sample_height_bilinear(&params, 0.5, 0.5).unwrap();
    assert!((h - 500.0).abs() < 1e-10);
}

#[test]
fn bilinear_insufficient_data() {
    let data = vec![100.0; 2]; // Too small for 3x3
    let params = make_heightmap_params(&data, 3, 3);
    assert!(sample_height_bilinear(&params, 0.5, 0.5).is_none());
}

// ─── sample_height_quantized ───────────────────────────────────────────────

fn make_quantized_params<'a>(
    vertices: &'a [u16],
    vertex_count: usize,
) -> QuantizedSampleParams<'a> {
    QuantizedSampleParams {
        quantized_vertices: vertices,
        vertex_count,
        tile_west: 0.0,
        tile_south: 0.0,
        tile_east: 1.0,
        tile_north: 1.0,
        min_height: 0.0,
        max_height: 1000.0,
    }
}

#[test]
fn quantized_single_vertex() {
    // 1 vertex at center (u=16383, v=16383, h=16383 → 500m)
    let vertices = vec![16383u16, 16383, 16383];
    let params = make_quantized_params(&vertices, 1);
    let h = sample_height_quantized(&params, 0.5, 0.5).unwrap();
    assert!((h - 500.0).abs() < 1.0); // ~500m (16383/32767 * 1000)
}

#[test]
fn quantized_nearest_vertex_selection() {
    // 2 vertices: one at (0,0) with h=0, one at (32767,32767) with h=32767
    // Layout: [u0, u1, v0, v1, h0, h1]
    let vertices = vec![0u16, 32767, 0, 32767, 0, 32767];
    let params = make_quantized_params(&vertices, 2);
    // Query near (0,0) → should pick vertex 0 → height ≈ 0
    let h = sample_height_quantized(&params, 0.01, 0.01).unwrap();
    assert!(h < 50.0);
    // Query near (1,1) → should pick vertex 1 → height ≈ 1000
    let h = sample_height_quantized(&params, 0.99, 0.99).unwrap();
    assert!(h > 950.0);
}

#[test]
fn quantized_dequantize_height() {
    // 1 vertex with h=32767 → max height
    let vertices = vec![16383u16, 16383, 32767];
    let params = make_quantized_params(&vertices, 1);
    let h = sample_height_quantized(&params, 0.5, 0.5).unwrap();
    assert!((h - 1000.0).abs() < 1.0);
}

#[test]
fn quantized_zero_height() {
    // 1 vertex with h=0 → min height
    let vertices = vec![16383u16, 16383, 0];
    let params = make_quantized_params(&vertices, 1);
    let h = sample_height_quantized(&params, 0.5, 0.5).unwrap();
    assert!(h.abs() < 1.0);
}

#[test]
fn quantized_insufficient_data() {
    let vertices = vec![100u16, 200]; // Too small for 2 vertices (need 6)
    let params = make_quantized_params(&vertices, 2);
    assert!(sample_height_quantized(&params, 0.5, 0.5).is_none());
}

#[test]
fn quantized_three_vertices() {
    // 3 vertices at different positions
    // u: [0, 16383, 32767], v: [0, 16383, 32767], h: [0, 16383, 32767]
    let vertices = vec![0u16, 16383, 32767, 0, 16383, 32767, 0, 16383, 32767];
    let params = make_quantized_params(&vertices, 3);
    // Query at center → nearest to vertex 1 (16383,16383) → h ≈ 500
    let h = sample_height_quantized(&params, 0.5, 0.5).unwrap();
    assert!((h - 500.0).abs() < 20.0);
}
