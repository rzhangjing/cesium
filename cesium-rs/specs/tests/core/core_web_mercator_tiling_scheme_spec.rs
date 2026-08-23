//! Tests for `cesium_core::WebMercatorTilingScheme`.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartographic::Cartographic;
use cesium_core::rectangle::Rectangle;
use cesium_core::tiling_scheme::TilingScheme;
use cesium_core::web_mercator_tiling_scheme::WebMercatorTilingScheme;

#[test]
fn default_has_one_tile_at_level_zero() {
    let ts = WebMercatorTilingScheme::new(None, None, None, None, None);
    assert_eq!(ts.get_number_of_x_tiles_at_level(0), 1);
    assert_eq!(ts.get_number_of_y_tiles_at_level(0), 1);
}

#[test]
fn level_zero_custom_tile_count() {
    let ts = WebMercatorTilingScheme::new(None, Some(2), Some(2), None, None);
    assert_eq!(ts.get_number_of_x_tiles_at_level(0), 2);
    assert_eq!(ts.get_number_of_y_tiles_at_level(1), 4);
}

#[test]
fn tile_xy_to_rectangle_returns_valid_rectangle() {
    let ts = WebMercatorTilingScheme::new(None, None, None, None, None);
    let mut rect = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
    ts.tile_xy_to_rectangle(0, 0, 0, &mut rect);
    assert!(rect.west < rect.east);
    assert!(rect.south < rect.north);
}

#[test]
fn position_to_tile_xy_returns_some_for_valid_position() {
    let ts = WebMercatorTilingScheme::new(None, None, None, None, None);
    let pos = Cartographic::new(0.0, 0.0, 0.0);
    let mut result = Cartesian2::default();
    let ok = ts.position_to_tile_xy(&pos, 1, &mut result);
    assert!(ok.is_some());
}

#[test]
fn rectangle_to_native_rectangle_produces_meter_bounds() {
    let ts = WebMercatorTilingScheme::new(None, None, None, None, None);
    let rect = Rectangle::from_radians(-1.0, -0.5, 1.0, 0.5);
    let mut result = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
    ts.rectangle_to_native_rectangle(&rect, &mut result);
    // Native rectangle should be in meters (much larger than radians)
    assert!(result.east > 100_000.0);
}
