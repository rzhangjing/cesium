//! Tests for `cesium_core::tile_availability::TileAvailability`.
//!
//! Mirrors `packages/engine/Specs/Core/TileAvailabilitySpec.js`.

use cesium_core::cartographic::Cartographic;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::rectangle::Rectangle;
use cesium_core::tile_availability::TileAvailability;
use cesium_core::tiling_scheme::TilingScheme;
use cesium_core::web_mercator_tiling_scheme::WebMercatorTilingScheme;

fn create_availability_geographic() -> TileAvailability {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut availability = TileAvailability::new(Box::new(ts), 15);
    let ts = GeographicTilingScheme::new(None, None, None, None);
    availability.add_available_tile_range(
        0,
        0,
        0,
        ts.get_number_of_x_tiles_at_level(0) - 1,
        ts.get_number_of_y_tiles_at_level(0) - 1,
    );
    availability
}

fn create_availability_web_mercator() -> TileAvailability {
    let ts = WebMercatorTilingScheme::new(None, None, None, None, None);
    let mut availability = TileAvailability::new(Box::new(ts), 15);
    let ts = WebMercatorTilingScheme::new(None, None, None, None, None);
    availability.add_available_tile_range(
        0,
        0,
        0,
        ts.get_number_of_x_tiles_at_level(0) - 1,
        ts.get_number_of_y_tiles_at_level(0) - 1,
    );
    availability
}

// Mirrors JS `geographic.getNumberOfXTilesAtLevel(n) - 1` / `getNumberOfYTilesAtLevel(n) - 1`
// used at the call sites below (geographic scheme: 2 x-tiles at level 0, 1 y-tile at level 0).
fn geo_x_at_level(level: i32) -> i32 {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    ts.get_number_of_x_tiles_at_level(level)
}
fn geo_y_at_level(level: i32) -> i32 {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    ts.get_number_of_y_tiles_at_level(level)
}

// =====================================================================
// computeMaximumLevelAtPosition
// =====================================================================

#[test]
fn returns_minus_1_if_position_outside_the_tiling_scheme() {
    let availability = create_availability_web_mercator();
    assert_eq!(
        availability
            .compute_maximum_level_at_position(&Cartographic::from_degrees_new(25.0, 88.0, None)),
        -1
    );
}

#[test]
fn returns_0_if_there_are_no_rectangles() {
    let availability = create_availability_geographic();
    assert_eq!(
        availability
            .compute_maximum_level_at_position(&Cartographic::from_degrees_new(25.0, 88.0, None)),
        0
    );
}

#[test]
fn returns_the_higher_level_when_on_a_boundary_at_level_0() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(0, 0, 0, 0, 0);
    availability.add_available_tile_range(1, 1, 0, 1, 0);
    assert_eq!(
        availability
            .compute_maximum_level_at_position(&Cartographic::from_radians_new(0.0, 0.0, None)),
        1
    );

    // Make sure it isn't dependent on the order we add the rectangles.
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(1, 1, 0, 1, 0);
    availability.add_available_tile_range(0, 0, 0, 0, 0);
    assert_eq!(
        availability
            .compute_maximum_level_at_position(&Cartographic::from_radians_new(0.0, 0.0, None)),
        1
    );
}

#[test]
fn returns_the_higher_level_when_on_a_boundary_at_level_1() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    availability.add_available_tile_range(1, 1, 1, 1, 1);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::from_radians_new(
            -std::f64::consts::FRAC_PI_2,
            0.0,
            None
        )),
        1
    );
}

// =====================================================================
// computeBestAvailableLevelOverRectangle
// =====================================================================

#[test]
fn returns_0_if_there_are_no_rectangles_over_rectangle() {
    let availability = create_availability_geographic();
    assert_eq!(
        availability
            .compute_best_available_level_over_rectangle(&Rectangle::from_degrees(1.0, 2.0, 3.0, 4.0)),
        0
    );
}

#[test]
fn reports_the_correct_level_when_entirely_inside_a_worldwide_rectangle_of_that_level() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(5, 0, 0, geo_x_at_level(5) - 1, geo_y_at_level(5) - 1);
    availability.add_available_tile_range(6, 7, 8, 9, 10);
    assert_eq!(
        availability
            .compute_best_available_level_over_rectangle(&Rectangle::from_degrees(1.0, 2.0, 3.0, 4.0)),
        5
    );
}

#[test]
fn reports_the_correct_level_when_entirely_inside_a_smaller_rectangle_of_that_level() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(5, 0, 0, geo_x_at_level(5) - 1, geo_y_at_level(5) - 1);
    availability.add_available_tile_range(6, 7, 8, 9, 10);
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    ts.tile_xy_to_rectangle(8, 9, 6, &mut rectangle);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&rectangle),
        6
    );
}

#[test]
fn reports_the_correct_level_when_partially_overlapping_a_smaller_rectangle() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(5, 0, 0, geo_x_at_level(5) - 1, geo_y_at_level(5) - 1);
    availability.add_available_tile_range(6, 7, 8, 7, 8);
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    ts.tile_xy_to_rectangle(7, 8, 6, &mut rectangle);
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
fn works_with_a_rectangle_crossing_180_degrees_longitude() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(5, 0, 0, geo_x_at_level(5) - 1, geo_y_at_level(5) - 1);
    availability.add_available_tile_range(6, 0, 0, 10, geo_y_at_level(6) - 1);
    availability.add_available_tile_range(
        6,
        geo_x_at_level(6) - 11,
        0,
        geo_x_at_level(6) - 1,
        geo_y_at_level(6) - 1,
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
fn works_when_four_rectangles_combine_to_cover_the_area() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(5, 0, 0, geo_x_at_level(5) - 1, geo_y_at_level(5) - 1);
    availability.add_available_tile_range(6, 0, 2, 1, 3);
    availability.add_available_tile_range(6, 2, 0, 3, 1);
    availability.add_available_tile_range(6, 0, 0, 1, 1);
    availability.add_available_tile_range(6, 2, 2, 3, 3);
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    ts.tile_xy_to_rectangle(0, 0, 4, &mut rectangle);
    assert_eq!(
        availability.compute_best_available_level_over_rectangle(&rectangle),
        6
    );
}

// =====================================================================
// addAvailableTileRange
// =====================================================================

#[test]
fn keeps_availability_ranges_sorted_by_rectangle() {
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    availability.add_available_tile_range(1, 0, 0, 3, 1);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::new(
            -std::f64::consts::FRAC_PI_2,
            0.0,
            0.0
        )),
        1
    );

    // We should get the same result adding them in the opposite order.
    let mut availability = create_availability_geographic();
    availability.add_available_tile_range(1, 0, 0, 3, 1);
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    assert_eq!(
        availability.compute_maximum_level_at_position(&Cartographic::new(
            -std::f64::consts::FRAC_PI_2,
            0.0,
            0.0
        )),
        1
    );
}

#[test]
fn ensure_the_boundary_rectangles_are_sorted_properly() {
    let ts = GeographicTilingScheme::new(None, None, None, None);
    let mut availability = TileAvailability::new(Box::new(ts), 6);
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    availability.add_available_tile_range(1, 0, 0, 2, 0);
    availability.add_available_tile_range(2, 0, 0, 4, 0);
    availability.add_available_tile_range(3, 0, 0, 8, 0);
    availability.add_available_tile_range(0, 0, 0, 1, 0);

    // Mirrors the JS loop over `availability._rootNodes` calling
    // `checkNodeRectanglesSorted(node)`.
    assert!(availability.debug_check_node_rectangles_sorted());
}
