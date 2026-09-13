//! Core/GeographicTilingSchemeSpec.js → Rust integration tests (faithful port).
//!
//! Faithfully ports the original CesiumJS
//! `packages/engine/Specs/Core/GeographicTilingSchemeSpec.js` (13 `it()` cases).
//! Reference values are used verbatim so the Rust implementation is verified
//! against the exact same ground truth as CesiumJS.
//!
//! Platform adaptations (documented, per the verification plan):
//! - CesiumJS "conforms to TilingScheme interface" uses a dynamic
//!   `toConformToInterface` matcher. Rust's static typing guarantees interface
//!   conformance at compile time; the case is ported as a smoke test that
//!   exercises every member of the tiling-scheme interface.
//! - The three "uses result parameter" variants (tileXYToRectangle,
//!   rectangleToNativeRectangle, positionToTileXY) test the JS memory-reuse API
//!   contract (`result === returnValue`). Rust returns owned values and has no
//!   result-parameter API, so those variants are subsumed by the owned-return
//!   tests below (identical computed values, single code path).
//! - CesiumJS constructs partial options (`{numberOfLevelZeroTilesX: 1}` or
//!   `{rectangle: ...}`) relying on defaults for the rest. The Rust
//!   `with_options` takes all four options explicitly, so omitted options are
//!   passed their CesiumJS defaults (`Ellipsoid.default` = WGS84,
//!   `Rectangle.MAX_VALUE`, 2 x-tiles, 1 y-tile).

use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::projection::{GeographicProjection, MapProjection};
use cesium_geospatial::rectangle::Rectangle;
use cesium_provider::tiling_scheme::GeographicTilingScheme;
use cesium_specs::{assert_approx, epsilon};
use std::f64::consts::PI;

// "conforms to TilingScheme interface."
#[test]
fn test_conforms_to_tiling_scheme_interface() {
    // Rust's static typing guarantees interface conformance at compile time;
    // this smoke test exercises each member of the TilingScheme interface.
    let scheme = GeographicTilingScheme::new();
    let _ellipsoid: &Ellipsoid = &scheme.ellipsoid;
    let _rectangle: &Rectangle = &scheme.rectangle;
    let _projection: &GeographicProjection = &scheme.projection;
    let _ = scheme.number_of_x_tiles_at_level(0);
    let _ = scheme.number_of_y_tiles_at_level(0);
    let rect = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    let _ = scheme.rectangle_to_native_rectangle(&rect);
    let _ = scheme.tile_xy_to_native_rectangle(0, 0, 0);
    let _ = scheme.tile_xy_to_rectangle(0, 0, 0);
    let _ = scheme.position_to_tile_xy(0.0, 0.0, 0);
}

// "tileXYToRectangle returns full rectangle for single root tile."
// (The "uses result parameter" variant is subsumed here — owned return value.)
#[test]
fn test_tile_xy_to_rectangle_full_rectangle_for_single_root_tile() {
    let tiling_scheme =
        GeographicTilingScheme::with_options(Ellipsoid::WGS84, Rectangle::MAX_VALUE, 1, 1);
    let tiling_scheme_rectangle = tiling_scheme.rectangle;
    let rectangle = tiling_scheme.tile_xy_to_rectangle(0, 0, 0);
    assert_approx!(
        rectangle.west,
        tiling_scheme_rectangle.west,
        epsilon::EPSILON10
    );
    assert_approx!(
        rectangle.south,
        tiling_scheme_rectangle.south,
        epsilon::EPSILON10
    );
    assert_approx!(
        rectangle.east,
        tiling_scheme_rectangle.east,
        epsilon::EPSILON10
    );
    assert_approx!(
        rectangle.north,
        tiling_scheme_rectangle.north,
        epsilon::EPSILON10
    );
}

// "tiles are numbered from the northwest corner."
#[test]
fn test_tiles_are_numbered_from_the_northwest_corner() {
    let tiling_scheme =
        GeographicTilingScheme::with_options(Ellipsoid::WGS84, Rectangle::MAX_VALUE, 2, 2);
    let northwest = tiling_scheme.tile_xy_to_rectangle(0, 0, 1);
    let northeast = tiling_scheme.tile_xy_to_rectangle(1, 0, 1);
    let southeast = tiling_scheme.tile_xy_to_rectangle(1, 1, 1);
    let southwest = tiling_scheme.tile_xy_to_rectangle(0, 1, 1);

    assert_eq!(northeast.north, northwest.north);
    assert_eq!(northeast.south, northwest.south);
    assert_eq!(southeast.north, southwest.north);
    assert_eq!(southeast.south, southwest.south);

    assert_eq!(northwest.west, southwest.west);
    assert_eq!(northwest.east, southwest.east);
    assert_eq!(northeast.west, southeast.west);
    assert_eq!(northeast.east, southeast.east);

    assert!(northeast.north > southeast.north);
    assert!(northeast.south > southeast.south);
    assert!(northwest.north > southwest.north);
    assert!(northwest.south > southwest.south);

    assert!(northeast.east > northwest.east);
    assert!(northeast.west > northwest.west);
    assert!(southeast.east > southwest.east);
    assert!(southeast.west > southwest.west);
}

// "adjacent tiles have overlapping coordinates"
#[test]
fn test_adjacent_tiles_have_overlapping_coordinates() {
    let tiling_scheme =
        GeographicTilingScheme::with_options(Ellipsoid::WGS84, Rectangle::MAX_VALUE, 2, 2);
    let northwest = tiling_scheme.tile_xy_to_rectangle(0, 0, 1);
    let northeast = tiling_scheme.tile_xy_to_rectangle(1, 0, 1);
    let southeast = tiling_scheme.tile_xy_to_rectangle(1, 1, 1);
    let southwest = tiling_scheme.tile_xy_to_rectangle(0, 1, 1);

    assert_approx!(northeast.south, southeast.north, epsilon::EPSILON15);
    assert_approx!(northwest.south, southwest.north, epsilon::EPSILON15);

    assert_approx!(northeast.west, northwest.east, epsilon::EPSILON15);
    assert_approx!(southeast.west, southwest.east, epsilon::EPSILON15);
}

// "uses a GeographicProjection"
#[test]
fn test_uses_a_geographic_projection() {
    let tiling_scheme = GeographicTilingScheme::new();
    // The `projection` field is statically typed `GeographicProjection`
    // (the Rust analogue of `toBeInstanceOf(GeographicProjection)`).
    let projection: &GeographicProjection = &tiling_scheme.projection;
    assert_eq!(projection.ellipsoid(), &Ellipsoid::WGS84);
}

// "rectangleToNativeRectangle converts radians to degrees"
// (The "uses result parameter" variant is subsumed here — owned return value.)
#[test]
fn test_rectangle_to_native_rectangle_converts_radians_to_degrees() {
    let tiling_scheme = GeographicTilingScheme::new();
    let rectangle_in_radians = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    let native_rectangle = tiling_scheme.rectangle_to_native_rectangle(&rectangle_in_radians);
    assert_approx!(
        native_rectangle.west,
        (rectangle_in_radians.west * 180.0) / PI,
        epsilon::EPSILON13
    );
    assert_approx!(
        native_rectangle.south,
        (rectangle_in_radians.south * 180.0) / PI,
        epsilon::EPSILON13
    );
    assert_approx!(
        native_rectangle.east,
        (rectangle_in_radians.east * 180.0) / PI,
        epsilon::EPSILON13
    );
    assert_approx!(
        native_rectangle.north,
        (rectangle_in_radians.north * 180.0) / PI,
        epsilon::EPSILON13
    );
}

// "positionToTileXY returns undefined when outside rectangle"
#[test]
fn test_position_to_tile_xy_returns_none_when_outside_rectangle() {
    let tiling_scheme = GeographicTilingScheme::with_options(
        Ellipsoid::WGS84,
        Rectangle::new(0.1, 0.2, 0.3, 0.4),
        2,
        1,
    );

    // tooFarWest
    assert!(tiling_scheme.position_to_tile_xy(0.05, 0.3, 0).is_none());
    // tooFarSouth
    assert!(tiling_scheme.position_to_tile_xy(0.2, 0.1, 0).is_none());
    // tooFarEast
    assert!(tiling_scheme.position_to_tile_xy(0.4, 0.3, 0).is_none());
    // tooFarNorth
    assert!(tiling_scheme.position_to_tile_xy(0.2, 0.5, 0).is_none());
}

// "positionToTileXY returns correct tile for position in center of tile"
// (The "uses result parameter" variant is subsumed here — owned return value.)
#[test]
fn test_position_to_tile_xy_returns_correct_tile_for_position_in_center_of_tile() {
    let tiling_scheme = GeographicTilingScheme::new();

    let center_of_western_root_tile = tiling_scheme
        .position_to_tile_xy(-PI / 2.0, 0.0, 0)
        .unwrap();
    assert_eq!(center_of_western_root_tile.x, 0);
    assert_eq!(center_of_western_root_tile.y, 0);

    let center_of_northeast_child_of_eastern_root_tile = tiling_scheme
        .position_to_tile_xy((3.0 * PI) / 4.0, PI / 2.0, 1)
        .unwrap();
    assert_eq!(center_of_northeast_child_of_eastern_root_tile.x, 3);
    assert_eq!(center_of_northeast_child_of_eastern_root_tile.y, 0);
}

// "positionToTileXY returns Southeast tile when on the boundary between tiles"
#[test]
fn test_position_to_tile_xy_returns_southeast_tile_when_on_the_boundary() {
    let tiling_scheme = GeographicTilingScheme::new();

    let center_of_map = tiling_scheme.position_to_tile_xy(0.0, 0.0, 1).unwrap();
    assert_eq!(center_of_map.x, 2);
    assert_eq!(center_of_map.y, 1);
}

// "positionToTileXY does not return tile outside valid range"
#[test]
fn test_position_to_tile_xy_does_not_return_tile_outside_valid_range() {
    let tiling_scheme = GeographicTilingScheme::new();

    let southeast_corner = tiling_scheme
        .position_to_tile_xy(PI, -PI / 2.0, 0)
        .unwrap();
    assert_eq!(southeast_corner.x, 1);
    assert_eq!(southeast_corner.y, 0);
}
