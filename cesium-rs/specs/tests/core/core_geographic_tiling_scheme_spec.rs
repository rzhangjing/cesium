//! Tests for `cesium_core::GeographicTilingScheme`.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::rectangle::Rectangle;
use cesium_core::tiling_scheme::TilingScheme;

const EPSILON8: f64 = 1e-8;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

#[test]
fn default_constructor() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    assert!(scheme.ellipsoid() == &Ellipsoid::WGS84);
    assert!(approx_eq(scheme.rectangle().west, -std::f64::consts::PI, EPSILON8));
    assert!(approx_eq(scheme.rectangle().east, std::f64::consts::PI, EPSILON8));
}

#[test]
fn number_of_tiles_at_level_zero() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    assert_eq!(scheme.get_number_of_x_tiles_at_level(0), 2);
    assert_eq!(scheme.get_number_of_y_tiles_at_level(0), 1);
}

#[test]
fn number_of_tiles_doubles_each_level() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    assert_eq!(scheme.get_number_of_x_tiles_at_level(1), 4);
    assert_eq!(scheme.get_number_of_y_tiles_at_level(1), 2);
    assert_eq!(scheme.get_number_of_x_tiles_at_level(2), 8);
    assert_eq!(scheme.get_number_of_y_tiles_at_level(2), 4);
}

#[test]
fn tile_xy_to_rectangle_level_zero() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut result = Rectangle::default();
    // Tile (0, 0) at level 0 should cover the left half of the globe
    scheme.tile_xy_to_rectangle(0, 0, 0, &mut result);
    assert!(approx_eq(result.west, -std::f64::consts::PI, EPSILON8));
    assert!(approx_eq(result.east, 0.0, EPSILON8));
    assert!(approx_eq(result.north, std::f64::consts::FRAC_PI_2, EPSILON8));
    assert!(approx_eq(result.south, -std::f64::consts::FRAC_PI_2, EPSILON8));
}

#[test]
fn position_to_tile_xy_at_level_zero() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut result = Cartesian2::default();
    // Position at (0, 0) should be in tile (1, 0) at level 0 (right half)
    let pos = Cartographic::new(0.0, 0.0, 0.0);
    let ok = scheme.position_to_tile_xy(&pos, 0, &mut result);
    assert!(ok.is_some());
    assert_eq!(result.x as i32, 1);
    assert_eq!(result.y as i32, 0);
}

#[test]
fn position_to_tile_xy_outside_rectangle_returns_none() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut result = Cartesian2::default();
    // Position outside the valid rectangle
    let pos = Cartographic::new(10.0, 5.0, 0.0);
    let ok = scheme.position_to_tile_xy(&pos, 0, &mut result);
    assert!(ok.is_none());
}

#[test]
fn custom_level_zero_tiles() {
    let scheme = GeographicTilingScheme::new(None, None, Some(4), Some(2));
    assert_eq!(scheme.get_number_of_x_tiles_at_level(0), 4);
    assert_eq!(scheme.get_number_of_y_tiles_at_level(0), 2);
}

#[test]
fn rectangle_to_native_converts_to_degrees() {
    let scheme = GeographicTilingScheme::new(None, None, None, None);
    let rect = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    let mut result = Rectangle::default();
    scheme.rectangle_to_native_rectangle(&rect, &mut result);
    // Should convert from radians to degrees
    assert!(result.west.abs() < 1e-6);
    assert!(result.south.abs() < 1e-6);
    assert!(approx_eq(result.east, 1.0_f64.to_degrees(), 1e-6));
    assert!(approx_eq(result.north, 1.0_f64.to_degrees(), 1e-6));
}
