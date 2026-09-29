//! 用于地图瓦片组织的裁剪方案。
//!
//! 映射到 CesiumJS：
//! - `Core/GeographicTilingScheme.js`
//! - `Core/WebMercatorTilingScheme.js`

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils::{to_degrees, TWO_PI};
use cesium_geospatial::projection::{
    GeographicProjection, MapProjection, WebMercatorProjection,
};
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use std::f64::consts::PI;

use crate::imagery_provider::TileCoord;

/// 用于将 globe 划分为瓦片的裁剪方案。
///
/// 映射到 CesiumJS `GeographicTilingScheme` 与 `WebMercatorTilingScheme`
#[derive(Debug, Clone)]
pub enum TilingScheme {
    /// 地理（EPSG:4326）裁剪方案。
    /// 默认：在层级 0 为 2 瓦宽、1 瓦高。
    Geographic(GeographicTilingScheme),
    /// Web 墨卡托（EPSG:3857）裁剪方案。
    /// 默认：在层级 0 为 1 瓦宽、1 瓦高。
    WebMercator(WebMercatorTilingScheme),
}

/// 地理（EPSG:4326）裁剪方案。
///
/// 映射到 CesiumJS `Core/GeographicTilingScheme.js`
#[derive(Debug, Clone)]
pub struct GeographicTilingScheme {
    /// 由该裁剪方案划分的椭球体。
    pub ellipsoid: Ellipsoid,
    /// 该裁剪方案使用的地图投影。
    pub projection: GeographicProjection,
    /// 裁剪方案覆盖的矩形（弧度）。
    pub rectangle: Rectangle,
    /// 层级 0 时 X 方向的瓦片数。
    pub number_of_level_zero_tiles_x: u32,
    /// 层级 0 时 Y 方向的瓦片数。
    pub number_of_level_zero_tiles_y: u32,
}

impl Default for GeographicTilingScheme {
    fn default() -> Self {
        Self::with_options(Ellipsoid::WGS84, Rectangle::MAX_VALUE, 2, 1)
    }
}

impl GeographicTilingScheme {
    /// 使用默认设置创建一个新的地理裁剪方案。
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用自定义参数创建一个地理裁剪方案。
    ///
    /// 映射到 CesiumJS 构造函数选项（`ellipsoid`、`rectangle`、
    /// `numberOfLevelZeroTilesX`、`numberOfLevelZeroTilesY`）。
    pub fn with_options(
        ellipsoid: Ellipsoid,
        rectangle: Rectangle,
        tiles_x: u32,
        tiles_y: u32,
    ) -> Self {
        Self {
            projection: GeographicProjection::new(ellipsoid),
            ellipsoid,
            rectangle,
            number_of_level_zero_tiles_x: tiles_x,
            number_of_level_zero_tiles_y: tiles_y,
        }
    }

    /// 获取给定层级下 X 方向的瓦片数。
    pub fn number_of_x_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_x << level
    }

    /// 获取给定层级下 Y 方向的瓦片数。
    pub fn number_of_y_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_y << level
    }

    /// 将瓦片 x、y、层级转换为以弧度表示的矩形。
    ///
    /// 映射到 `GeographicTilingScheme.tileXYToRectangle`
    pub fn tile_xy_to_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let rectangle = &self.rectangle;
        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let x_tile_width = rectangle.width() / x_tiles as f64;
        let west = x as f64 * x_tile_width + rectangle.west;
        let east = (x as f64 + 1.0) * x_tile_width + rectangle.west;

        let y_tile_height = rectangle.height() / y_tiles as f64;
        let north = rectangle.north - y as f64 * y_tile_height;
        let south = rectangle.north - (y as f64 + 1.0) * y_tile_height;

        Rectangle::new(west, south, east, north)
    }

    /// 将一个位置（弧度）转换为给定层级下的瓦片坐标。
    ///
    /// 映射到 `GeographicTilingScheme.positionToTileXY`
    pub fn position_to_tile_xy(
        &self,
        longitude: f64,
        latitude: f64,
        level: u32,
    ) -> Option<TileCoord> {
        let rectangle = &self.rectangle;
        if !rectangle.contains(longitude, latitude) {
            // 超出裁剪方案的边界
            return None;
        }

        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let x_tile_width = rectangle.width() / x_tiles as f64;
        let y_tile_height = rectangle.height() / y_tiles as f64;

        let mut longitude = longitude;
        if rectangle.east < rectangle.west {
            longitude += TWO_PI;
        }

        // JS 的 `| 0` 向零截断；Rust 的 `as i64` 行为相同。
        let mut x_tile_coordinate = ((longitude - rectangle.west) / x_tile_width) as i64;
        if x_tile_coordinate >= x_tiles as i64 {
            x_tile_coordinate = x_tiles as i64 - 1;
        }

        let mut y_tile_coordinate = ((rectangle.north - latitude) / y_tile_height) as i64;
        if y_tile_coordinate >= y_tiles as i64 {
            y_tile_coordinate = y_tiles as i64 - 1;
        }

        Some(TileCoord::new(
            x_tile_coordinate as u32,
            y_tile_coordinate as u32,
            level,
        ))
    }

    /// 将一个测地位置转换为瓦片坐标。
    pub fn cartographic_to_tile_xy(
        &self,
        cartographic: &Cartographic,
        level: u32,
    ) -> Option<TileCoord> {
        self.position_to_tile_xy(cartographic.longitude, cartographic.latitude, level)
    }

    /// 将一个矩形变换到原生坐标（地理方案为度）。
    ///
    /// 映射到 `GeographicTilingScheme.rectangleToNativeRectangle`
    pub fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle) -> Rectangle {
        Rectangle::new(
            to_degrees(rectangle.west),
            to_degrees(rectangle.south),
            to_degrees(rectangle.east),
            to_degrees(rectangle.north),
        )
    }

    /// 将瓦片 x、y、层级转换为原生矩形（度）。
    ///
    /// 映射到 `GeographicTilingScheme.tileXYToNativeRectangle`
    pub fn tile_xy_to_native_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let rect = self.tile_xy_to_rectangle(x, y, level);
        self.rectangle_to_native_rectangle(&rect)
    }
}

/// Web 墨卡托（EPSG:3857）裁剪方案。
///
/// 映射到 CesiumJS `Core/WebMercatorTilingScheme.js`
#[derive(Debug, Clone)]
pub struct WebMercatorTilingScheme {
    /// 由该裁剪方案划分的椭球体。
    pub ellipsoid: Ellipsoid,
    /// 该裁剪方案使用的地图投影。
    pub projection: WebMercatorProjection,
    /// 覆盖的矩形（弧度，钳制到墨卡托边界）。
    pub rectangle: Rectangle,
    /// 层级 0 时 X 方向的瓦片数。
    pub number_of_level_zero_tiles_x: u32,
    /// 层级 0 时 Y 方向的瓦片数。
    pub number_of_level_zero_tiles_y: u32,
    /// 投影米制下的西南角。
    pub rectangle_southwest_in_meters: (f64, f64),
    /// 投影米制下的东北角。
    pub rectangle_northeast_in_meters: (f64, f64),
}

impl Default for WebMercatorTilingScheme {
    fn default() -> Self {
        Self::with_options(Ellipsoid::WGS84, 1, 1, None, None)
    }
}

impl WebMercatorTilingScheme {
    /// 使用默认设置创建一个新的 Web 墨卡托裁剪方案。
    pub fn new() -> Self {
        Self::default()
    }

    /// 为自定义椭球体创建一个 Web 墨卡托裁剪方案。
    pub fn with_ellipsoid(ellipsoid: Ellipsoid) -> Self {
        Self::with_options(ellipsoid, 1, 1, None, None)
    }

    /// 创建一个覆盖自定义矩形的 Web 墨卡托裁剪方案，该矩形
    /// 由其在投影米制下的西南/东北角给定。
    pub fn with_meter_corners(
        ellipsoid: Ellipsoid,
        southwest_in_meters: (f64, f64),
        northeast_in_meters: (f64, f64),
    ) -> Self {
        Self::with_options(
            ellipsoid,
            1,
            1,
            Some(southwest_in_meters),
            Some(northeast_in_meters),
        )
    }

    /// 使用完全自定义选项创建一个 Web 墨卡托裁剪方案。
    ///
    /// 映射到 CesiumJS 构造函数选项（`ellipsoid`、
    /// `numberOfLevelZeroTilesX/Y`、`rectangleSouthwestInMeters`、
    /// `rectangleNortheastInMeters`）。
    pub fn with_options(
        ellipsoid: Ellipsoid,
        tiles_x: u32,
        tiles_y: u32,
        southwest_in_meters: Option<(f64, f64)>,
        northeast_in_meters: Option<(f64, f64)>,
    ) -> Self {
        let projection = WebMercatorProjection::new(ellipsoid);

        let (sw, ne) = match (southwest_in_meters, northeast_in_meters) {
            (Some(sw), Some(ne)) => (sw, ne),
            _ => {
                let semimajor_axis_times_pi = ellipsoid.maximum_radius() * PI;
                (
                    (-semimajor_axis_times_pi, -semimajor_axis_times_pi),
                    (semimajor_axis_times_pi, semimajor_axis_times_pi),
                )
            }
        };

        let southwest = projection.unproject(DVec3::new(sw.0, sw.1, 0.0));
        let northeast = projection.unproject(DVec3::new(ne.0, ne.1, 0.0));

        let rectangle = Rectangle::new(
            southwest.longitude,
            southwest.latitude,
            northeast.longitude,
            northeast.latitude,
        );

        Self {
            ellipsoid,
            projection,
            rectangle,
            number_of_level_zero_tiles_x: tiles_x,
            number_of_level_zero_tiles_y: tiles_y,
            rectangle_southwest_in_meters: sw,
            rectangle_northeast_in_meters: ne,
        }
    }

    /// 获取给定层级下 X 方向的瓦片数。
    pub fn number_of_x_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_x << level
    }

    /// 获取给定层级下 Y 方向的瓦片数。
    pub fn number_of_y_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_y << level
    }

    /// 将一个矩形变换到原生坐标（Web 墨卡托米制）。
    ///
    /// 映射到 `WebMercatorTilingScheme.rectangleToNativeRectangle`
    pub fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle) -> Rectangle {
        let southwest = self.projection.project(&rectangle.southwest());
        let northeast = self.projection.project(&rectangle.northeast());
        Rectangle::new(southwest.x, southwest.y, northeast.x, northeast.y)
    }

    /// 将瓦片 x、y、层级转换为原生矩形（米）。
    ///
    /// 映射到 `WebMercatorTilingScheme.tileXYToNativeRectangle`
    pub fn tile_xy_to_native_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let (sw_x, sw_y) = self.rectangle_southwest_in_meters;
        let (ne_x, ne_y) = self.rectangle_northeast_in_meters;

        let x_tile_width = (ne_x - sw_x) / x_tiles as f64;
        let west = sw_x + x as f64 * x_tile_width;
        let east = sw_x + (x as f64 + 1.0) * x_tile_width;

        let y_tile_height = (ne_y - sw_y) / y_tiles as f64;
        let north = ne_y - y as f64 * y_tile_height;
        let south = ne_y - (y as f64 + 1.0) * y_tile_height;

        Rectangle::new(west, south, east, north)
    }

    /// 将瓦片 x、y、层级转换为以弧度表示的矩形。
    ///
    /// 映射到 `WebMercatorTilingScheme.tileXYToRectangle`
    pub fn tile_xy_to_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let native = self.tile_xy_to_native_rectangle(x, y, level);
        let southwest = self
            .projection
            .unproject(DVec3::new(native.west, native.south, 0.0));
        let northeast = self
            .projection
            .unproject(DVec3::new(native.east, native.north, 0.0));
        Rectangle::new(
            southwest.longitude,
            southwest.latitude,
            northeast.longitude,
            northeast.latitude,
        )
    }

    /// 将一个位置（弧度）转换为给定层级下的瓦片坐标。
    ///
    /// 映射到 `WebMercatorTilingScheme.positionToTileXY`
    pub fn position_to_tile_xy(
        &self,
        longitude: f64,
        latitude: f64,
        level: u32,
    ) -> Option<TileCoord> {
        let rectangle = &self.rectangle;
        if !rectangle.contains(longitude, latitude) {
            // 超出裁剪方案的边界
            return None;
        }

        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let (sw_x, sw_y) = self.rectangle_southwest_in_meters;
        let (ne_x, ne_y) = self.rectangle_northeast_in_meters;

        let overall_width = ne_x - sw_x;
        let x_tile_width = overall_width / x_tiles as f64;
        let overall_height = ne_y - sw_y;
        let y_tile_height = overall_height / y_tiles as f64;

        let position = Cartographic::from_radians(longitude, latitude, 0.0);
        let web_mercator_position = self.projection.project(&position);
        let distance_from_west = web_mercator_position.x - sw_x;
        let distance_from_north = ne_y - web_mercator_position.y;

        // JS 的 `| 0` 向零截断；Rust 的 `as i64` 行为相同。
        let mut x_tile_coordinate = (distance_from_west / x_tile_width) as i64;
        if x_tile_coordinate >= x_tiles as i64 {
            x_tile_coordinate = x_tiles as i64 - 1;
        }
        let mut y_tile_coordinate = (distance_from_north / y_tile_height) as i64;
        if y_tile_coordinate >= y_tiles as i64 {
            y_tile_coordinate = y_tiles as i64 - 1;
        }

        Some(TileCoord::new(
            x_tile_coordinate as u32,
            y_tile_coordinate as u32,
            level,
        ))
    }

    /// 将一个测地位置转换为瓦片坐标。
    pub fn cartographic_to_tile_xy(
        &self,
        cartographic: &Cartographic,
        level: u32,
    ) -> Option<TileCoord> {
        self.position_to_tile_xy(cartographic.longitude, cartographic.latitude, level)
    }
}

impl TilingScheme {
    /// 创建一个默认的地理裁剪方案。
    pub fn geographic() -> Self {
        Self::Geographic(GeographicTilingScheme::default())
    }

    /// 创建一个默认的 Web 墨卡托裁剪方案。
    pub fn web_mercator() -> Self {
        Self::WebMercator(WebMercatorTilingScheme::default())
    }

    /// 获取给定层级下 X 方向的瓦片数。
    pub fn number_of_x_tiles_at_level(&self, level: u32) -> u32 {
        match self {
            Self::Geographic(g) => g.number_of_x_tiles_at_level(level),
            Self::WebMercator(w) => w.number_of_x_tiles_at_level(level),
        }
    }

    /// 获取给定层级下 Y 方向的瓦片数。
    pub fn number_of_y_tiles_at_level(&self, level: u32) -> u32 {
        match self {
            Self::Geographic(g) => g.number_of_y_tiles_at_level(level),
            Self::WebMercator(w) => w.number_of_y_tiles_at_level(level),
        }
    }

    /// 将瓦片坐标转换为以弧度表示的矩形。
    pub fn tile_xy_to_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        match self {
            Self::Geographic(g) => g.tile_xy_to_rectangle(x, y, level),
            Self::WebMercator(w) => w.tile_xy_to_rectangle(x, y, level),
        }
    }

    /// 将一个位置转换为瓦片坐标。
    pub fn position_to_tile_xy(
        &self,
        longitude: f64,
        latitude: f64,
        level: u32,
    ) -> Option<TileCoord> {
        match self {
            Self::Geographic(g) => g.position_to_tile_xy(longitude, latitude, level),
            Self::WebMercator(w) => w.position_to_tile_xy(longitude, latitude, level),
        }
    }

    /// 获取该裁剪方案覆盖的矩形。
    pub fn rectangle(&self) -> &Rectangle {
        match self {
            Self::Geographic(g) => &g.rectangle,
            Self::WebMercator(w) => &w.rectangle,
        }
    }
}

// ─── TileAvailability（对 Core/TileAvailability.js 的忠实移植）────────────

/// 标记了可用性层级的矩形。
#[derive(Debug, Clone, Copy)]
struct RectangleWithLevel {
    level: u32,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
}

/// 内部四叉树节点（slab 分配）。
#[derive(Debug, Clone)]
struct AvailabilityNode {
    level: u32,
    x: u32,
    y: u32,
    extent: Rectangle,
    rectangles: Vec<RectangleWithLevel>,
    parent: Option<usize>,
    /// 子节点：[nw, ne, sw, se]，惰性创建。
    children: [Option<usize>; 4],
}

/// 报告裁剪方案中瓦片的可用性。
///
/// 映射到 CesiumJS `Core/TileAvailability.js`
#[derive(Debug, Clone)]
pub struct TileAvailability {
    tiling_scheme: TilingScheme,
    maximum_level: u32,
    root_nodes: Vec<usize>,
    nodes: Vec<AvailabilityNode>,
}

fn rectangles_overlap(r1_west: f64, r1_south: f64, r1_east: f64, r1_north: f64, r2: &RectangleWithLevel) -> bool {
    let west = r1_west.max(r2.west);
    let south = r1_south.max(r2.south);
    let east = r1_east.min(r2.east);
    let north = r1_north.min(r2.north);
    south < north && west < east
}

fn rectangle_fully_contains(container: &Rectangle, r: &RectangleWithLevel) -> bool {
    r.west >= container.west && r.east <= container.east
        && r.south >= container.south && r.north <= container.north
}

fn rectangle_contains_position(r_west: f64, r_south: f64, r_east: f64, r_north: f64, lon: f64, lat: f64) -> bool {
    lon >= r_west && lon <= r_east && lat >= r_south && lat <= r_north
}

/// 用于覆盖相减的简单矩形。
#[derive(Debug, Clone, Copy)]
struct CoverageRect {
    west: f64,
    south: f64,
    east: f64,
    north: f64,
}

fn coverage_rects_overlap(a: &CoverageRect, b: &CoverageRect) -> bool {
    let west = a.west.max(b.west);
    let south = a.south.max(b.south);
    let east = a.east.min(b.east);
    let north = a.north.min(b.north);
    south < north && west < east
}

fn subtract_rectangle(rectangle_list: &[CoverageRect], sub: &CoverageRect) -> Vec<CoverageRect> {
    let mut result = Vec::new();
    for rect in rectangle_list {
        if !coverage_rects_overlap(rect, sub) {
            result.push(*rect);
        } else {
            if rect.west < sub.west {
                result.push(CoverageRect { west: rect.west, south: rect.south, east: sub.west, north: rect.north });
            }
            if rect.east > sub.east {
                result.push(CoverageRect { west: sub.east, south: rect.south, east: rect.east, north: rect.north });
            }
            if rect.south < sub.south {
                result.push(CoverageRect {
                    west: sub.west.max(rect.west),
                    south: rect.south,
                    east: sub.east.min(rect.east),
                    north: sub.south,
                });
            }
            if rect.north > sub.north {
                result.push(CoverageRect {
                    west: sub.west.max(rect.west),
                    south: sub.north,
                    east: sub.east.min(rect.east),
                    north: rect.north,
                });
            }
        }
    }
    result
}

impl TileAvailability {
    /// 创建一个新的瓦片可用性跟踪器。
    ///
    /// 映射到 `new TileAvailability(tilingScheme, maximumLevel)`
    pub fn new(tiling_scheme: TilingScheme, maximum_level: u32) -> Self {
        Self {
            tiling_scheme,
            maximum_level,
            root_nodes: Vec::new(),
            nodes: Vec::new(),
        }
    }

    /// 创建一个所有瓦片在 maximum_level 之前均可用的可用性。
    pub fn all(maximum_level: u32) -> Self {
        let mut avail = Self::new(TilingScheme::geographic(), maximum_level);
        let x_tiles = avail.tiling_scheme.number_of_x_tiles_at_level(0);
        let y_tiles = avail.tiling_scheme.number_of_y_tiles_at_level(0);
        avail.add_available_tile_range(0, 0, 0, x_tiles - 1, y_tiles - 1);
        avail.add_available_tile_range(
            maximum_level, 0, 0,
            avail.tiling_scheme.number_of_x_tiles_at_level(maximum_level) - 1,
            avail.tiling_scheme.number_of_y_tiles_at_level(maximum_level) - 1,
        );
        avail
    }

    fn create_node(&mut self, parent: Option<usize>, level: u32, x: u32, y: u32) -> usize {
        let extent = self.tiling_scheme.tile_xy_to_rectangle(x, y, level);
        let idx = self.nodes.len();
        self.nodes.push(AvailabilityNode {
            level,
            x,
            y,
            extent,
            rectangles: Vec::new(),
            parent,
            children: [None; 4],
        });
        idx
    }

    /// 获取或创建给定槽位中的子节点（0=nw, 1=ne, 2=sw, 3=se）。
    fn get_child(&mut self, node_idx: usize, slot: usize) -> usize {
        if let Some(child) = self.nodes[node_idx].children[slot] {
            return child;
        }
        let (level, x, y) = {
            let n = &self.nodes[node_idx];
            let child_level = n.level + 1;
            match slot {
                0 => (child_level, n.x * 2, n.y * 2),         // nw
                1 => (child_level, n.x * 2 + 1, n.y * 2),     // ne
                2 => (child_level, n.x * 2, n.y * 2 + 1),     // sw
                _ => (child_level, n.x * 2 + 1, n.y * 2 + 1), // se
            }
        };
        let child = self.create_node(Some(node_idx), level, x, y);
        self.nodes[node_idx].children[slot] = Some(child);
        child
    }

    /// 将某一特定层级中的一段矩形瓦片范围标记为可用。
    ///
    /// 映射到 `TileAvailability.addAvailableTileRange`
    pub fn add_available_tile_range(
        &mut self,
        level: u32,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
    ) {
        if level == 0 {
            for y in start_y..=end_y {
                for x in start_x..=end_x {
                    let exists = self.root_nodes.iter().any(|&idx| {
                        let n = &self.nodes[idx];
                        n.x == x && n.y == y && n.level == 0
                    });
                    if !exists {
                        let idx = self.create_node(None, 0, x, y);
                        self.root_nodes.push(idx);
                    }
                }
            }
        }

        let start_rect = self.tiling_scheme.tile_xy_to_rectangle(start_x, start_y, level);
        let west = start_rect.west;
        let north = start_rect.north;

        let end_rect = self.tiling_scheme.tile_xy_to_rectangle(end_x, end_y, level);
        let east = end_rect.east;
        let south = end_rect.south;

        let rectangle_with_level = RectangleWithLevel { level, west, south, east, north };

        let root_indices: Vec<usize> = self.root_nodes.clone();
        for &root_idx in &root_indices {
            let (rw, rs, re, rn) = {
                let e = &self.nodes[root_idx].extent;
                (e.west, e.south, e.east, e.north)
            };
            if rectangles_overlap(rw, rs, re, rn, &rectangle_with_level) {
                self.put_rectangle_in_quadtree(root_idx, rectangle_with_level);
            }
        }
    }

    /// 将单个瓦片标记为可用。
    pub fn add_available_tile(&mut self, level: u32, x: u32, y: u32) {
        self.add_available_tile_range(level, x, y, x, y);
    }

    fn put_rectangle_in_quadtree(&mut self, root_idx: usize, rectangle: RectangleWithLevel) {
        let max_depth = self.maximum_level;
        let mut node_idx = root_idx;

        while self.nodes[node_idx].level < max_depth {
            // 依次尝试每个子节点：nw, ne, sw, se
            let mut descended = false;
            for slot in 0..4 {
                let child_idx = self.get_child(node_idx, slot);
                if rectangle_fully_contains(&self.nodes[child_idx].extent.clone(), &rectangle) {
                    node_idx = child_idx;
                    descended = true;
                    break;
                }
            }
            if !descended {
                break;
            }
        }

        let node = &mut self.nodes[node_idx];
        if node.rectangles.is_empty()
            || node.rectangles[node.rectangles.len() - 1].level <= rectangle.level
        {
            node.rectangles.push(rectangle);
        } else {
            // 插入时按层级维持顺序（binarySearch + splice）。
            let index = node
                .rectangles
                .partition_point(|r| r.level < rectangle.level);
            node.rectangles.insert(index, rectangle);
        }
    }

    /// 确定覆盖该位置的最详细瓦片的层级。
    /// 若该位置在裁剪方案之外则返回 -1。
    ///
    /// 映射到 `TileAvailability.computeMaximumLevelAtPosition`
    pub fn compute_maximum_level_at_position(&self, position: &Cartographic) -> i32 {
        // 找到包含该位置的根节点。
        let mut node_idx = None;
        for &root_idx in &self.root_nodes {
            let e = &self.nodes[root_idx].extent;
            if rectangle_contains_position(
                e.west, e.south, e.east, e.north,
                position.longitude, position.latitude,
            ) {
                node_idx = Some(root_idx);
                break;
            }
        }

        match node_idx {
            Some(idx) => self.find_max_level_from_node(None, idx, position),
            None => -1,
        }
    }

    fn find_max_level_from_node(
        &self,
        stop_node: Option<usize>,
        start_node: usize,
        position: &Cartographic,
    ) -> i32 {
        let mut max_level: i32 = 0;
        let (lon, lat) = (position.longitude, position.latitude);

        // 找到包含该点的最深四叉树节点。
        let mut node_idx = start_node;
        loop {
            let children = self.nodes[node_idx].children;
            let mut containing: Vec<usize> = Vec::new();
            for &child_opt in &children {
                if let Some(child_idx) = child_opt {
                    let e = &self.nodes[child_idx].extent;
                    if rectangle_contains_position(e.west, e.south, e.east, e.north, lon, lat) {
                        containing.push(child_idx);
                    }
                }
            }

            if containing.len() > 1 {
                // 点位于瓦片之间的边界上；全部检查。
                for &child_idx in &containing {
                    let level = self.find_max_level_from_node(
                        Some(node_idx), child_idx, position,
                    );
                    max_level = max_level.max(level);
                }
                break;
            } else if containing.len() == 1 {
                node_idx = containing[0];
            } else {
                break;
            }
        }

        // 沿树向上查找，直到找到一个包含该点的矩形。
        let mut current = Some(node_idx);
        while current != stop_node {
            let idx = current.unwrap();
            let rectangles = &self.nodes[idx].rectangles;

            // 矩形按层级排序，最低的在前。
            for i in (0..rectangles.len()).rev() {
                if (rectangles[i].level as i32) <= max_level {
                    break;
                }
                let r = &rectangles[i];
                if rectangle_contains_position(r.west, r.south, r.east, r.north, lon, lat) {
                    max_level = rectangles[i].level as i32;
                }
            }

            current = self.nodes[idx].parent;
        }

        max_level
    }

    /// 查找在给定矩形内 _处处_ 均可用的最详细层级。
    ///
    /// 映射到 `TileAvailability.computeBestAvailableLevelOverRectangle`
    pub fn compute_best_available_level_over_rectangle(&self, rectangle: &Rectangle) -> u32 {
        let mut rectangles_to_cover: Vec<CoverageRect> = Vec::new();

        if rectangle.east < rectangle.west {
            // 矩形跨越 IDL，将其拆为两个矩形。
            rectangles_to_cover.push(CoverageRect {
                west: -PI,
                south: rectangle.south,
                east: rectangle.east,
                north: rectangle.north,
            });
            rectangles_to_cover.push(CoverageRect {
                west: rectangle.west,
                south: rectangle.south,
                east: PI,
                north: rectangle.north,
            });
        } else {
            rectangles_to_cover.push(CoverageRect {
                west: rectangle.west,
                south: rectangle.south,
                east: rectangle.east,
                north: rectangle.north,
            });
        }

        // remainingToCoverByLevel：索引 = 层级
        let mut remaining_to_cover: Vec<Option<Vec<CoverageRect>>> = Vec::new();

        for &root_idx in &self.root_nodes {
            self.update_coverage_with_node(
                &mut remaining_to_cover,
                root_idx,
                &rectangles_to_cover,
            );
        }

        for i in (0..remaining_to_cover.len()).rev() {
            if let Some(ref rects) = remaining_to_cover[i] {
                if rects.is_empty() {
                    return i as u32;
                }
            }
        }

        0
    }

    fn update_coverage_with_node(
        &self,
        remaining: &mut Vec<Option<Vec<CoverageRect>>>,
        node_idx: usize,
        rectangles_to_cover: &[CoverageRect],
    ) {
        let node = &self.nodes[node_idx];

        let any_overlap = rectangles_to_cover.iter().any(|r| {
            let e = &node.extent;
            let sub = CoverageRect { west: e.west, south: e.south, east: e.east, north: e.north };
            coverage_rects_overlap(&sub, r)
        });

        if !any_overlap {
            return;
        }

        for rectangle in &node.rectangles {
            let level = rectangle.level as usize;
            if level >= remaining.len() {
                remaining.resize(level + 1, None);
            }
            if remaining[level].is_none() {
                remaining[level] = Some(rectangles_to_cover.to_vec());
            }
            let sub = CoverageRect {
                west: rectangle.west,
                south: rectangle.south,
                east: rectangle.east,
                north: rectangle.north,
            };
            let current = remaining[level].take().unwrap();
            remaining[level] = Some(subtract_rectangle(&current, &sub));
        }

        // 用子节点更新。
        for &child_opt in &node.children {
            if let Some(child_idx) = child_opt {
                self.update_coverage_with_node(remaining, child_idx, rectangles_to_cover);
            }
        }
    }

    /// 判断某个特定瓦片是否可用。
    ///
    /// 映射到 `TileAvailability.isTileAvailable`
    pub fn is_tile_available(&self, level: u32, x: u32, y: u32) -> bool {
        let rectangle = self.tiling_scheme.tile_xy_to_rectangle(x, y, level);
        let center = rectangle.center();
        self.compute_maximum_level_at_position(&center) >= level as i32
    }

    /// 计算一个位掩码，指示一个瓦片的四个子节点中哪些存在。
    /// Bit 0 (1) = SW, bit 1 (2) = SE, bit 2 (4) = NW, bit 3 (8) = NE.
    ///
    /// 映射到 `TileAvailability.computeChildMaskForTile`
    pub fn compute_child_mask_for_tile(&self, level: u32, x: u32, y: u32) -> u8 {
        let child_level = level + 1;
        if child_level >= self.maximum_level {
            return 0;
        }

        let mut mask: u8 = 0;
        if self.is_tile_available(child_level, 2 * x, 2 * y + 1) {
            mask |= 1;
        }
        if self.is_tile_available(child_level, 2 * x + 1, 2 * y + 1) {
            mask |= 2;
        }
        if self.is_tile_available(child_level, 2 * x, 2 * y) {
            mask |= 4;
        }
        if self.is_tile_available(child_level, 2 * x + 1, 2 * y) {
            mask |= 8;
        }
        mask
    }

    /// 获取某个位置（经度/纬度，以弧度表示）的最佳可用层级。
    pub fn best_available_level(&self, longitude: f64, latitude: f64) -> u32 {
        let pos = Cartographic::from_radians(longitude, latitude, 0.0);
        self.compute_maximum_level_at_position(&pos).max(0) as u32
    }

    /// 返回已分配的四叉树节点数。
    pub fn tile_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geographic_default() {
        let scheme = GeographicTilingScheme::new();
        assert_eq!(scheme.number_of_level_zero_tiles_x, 2);
        assert_eq!(scheme.number_of_level_zero_tiles_y, 1);
        assert_eq!(scheme.number_of_x_tiles_at_level(0), 2);
        assert_eq!(scheme.number_of_y_tiles_at_level(0), 1);
        assert_eq!(scheme.number_of_x_tiles_at_level(1), 4);
        assert_eq!(scheme.number_of_y_tiles_at_level(1), 2);
        assert_eq!(scheme.number_of_x_tiles_at_level(3), 16);
    }

    #[test]
    fn test_geographic_tile_to_rectangle() {
        let scheme = GeographicTilingScheme::new();

        // 层级 0，瓦片 (0,0) 应为西半球
        let rect = scheme.tile_xy_to_rectangle(0, 0, 0);
        assert!((rect.west - (-PI)).abs() < 1e-10);
        assert!((rect.east - 0.0).abs() < 1e-10);
        assert!((rect.south - (-PI / 2.0)).abs() < 1e-10);
        assert!((rect.north - (PI / 2.0)).abs() < 1e-10);

        // 层级 0，瓦片 (1,0) 应为东半球
        let rect = scheme.tile_xy_to_rectangle(1, 0, 0);
        assert!((rect.west - 0.0).abs() < 1e-10);
        assert!((rect.east - PI).abs() < 1e-10);
    }

    #[test]
    fn test_geographic_position_to_tile() {
        let scheme = GeographicTilingScheme::new();

        // 位置 (0, 0) 在层级 0 应位于瓦片 (1, 0)
        let tile = scheme.position_to_tile_xy(0.01, 0.0, 0).unwrap();
        assert_eq!(tile.x, 1);
        assert_eq!(tile.y, 0);

        // 位置 (-90°, 45°) 在层级 0 应位于瓦片 (0, 0)
        let tile = scheme
            .position_to_tile_xy(-PI / 2.0, PI / 4.0, 0)
            .unwrap();
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);
    }

    #[test]
    fn test_geographic_position_outside() {
        let scheme = GeographicTilingScheme::with_options(
            Ellipsoid::WGS84,
            Rectangle::new(0.0, 0.0, 1.0, 1.0),
            1,
            1,
        );

        // 位于矩形之外的位置
        let result = scheme.position_to_tile_xy(2.0, 0.5, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_geographic_native_rectangle() {
        let scheme = GeographicTilingScheme::new();
        let rect = Rectangle::new(-PI / 2.0, -PI / 4.0, PI / 2.0, PI / 4.0);
        let native = scheme.rectangle_to_native_rectangle(&rect);

        assert!((native.west - (-90.0)).abs() < 1e-6);
        assert!((native.south - (-45.0)).abs() < 1e-6);
        assert!((native.east - 90.0).abs() < 1e-6);
        assert!((native.north - 45.0).abs() < 1e-6);
    }

    #[test]
    fn test_web_mercator_default() {
        let scheme = WebMercatorTilingScheme::new();
        assert_eq!(scheme.number_of_level_zero_tiles_x, 1);
        assert_eq!(scheme.number_of_level_zero_tiles_y, 1);
        assert_eq!(scheme.number_of_x_tiles_at_level(1), 2);
        assert_eq!(scheme.number_of_y_tiles_at_level(1), 2);
        assert_eq!(scheme.number_of_x_tiles_at_level(2), 4);
    }

    #[test]
    fn test_web_mercator_project_unproject() {
        // 通过该方案的 WebMercatorProjection 进行往返测试
        let scheme = WebMercatorTilingScheme::new();
        let c = Cartographic::from_radians(0.5, 0.3, 0.0);
        let projected = scheme.projection.project(&c);
        let back = scheme.projection.unproject(projected);

        assert!((0.5 - back.longitude).abs() < 1e-10);
        assert!((0.3 - back.latitude).abs() < 1e-10);
    }

    #[test]
    fn test_web_mercator_project_origin() {
        let scheme = WebMercatorTilingScheme::new();
        let c = Cartographic::from_radians(0.0, 0.0, 0.0);
        let projected = scheme.projection.project(&c);
        assert!(projected.x.abs() < 1e-6);
        assert!(projected.y.abs() < 1e-6);
    }

    #[test]
    fn test_web_mercator_tile_to_rectangle() {
        let scheme = WebMercatorTilingScheme::new();

        // 层级 0，单个瓦片应覆盖完整范围
        let rect = scheme.tile_xy_to_rectangle(0, 0, 0);
        assert!((rect.west - (-PI)).abs() < 1e-6);
        assert!((rect.east - PI).abs() < 1e-6);
        assert!(rect.south < -1.4);
        assert!(rect.north > 1.4);
    }

    #[test]
    fn test_web_mercator_position_to_tile() {
        let scheme = WebMercatorTilingScheme::new();

        // 在层级 1，位置 (0, 0) 应位于瓦片 (1, 1)（中心的右下角）
        let tile = scheme.position_to_tile_xy(0.01, -0.01, 1).unwrap();
        assert_eq!(tile.x, 1);
        assert_eq!(tile.y, 1);

        // 左上象限
        let tile = scheme.position_to_tile_xy(-1.0, 1.0, 1).unwrap();
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);
    }

    #[test]
    fn test_web_mercator_native_rectangle() {
        let scheme = WebMercatorTilingScheme::new();
        let rect = scheme.tile_xy_to_native_rectangle(0, 0, 0);

        let extent = PI * Ellipsoid::WGS84.maximum_radius();
        assert!((rect.west - (-extent)).abs() < 1.0);
        assert!((rect.east - extent).abs() < 1.0);
    }

    #[test]
    fn test_tiling_scheme_enum() {
        let geo = TilingScheme::geographic();
        assert_eq!(geo.number_of_x_tiles_at_level(0), 2);
        assert_eq!(geo.number_of_y_tiles_at_level(0), 1);

        let merc = TilingScheme::web_mercator();
        assert_eq!(merc.number_of_x_tiles_at_level(0), 1);
        assert_eq!(merc.number_of_y_tiles_at_level(0), 1);
    }

    #[test]
    fn test_tile_availability_all() {
        let avail = TileAvailability::all(18);
        assert!(avail.is_tile_available(0, 0, 0));
        assert!(avail.is_tile_available(18, 100, 200));
        assert!(!avail.is_tile_available(19, 0, 0));
    }

    #[test]
    fn test_tile_availability_explicit() {
        let mut avail = TileAvailability::new(TilingScheme::geographic(), 10);
        avail.add_available_tile_range(0, 0, 0, 1, 0);
        avail.add_available_tile(1, 0, 0);
        avail.add_available_tile(1, 1, 0);

        assert!(avail.is_tile_available(0, 0, 0));
        assert!(avail.is_tile_available(1, 0, 0));
        assert!(avail.is_tile_available(1, 1, 0));
        assert!(!avail.is_tile_available(1, 0, 1));
        assert!(!avail.is_tile_available(2, 0, 0));
    }

    #[test]
    fn test_tile_availability_no_duplicate_roots() {
        let mut avail = TileAvailability::new(TilingScheme::geographic(), 10);
        avail.add_available_tile_range(0, 0, 0, 1, 0);
        let count_after_first = avail.tile_count();
        avail.add_available_tile_range(0, 0, 0, 1, 0);
        // 重复的范围不应创建新节点
        assert_eq!(avail.tile_count(), count_after_first);
    }

    #[test]
    fn test_geographic_level2_tiles() {
        let scheme = GeographicTilingScheme::new();

        // 层级 2：8 x 4 瓦片
        assert_eq!(scheme.number_of_x_tiles_at_level(2), 8);
        assert_eq!(scheme.number_of_y_tiles_at_level(2), 4);

        // 层级 2 的瓦片 (0,0) 应为 1/8 宽、1/4 高
        let rect = scheme.tile_xy_to_rectangle(0, 0, 2);
        let expected_width = 2.0 * PI / 8.0;
        let expected_height = PI / 4.0;
        assert!((rect.width() - expected_width).abs() < 1e-10);
        assert!((rect.height() - expected_height).abs() < 1e-10);
    }
}
