//! Core/GeographicTilingSchemeSpec.js, WebMercatorTilingSchemeSpec.js, TileAvailabilitySpec.js
//! → Rust integration tests

use cesium_geospatial::rectangle::Rectangle;
use cesium_provider::tiling_scheme::{GeographicTilingScheme, WebMercatorTilingScheme};
use cesium_specs::{assert_approx, epsilon};
use std::f64::consts::PI;

// === GeographicTilingScheme ===

#[test]
fn test_geographic_tiling_scheme_default() {
    let scheme = GeographicTilingScheme::new();
    assert_eq!(scheme.number_of_level_zero_tiles_x, 2);
    assert_eq!(scheme.number_of_level_zero_tiles_y, 1);
}

#[test]
fn test_geographic_tiling_scheme_rectangle() {
    let scheme = GeographicTilingScheme::new();
    // Should cover the full globe
    assert_approx!(scheme.rectangle.west, -PI, epsilon::EPSILON10);
    assert_approx!(scheme.rectangle.east, PI, epsilon::EPSILON10);
    assert_approx!(scheme.rectangle.south, -PI / 2.0, epsilon::EPSILON10);
    assert_approx!(scheme.rectangle.north, PI / 2.0, epsilon::EPSILON10);
}

#[test]
fn test_geographic_number_of_tiles_at_level() {
    let scheme = GeographicTilingScheme::new();
    assert_eq!(scheme.number_of_x_tiles_at_level(0), 2);
    assert_eq!(scheme.number_of_y_tiles_at_level(0), 1);
    assert_eq!(scheme.number_of_x_tiles_at_level(1), 4);
    assert_eq!(scheme.number_of_y_tiles_at_level(1), 2);
    assert_eq!(scheme.number_of_x_tiles_at_level(2), 8);
    assert_eq!(scheme.number_of_y_tiles_at_level(2), 4);
}

#[test]
fn test_geographic_tile_xy_to_rectangle_level0() {
    let scheme = GeographicTilingScheme::new();
    // Tile (0,0) at level 0 should be the western hemisphere
    let rect = scheme.tile_xy_to_rectangle(0, 0, 0);
    assert_approx!(rect.west, -PI, epsilon::EPSILON10);
    assert_approx!(rect.east, 0.0, epsilon::EPSILON10);
    assert_approx!(rect.south, -PI / 2.0, epsilon::EPSILON10);
    assert_approx!(rect.north, PI / 2.0, epsilon::EPSILON10);
}

#[test]
fn test_geographic_tile_xy_to_rectangle_level0_east() {
    let scheme = GeographicTilingScheme::new();
    // Tile (1,0) at level 0 should be the eastern hemisphere
    let rect = scheme.tile_xy_to_rectangle(1, 0, 0);
    assert_approx!(rect.west, 0.0, epsilon::EPSILON10);
    assert_approx!(rect.east, PI, epsilon::EPSILON10);
}

#[test]
fn test_geographic_position_to_tile_xy() {
    let scheme = GeographicTilingScheme::new();
    // Position at (0, 0) should be in tile (1, 0) at level 0 (eastern hemisphere)
    let coord = scheme.position_to_tile_xy(0.001, 0.0, 0).unwrap();
    assert_eq!(coord.x, 1);
    assert_eq!(coord.y, 0);
}

#[test]
fn test_geographic_position_to_tile_xy_western() {
    let scheme = GeographicTilingScheme::new();
    // Position at (-90°, 0) should be in tile (0, 0) at level 0
    let coord = scheme.position_to_tile_xy(-PI / 2.0, 0.0, 0).unwrap();
    assert_eq!(coord.x, 0);
    assert_eq!(coord.y, 0);
}

#[test]
fn test_geographic_with_options() {
    let rect = Rectangle::new(-PI, -PI / 2.0, PI, PI / 2.0);
    let scheme = GeographicTilingScheme::with_options(rect, 4, 2);
    assert_eq!(scheme.number_of_level_zero_tiles_x, 4);
    assert_eq!(scheme.number_of_level_zero_tiles_y, 2);
    assert_eq!(scheme.number_of_x_tiles_at_level(0), 4);
    assert_eq!(scheme.number_of_y_tiles_at_level(0), 2);
}

#[test]
fn test_geographic_native_rectangle() {
    let scheme = GeographicTilingScheme::new();
    let rect = scheme.tile_xy_to_rectangle(0, 0, 0);
    let native = scheme.rectangle_to_native_rectangle(&rect);
    // Native should be in degrees
    assert_approx!(native.west, -180.0, epsilon::EPSILON6);
    assert_approx!(native.east, 0.0, epsilon::EPSILON6);
}

// === WebMercatorTilingScheme ===

#[test]
fn test_web_mercator_tiling_scheme_default() {
    let scheme = WebMercatorTilingScheme::new();
    assert_eq!(scheme.number_of_level_zero_tiles_x, 1);
    assert_eq!(scheme.number_of_level_zero_tiles_y, 1);
}

#[test]
fn test_web_mercator_number_of_tiles_at_level() {
    let scheme = WebMercatorTilingScheme::new();
    assert_eq!(scheme.number_of_x_tiles_at_level(0), 1);
    assert_eq!(scheme.number_of_y_tiles_at_level(0), 1);
    assert_eq!(scheme.number_of_x_tiles_at_level(1), 2);
    assert_eq!(scheme.number_of_y_tiles_at_level(1), 2);
    assert_eq!(scheme.number_of_x_tiles_at_level(2), 4);
    assert_eq!(scheme.number_of_y_tiles_at_level(2), 4);
}

#[test]
fn test_web_mercator_project_unproject_roundtrip() {
    let lon = 0.5; // radians
    let lat = 0.3; // radians
    let (x, y) = WebMercatorTilingScheme::project(lon, lat);
    let (lon2, lat2) = WebMercatorTilingScheme::unproject(x, y);
    assert_approx!(lon2, lon, epsilon::EPSILON10);
    assert_approx!(lat2, lat, epsilon::EPSILON10);
}

#[test]
fn test_web_mercator_project_origin() {
    let (x, y) = WebMercatorTilingScheme::project(0.0, 0.0);
    assert_approx!(x, 0.0, epsilon::EPSILON6);
    assert_approx!(y, 0.0, epsilon::EPSILON6);
}

#[test]
fn test_web_mercator_tile_xy_to_rectangle_level0() {
    let scheme = WebMercatorTilingScheme::new();
    let rect = scheme.tile_xy_to_rectangle(0, 0, 0);
    // Level 0 tile should cover the full Mercator extent
    assert_approx!(rect.west, -PI, epsilon::EPSILON6);
    assert_approx!(rect.east, PI, epsilon::EPSILON6);
}

#[test]
fn test_web_mercator_position_to_tile_xy() {
    let scheme = WebMercatorTilingScheme::new();
    // At level 1, position (0, 0) should be in tile (1, 1) (bottom-right quadrant)
    let coord = scheme.position_to_tile_xy(0.001, -0.001, 1).unwrap();
    assert_eq!(coord.x, 1);
    assert_eq!(coord.y, 1);
}

#[test]
fn test_web_mercator_maximum_latitude() {
    let scheme = WebMercatorTilingScheme::new();
    // Max latitude should be ~85.05 degrees
    let max_lat_deg = scheme.rectangle.north.to_degrees();
    assert!(max_lat_deg > 85.0 && max_lat_deg < 85.1);
}
