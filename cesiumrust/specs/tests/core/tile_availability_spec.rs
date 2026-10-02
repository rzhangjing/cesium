//! TileAvailability 规格测试 - 参考自 Core/TileAvailabilitySpec
//!
//! 原始：12 个 it() 测试。移植：11 个 A 类。
//! 省略：1 个（内部 _rootNodes 结构检查 → 改为功能性测试）。
//!
//! 说明：
//! - `checkNodeRectanglesSorted`（内部四叉树结构校验）属于
//!   C 类（访问私有的 _rootNodes/_ne/_se/_nw/_sw）；改为一个
//!   等价的功能性测试，用于验证 addAvailableTileRange 与添加顺序无关。

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

    // 确保它不依赖于我们添加矩形的顺序。
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

    // 以相反顺序添加它们应得到相同结果。
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
    // 改编自原始测试：不检查内部节点结构，
    // 而是验证按边界排序矩形的功能正确性。
    let mut availability = TileAvailability::new(geographic(), 6);
    availability.add_available_tile_range(0, 0, 0, 1, 0);
    availability.add_available_tile_range(1, 0, 0, 2, 0);
    availability.add_available_tile_range(2, 0, 0, 4, 0);
    availability.add_available_tile_range(3, 0, 0, 8, 0);
    availability.add_available_tile_range(0, 0, 0, 1, 0);

    // 在所有范围都覆盖的位置，所有层级都应可用。
    // Level 3 范围覆盖 x=0..8、y=0 → 经度 -180..22.5E、纬度 67.5..90N
    let pos = Cartographic::from_degrees(-90.0, 78.75, 0.0);
    assert_eq!(availability.compute_maximum_level_at_position(&pos), 3);

    // 该位置仅覆盖到 level 2（45E,45N 位于 level-2 范围边缘）
    let pos2 = Cartographic::from_degrees(22.5, 56.25, 0.0);
    assert_eq!(availability.compute_maximum_level_at_position(&pos2), 2);

    // isTileAvailable 应对所有已添加的层级生效
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

    // Level 0 瓦片 (0,0)：level 1 的四个子瓦片都应可用
    // NW=(0,0), NE=(1,0), SW=(0,1), SE=(1,1)
    let mask = availability.compute_child_mask_for_tile(0, 0, 0);
    assert_eq!(mask, 0b1111);

    // Level 0 瓦片 (1,0)：level 1 的子瓦片为 (2,0),(3,0),(2,1),(3,1)
    let mask = availability.compute_child_mask_for_tile(0, 1, 0);
    assert_eq!(mask, 0b1111);

    // 在最大层级处，掩码应为 0
    let mask = availability.compute_child_mask_for_tile(15, 0, 0);
    assert_eq!(mask, 0);
}
