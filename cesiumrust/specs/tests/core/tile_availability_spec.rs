//! TileAvailability specs - ported from Core/TileAvailabilitySpec.js
//!
//! Original: 12 it() tests. Ported: 11 A-class.
//! Omitted: 1 (internal _rootNodes structure check → adapted to functional test).
//!
//! Notes:
//! - `checkNodeRectanglesSorted` (internal quadtree structure validation) is
//!   C-class (accesses private _rootNodes/_ne/_se/_nw/_sw); replaced with a
//!   functional equivalent verifying order-independence of addAvailableTileRange.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::rectangle::Rectangle;
use cesium_provider::tiling_scheme::{GeographicTilingScheme, TileAvailability, TilingScheme, WebMercatorTilingScheme};
use std::f64::consts::PI;

fn create_availability(scheme: TilingScheme, maximum_level: u32) -> TileAvailability {
    let x_tiles = scheme.number_of_x_tiles_at_level(0);
    let y_tiles = scheme.number_of_y_tiles_at_level(0);
    let mut availability = TileAvailability::new(scheme, maximum_level);
    availability.add_available_tile_range(0, 0, 0, x_tiles - 1, y_tiles - 1);
    availability
}

fn geographic() -> TilingScheme {
    TilingScheme::Geographic(GeographicTilingScheme::new())
}

fn web_mercator() -> TilingScheme {
    TilingScheme::WebMercator(WebMercatorTilingScheme::new())
}

// ─── computeMaximumLevelAtPosition ──────────────────────────────────────────

#[test]
fn max_level_returns_minus1_if_position_outside_tiling_scheme() {
    let availability = create_availability(web_mercator(), 15);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_degrees(25.0, 88.0, 0.0)),
        -1
    );
}

#[test]
fn max_level_returns_0_if_there_are_no_rectangles() {
    let availability = create_availability(geographic(), 15);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_degrees(25.0, 88.0, 0.0)),
        0
    );
}

#[test]
fn max_level_returns_higher_level_when_on_boundary_at_level_0() {
    let mut availability = create_availability(geographic(), 15);
    availability.add_available_tile_range(0, 0, 0, 0, 0);
    availability.add_available_tile_range(1, 1, 0, 1, 0);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_radians(0.0, 0.0, 0.0)),
        1
    );

    // Make sure it isn't dependent on the order we add the rectangles.
    let mut availability = create_availability(geographic(), 15);
    availability.add_available_tile_range(1, 1, 0, 1, 0);
    availability.add_available_tile_range(0, 0, 0, 0, 0);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_radians(0.0, 0.0, 0.0)),
        1
    );
}

#[test]
fn max_level_returns_higher_level_when_on_boundary_at_level_1() {
    let mut availability = create_availability(geographic(), 15);
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    availability.add_available_tile_range(1, 1, 1, 1, 1);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_radians(-PI / 2.0, 0.0, 0.0)),
        1
    );
}

// ─── computeBestAvailableLevelOverRectangle ─────────────────────────────────

#[test]
fn best_level_returns_0_if_there_are_no_rectangles() {
    let availability = create_availability(geographic(), 15);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&Rectangle::from_degrees(1.0, 2.0, 3.0, 4.0)),
        0
    );
}

#[test]
fn best_level_reports_correct_level_when_entirely_inside_worldwide_rectangle() {
    let scheme = geographic();
    let mut availability = create_availability(scheme.clone(), 15);
    availability.add_available_tile_range(
        5, 0, 0,
        scheme.number_of_x_tiles_at_level(5) - 1,
        scheme.number_of_y_tiles_at_level(5) - 1,
    );
    availability.add_available_tile_range(6, 7, 8, 9, 10);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&Rectangle::from_degrees(1.0, 2.0, 3.0, 4.0)),
        5
    );
}

#[test]
fn best_level_reports_correct_level_when_entirely_inside_smaller_rectangle() {
    let scheme = geographic();
    let mut availability = create_availability(scheme.clone(), 15);
    availability.add_available_tile_range(
        5, 0, 0,
        scheme.number_of_x_tiles_at_level(5) - 1,
        scheme.number_of_y_tiles_at_level(5) - 1,
    );
    availability.add_available_tile_range(6, 7, 8, 9, 10);

    let geo = match &scheme {
        TilingScheme::Geographic(g) => g,
        _ => unreachable!(),
    };
    let rectangle = geo.tile_xy_to_rectangle(8, 9, 6);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&rectangle),
        6
    );
}

#[test]
fn best_level_reports_correct_level_when_partially_overlapping() {
    let scheme = geographic();
    let mut availability = create_availability(scheme.clone(), 15);
    availability.add_available_tile_range(
        5, 0, 0,
        scheme.number_of_x_tiles_at_level(5) - 1,
        scheme.number_of_y_tiles_at_level(5) - 1,
    );
    availability.add_available_tile_range(6, 7, 8, 7, 8);

    let geo = match &scheme {
        TilingScheme::Geographic(g) => g,
        _ => unreachable!(),
    };
    let mut rectangle = geo.tile_xy_to_rectangle(7, 8, 6);
    rectangle.west -= 0.01;
    rectangle.east += 0.01;
    rectangle.south -= 0.01;
    rectangle.north += 0.01;
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&rectangle),
        5
    );
}

#[test]
fn best_level_works_with_rectangle_crossing_180_degrees_longitude() {
    let scheme = geographic();
    let mut availability = create_availability(scheme.clone(), 15);
    availability.add_available_tile_range(
        5, 0, 0,
        scheme.number_of_x_tiles_at_level(5) - 1,
        scheme.number_of_y_tiles_at_level(5) - 1,
    );
    availability.add_available_tile_range(
        6, 0, 0, 10,
        scheme.number_of_y_tiles_at_level(6) - 1,
    );
    availability.add_available_tile_range(
        6,
        scheme.number_of_x_tiles_at_level(6) - 11, 0,
        scheme.number_of_x_tiles_at_level(6) - 1,
        scheme.number_of_y_tiles_at_level(6) - 1,
    );

    let rectangle = Rectangle::from_degrees(179.0, 45.0, -179.0, 50.0);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&rectangle),
        6
    );

    let rectangle = Rectangle::from_degrees(45.0, 45.0, -45.0, 50.0);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&rectangle),
        5
    );
}

#[test]
fn best_level_works_when_four_rectangles_combine_to_cover_area() {
    let scheme = geographic();
    let mut availability = create_availability(scheme.clone(), 15);
    availability.add_available_tile_range(
        5, 0, 0,
        scheme.number_of_x_tiles_at_level(5) - 1,
        scheme.number_of_y_tiles_at_level(5) - 1,
    );
    availability.add_available_tile_range(6, 0, 2, 1, 3);
    availability.add_available_tile_range(6, 2, 0, 3, 1);
    availability.add_available_tile_range(6, 0, 0, 1, 1);
    availability.add_available_tile_range(6, 2, 2, 3, 3);

    let geo = match &scheme {
        TilingScheme::Geographic(g) => g,
        _ => unreachable!(),
    };
    let rectangle = geo.tile_xy_to_rectangle(0, 0, 4);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&rectangle),
        6
    );
}

// ─── addAvailableTileRange ──────────────────────────────────────────────────

#[test]
fn add_range_keeps_availability_sorted_by_level() {
    let mut availability = create_availability(geographic(), 15);
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    availability.add_available_tile_range(1, 0, 0, 3, 1);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_radians(-PI / 2.0, 0.0, 0.0)),
        1
    );

    // We should get the same result adding them in the opposite order.
    let mut availability = create_availability(geographic(), 15);
    availability.add_available_tile_range(1, 0, 0, 3, 1);
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_radians(-PI / 2.0, 0.0, 0.0)),
        1
    );
}

#[test]
fn add_range_boundary_rectangles_sorted_properly() {
    // Adapted from original: instead of checking internal node structure,
    // verify functional correctness of boundary-sorted rectangles.
    let mut availability = TileAvailability::new(geographic(), 6);
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    availability.add_available_tile_range(1, 0, 0, 2, 0);
    availability.add_available_tile_range(2, 0, 0, 4, 0);
    availability.add_available_tile_range(3, 0, 0, 8, 0);
    availability.add_available_tile_range(0, 0, 0, 1, 0);

    // All levels should be available at a position covered by all ranges.
    // Level 3 range covers x=0..8, y=0 → longitude -180..22.5E, latitude 67.5..90N
    let pos = Cartographic::from_degrees(-90.0, 78.75, 0.0);
    assert_eq!(availability.compute_maximum_level_at_position(&pos), 3);

    // Position only covered up to level 2 (45E,45N is at edge of level-2 range)
    let pos2 = Cartographic::from_degrees(22.5, 56.25, 0.0);
    assert_eq!(availability.compute_maximum_level_at_position(&pos2), 2);

    // isTileAvailable should work for all added levels
    assert!(availability.is_tile_available(0, 0, 0));
    assert!(availability.is_tile_available(1, 1, 0));
    assert!(availability.is_tile_available(2, 2, 0));
    assert!(availability.is_tile_available(3, 5, 0));
    assert!(!availability.is_tile_available(4, 0, 0));
}

#[test]
fn compute_child_mask_for_tile_works() {
    let mut availability = create_availability(geographic(), 15);
    availability.add_available_tile_range(1, 0, 0, 3, 1);

    // Level 0 tile (0,0): all four children at level 1 should be available
    // NW=(0,0), NE=(1,0), SW=(0,1), SE=(1,1)
    let mask = availability.compute_child_mask_for_tile(0, 0, 0);
    assert_eq!(mask, 0b1111);

    // Level 0 tile (1,0): children are (2,0),(3,0),(2,1),(3,1) at level 1
    let mask = availability.compute_child_mask_for_tile(0, 1, 0);
    assert_eq!(mask, 0b1111);

    // At maximum level, mask should be 0
    let mask = availability.compute_child_mask_for_tile(15, 0, 0);
    assert_eq!(mask, 0);
}
