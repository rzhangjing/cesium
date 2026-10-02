//! Core/GeographicTilingSchemeSpec → Rust 集成测试（对齐实现）。
//!
//! 对齐实现原始 CesiumJS
//! `Specs/Core/GeographicTilingSchemeSpec`（13 个 `it()` 用例）。
//! 参考值逐字使用，因此 Rust 实现针对与 CesiumJS 完全相同的
//! 基准真值进行验证。
//!
//! 平台适配（按验证计划记录在案）：
//! - CesiumJS 的 "conforms to TilingScheme interface" 使用动态的
//!   `toConformToInterface` 匹配器。Rust 的静态类型在编译期保证接口一致性；
//!   该用例被移植为一个冒烟测试，逐一检验瓦片方案接口的每个成员。
//! - 三个 "uses result parameter" 变体（tileXYToRectangle、
//!   rectangleToNativeRectangle、positionToTileXY）测试 JS 的内存复用 API
//!   契约（`result === returnValue`）。Rust 返回 owned 值且没有
//!   结果参数 API，因此这些变体被下面的 owned-返回测试所涵盖
//!   （计算值相同，单一代码路径）。
//! - CesiumJS 构造部分选项（`{numberOfLevelZeroTilesX: 1}` 或
//!   `{rectangle: ...}`）并依赖其余项的默认值。Rust 的
//!   `with_options` 显式接受全部四个选项，因此省略的选项被
//!   传入其 CesiumJS 默认值（`Ellipsoid.default` = WGS84、
//!   `Rectangle.MAX_VALUE`、2 个 x 瓦片、1 个 y 瓦片）。

use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::projection::{GeographicProjection, MapProjection};
use cesium_geospatial::rectangle::Rectangle;
use cesium_provider::tiling_scheme::GeographicTilingScheme;
use cesium_specs::{assert_approx, epsilon};
use std::f64::consts::PI;

// "conforms to TilingScheme interface."
#[test]
fn test_conforms_to_tiling_scheme_interface() {
    // Rust 的静态类型在编译期保证接口一致性；
    // 此冒烟测试逐一检验 TilingScheme 接口的每个成员。
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
