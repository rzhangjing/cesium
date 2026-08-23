//! Tests for `cesium_core::TileAvailability`.

use cesium_core::cartographic::Cartographic;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::tile_availability::TileAvailability;

#[test]
fn new_creates_empty_availability() {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let ta = TileAvailability::new(Box::new(ts), 10);
    let pos = Cartographic::new(0.0, 0.0, 0.0);
    assert_eq!(ta.compute_maximum_level_at_position(&pos), -1);
}

#[test]
fn add_available_tile_range_makes_tile_available() {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut ta = TileAvailability::new(Box::new(ts), 5);
    ta.add_available_tile_range(0, 0, 0, 0, 0);
    let pos = Cartographic::new(0.0, 0.0, 0.0);
    assert!(ta.compute_maximum_level_at_position(&pos) >= 0);
}

#[test]
fn is_tile_available_returns_true_for_added_range() {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut ta = TileAvailability::new(Box::new(ts), 5);
    ta.add_available_tile_range(0, 0, 0, 1, 1);
    assert!(ta.is_tile_available(0, 0, 0));
}

#[test]
fn compute_child_mask_returns_nonnegative() {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut ta = TileAvailability::new(Box::new(ts), 5);
    ta.add_available_tile_range(0, 0, 0, 1, 1);
    ta.add_available_tile_range(1, 0, 0, 1, 1);
    let mask = ta.compute_child_mask_for_tile(0, 0, 0);
    // Just verify it returns a valid u8 mask
    assert!(mask <= 15);
}
