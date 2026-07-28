//! Core/WebMercatorTilingSchemeSpec.js → Rust integration tests (faithful port).
//!
//! Faithfully ports the original CesiumJS
//! `packages/engine/Specs/Core/WebMercatorTilingSchemeSpec.js` (20 `it()` cases).
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
//! - CesiumJS `Rectangle.southwest/northwest/northeast/southeast(rect)` return a
//!   `Cartographic` which is then passed to `positionToTileXY`. The Rust
//!   `position_to_tile_xy` takes `(longitude, latitude)` directly, so the corner
//!   accessor's fields are forwarded (same values, no semantic change).
//! - The "uses specified ellipsoid" / "outside rectangle" cases construct the
//!   scheme with partial options in CesiumJS; the Rust `with_ellipsoid` /
//!   `with_meter_corners` constructors express the same intent.

use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::projection::{MapProjection, WebMercatorProjection};
use cesium_geospatial::rectangle::Rectangle;
use cesium_provider::tiling_scheme::WebMercatorTilingScheme;
use cesium_specs::{assert_approx, epsilon};
use std::f64::consts::PI;

// "conforms to TilingScheme interface."
#[test]
fn test_conforms_to_tiling_scheme_interface() {
    // Rust's static typing guarantees interface conformance at compile time;
    // this smoke test exercises each member of the TilingScheme interface.
    let scheme = WebMercatorTilingScheme::new();
    let _ellipsoid: &Ellipsoid = &scheme.ellipsoid;
    let _rectangle: &Rectangle = &scheme.rectangle;
    let _projection: &WebMercatorProjection = &scheme.projection;
    let _ = scheme.number_of_x_tiles_at_level(0);
    let _ = scheme.number_of_y_tiles_at_level(0);
    let rect = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    let _ = scheme.rectangle_to_native_rectangle(&rect);
    let _ = scheme.tile_xy_to_native_rectangle(0, 0, 0);
    let _ = scheme.tile_xy_to_rectangle(0, 0, 0);
    let _ = scheme.position_to_tile_xy(0.0, 0.0, 0);
}

// "default constructing uses WGS84 ellipsoid"
#[test]
fn test_default_constructing_uses_wgs84_ellipsoid() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    assert_eq!(tiling_scheme.ellipsoid, Ellipsoid::WGS84);
}

// "uses specified ellipsoid"
#[test]
fn test_uses_specified_ellipsoid() {
    let tiling_scheme = WebMercatorTilingScheme::with_ellipsoid(Ellipsoid::UNIT_SPHERE);
    assert_eq!(tiling_scheme.ellipsoid, Ellipsoid::UNIT_SPHERE);
}

// "tileXYToRectangle returns full rectangle for single root tile."
// (The "uses result parameter" variant is subsumed here — owned return value.)
#[test]
fn test_tile_xy_to_rectangle_full_rectangle_for_single_root_tile() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    let rectangle = tiling_scheme.tile_xy_to_rectangle(0, 0, 0);
    let tiling_scheme_rectangle = tiling_scheme.rectangle;
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
    let tiling_scheme = WebMercatorTilingScheme::new();
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
    let tiling_scheme = WebMercatorTilingScheme::new();
    let northwest = tiling_scheme.tile_xy_to_rectangle(0, 0, 1);
    let northeast = tiling_scheme.tile_xy_to_rectangle(1, 0, 1);
    let southeast = tiling_scheme.tile_xy_to_rectangle(1, 1, 1);
    let southwest = tiling_scheme.tile_xy_to_rectangle(0, 1, 1);

    assert_approx!(northeast.south, southeast.north, epsilon::EPSILON15);
    assert_approx!(northwest.south, southwest.north, epsilon::EPSILON15);

    assert_approx!(northeast.west, northwest.east, epsilon::EPSILON15);
    assert_approx!(southeast.west, southwest.east, epsilon::EPSILON15);
}

// "calculates correct tile indices for 4 corners at level 0"
#[test]
fn test_calculates_correct_tile_indices_for_4_corners_at_level_0() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    let rect = tiling_scheme.rectangle;

    // southwest corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.west, rect.south, 0)
        .unwrap();
    assert_eq!(coordinates.x, 0);
    assert_eq!(coordinates.y, 0);

    // northwest corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.west, rect.north, 0)
        .unwrap();
    assert_eq!(coordinates.x, 0);
    assert_eq!(coordinates.y, 0);

    // northeast corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.east, rect.north, 0)
        .unwrap();
    assert_eq!(coordinates.x, 0);
    assert_eq!(coordinates.y, 0);

    // southeast corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.east, rect.south, 0)
        .unwrap();
    assert_eq!(coordinates.x, 0);
    assert_eq!(coordinates.y, 0);
}

// "calculates correct tile indices for 4 corners at level 1"
#[test]
fn test_calculates_correct_tile_indices_for_4_corners_at_level_1() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    let rect = tiling_scheme.rectangle;

    // southwest corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.west, rect.south, 1)
        .unwrap();
    assert_eq!(coordinates.x, 0);
    assert_eq!(coordinates.y, 1);

    // northwest corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.west, rect.north, 1)
        .unwrap();
    assert_eq!(coordinates.x, 0);
    assert_eq!(coordinates.y, 0);

    // northeast corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.east, rect.north, 1)
        .unwrap();
    assert_eq!(coordinates.x, 1);
    assert_eq!(coordinates.y, 0);

    // southeast corner
    let coordinates = tiling_scheme
        .position_to_tile_xy(rect.east, rect.south, 1)
        .unwrap();
    assert_eq!(coordinates.x, 1);
    assert_eq!(coordinates.y, 1);
}

// "calculates correct tile indices for the center at level 1"
#[test]
fn test_calculates_correct_tile_indices_for_the_center_at_level_1() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    let coordinates = tiling_scheme.position_to_tile_xy(0.0, 0.0, 1).unwrap();
    assert_eq!(coordinates.x, 1);
    assert_eq!(coordinates.y, 1);
}

// "calculates correct tile indices for the center at level 2"
#[test]
fn test_calculates_correct_tile_indices_for_the_center_at_level_2() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    let coordinates = tiling_scheme.position_to_tile_xy(0.0, 0.0, 2).unwrap();
    assert_eq!(coordinates.x, 2);
    assert_eq!(coordinates.y, 2);
}

// "calculates correct tile indices around the center at level 2"
#[test]
fn test_calculates_correct_tile_indices_around_the_center_at_level_2() {
    let tiling_scheme = WebMercatorTilingScheme::new();

    let coordinates = tiling_scheme.position_to_tile_xy(-0.05, -0.05, 2).unwrap();
    assert_eq!(coordinates.x, 1);
    assert_eq!(coordinates.y, 2);

    let coordinates = tiling_scheme.position_to_tile_xy(-0.05, 0.05, 2).unwrap();
    assert_eq!(coordinates.x, 1);
    assert_eq!(coordinates.y, 1);

    let coordinates = tiling_scheme.position_to_tile_xy(0.05, 0.05, 2).unwrap();
    assert_eq!(coordinates.x, 2);
    assert_eq!(coordinates.y, 1);

    let coordinates = tiling_scheme.position_to_tile_xy(0.05, -0.05, 2).unwrap();
    assert_eq!(coordinates.x, 2);
    assert_eq!(coordinates.y, 2);
}

// "uses a WebMercatorProjection"
#[test]
fn test_uses_a_web_mercator_projection() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    // The `projection` field is statically typed `WebMercatorProjection`
    // (the Rust analogue of `toBeInstanceOf(WebMercatorProjection)`).
    let projection: &WebMercatorProjection = &tiling_scheme.projection;
    assert_eq!(projection.ellipsoid(), &Ellipsoid::WGS84);
}

// "rectangleToNativeRectangle converts radians to web mercator meters"
// (The "uses result parameter" variant is subsumed here — owned return value.)
#[test]
fn test_rectangle_to_native_rectangle_converts_radians_to_web_mercator_meters() {
    let tiling_scheme = WebMercatorTilingScheme::new();
    let rectangle_in_radians = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    let native_rectangle = tiling_scheme.rectangle_to_native_rectangle(&rectangle_in_radians);

    let projection = WebMercatorProjection::wgs84();
    let expected_southwest = projection.project(&rectangle_in_radians.southwest());
    let expected_northeast = projection.project(&rectangle_in_radians.northeast());

    assert_approx!(native_rectangle.west, expected_southwest.x, epsilon::EPSILON13);
    assert_approx!(native_rectangle.south, expected_southwest.y, epsilon::EPSILON13);
    assert_approx!(native_rectangle.east, expected_northeast.x, epsilon::EPSILON13);
    assert_approx!(native_rectangle.north, expected_northeast.y, epsilon::EPSILON13);
}

// "positionToTileXY returns undefined when outside rectangle"
#[test]
fn test_position_to_tile_xy_returns_none_when_outside_rectangle() {
    let projection = WebMercatorProjection::wgs84();
    let rectangle_in_radians = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    let sw = projection.project(&rectangle_in_radians.southwest());
    let ne = projection.project(&rectangle_in_radians.northeast());
    let tiling_scheme =
        WebMercatorTilingScheme::with_meter_corners(Ellipsoid::WGS84, (sw.x, sw.y), (ne.x, ne.y));

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
    let tiling_scheme = WebMercatorTilingScheme::new();

    let center_of_southwestern_child = tiling_scheme
        .position_to_tile_xy(-PI / 2.0, -PI / 4.0, 1)
        .unwrap();
    assert_eq!(center_of_southwestern_child.x, 0);
    assert_eq!(center_of_southwestern_child.y, 1);

    let center_of_northeastern_child = tiling_scheme
        .position_to_tile_xy(PI / 2.0, PI / 4.0, 1)
        .unwrap();
    assert_eq!(center_of_northeastern_child.x, 1);
    assert_eq!(center_of_northeastern_child.y, 0);
}

// "positionToTileXY returns Southeast tile when on the boundary between tiles"
#[test]
fn test_position_to_tile_xy_returns_southeast_tile_when_on_the_boundary() {
    let tiling_scheme = WebMercatorTilingScheme::new();

    let center_of_map = tiling_scheme.position_to_tile_xy(0.0, 0.0, 1).unwrap();
    assert_eq!(center_of_map.x, 1);
    assert_eq!(center_of_map.y, 1);
}

// "positionToTileXY does not return tile outside valid range"
#[test]
fn test_position_to_tile_xy_does_not_return_tile_outside_valid_range() {
    let tiling_scheme = WebMercatorTilingScheme::new();

    let southeast_corner = tiling_scheme.rectangle.southeast();
    let coordinates = tiling_scheme
        .position_to_tile_xy(southeast_corner.longitude, southeast_corner.latitude, 1)
        .unwrap();
    assert_eq!(coordinates.x, 1);
    assert_eq!(coordinates.y, 1);
}
