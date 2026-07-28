//! Tiling schemes for map tile organization.
//!
//! Maps to CesiumJS:
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

/// A tiling scheme for dividing the globe into tiles.
///
/// Maps to CesiumJS `GeographicTilingScheme` and `WebMercatorTilingScheme`
#[derive(Debug, Clone)]
pub enum TilingScheme {
    /// Geographic (EPSG:4326) tiling scheme.
    /// Default: 2 tiles wide, 1 tile tall at level 0.
    Geographic(GeographicTilingScheme),
    /// Web Mercator (EPSG:3857) tiling scheme.
    /// Default: 1 tile wide, 1 tile tall at level 0.
    WebMercator(WebMercatorTilingScheme),
}

/// Geographic (EPSG:4326) tiling scheme.
///
/// Maps to CesiumJS `Core/GeographicTilingScheme.js`
#[derive(Debug, Clone)]
pub struct GeographicTilingScheme {
    /// The ellipsoid that is tiled by this tiling scheme.
    pub ellipsoid: Ellipsoid,
    /// The map projection used by this tiling scheme.
    pub projection: GeographicProjection,
    /// The rectangle covered by the tiling scheme (radians).
    pub rectangle: Rectangle,
    /// Number of tiles in X at level 0.
    pub number_of_level_zero_tiles_x: u32,
    /// Number of tiles in Y at level 0.
    pub number_of_level_zero_tiles_y: u32,
}

impl Default for GeographicTilingScheme {
    fn default() -> Self {
        Self::with_options(Ellipsoid::WGS84, Rectangle::MAX_VALUE, 2, 1)
    }
}

impl GeographicTilingScheme {
    /// Creates a new geographic tiling scheme with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a geographic tiling scheme with custom parameters.
    ///
    /// Maps to the CesiumJS constructor options (`ellipsoid`, `rectangle`,
    /// `numberOfLevelZeroTilesX`, `numberOfLevelZeroTilesY`).
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

    /// Gets the number of tiles in X at a given level.
    pub fn number_of_x_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_x << level
    }

    /// Gets the number of tiles in Y at a given level.
    pub fn number_of_y_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_y << level
    }

    /// Converts tile x, y, level to a rectangle in radians.
    ///
    /// Maps to `GeographicTilingScheme.tileXYToRectangle`
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

    /// Converts a position (radians) to tile coordinates at a given level.
    ///
    /// Maps to `GeographicTilingScheme.positionToTileXY`
    pub fn position_to_tile_xy(
        &self,
        longitude: f64,
        latitude: f64,
        level: u32,
    ) -> Option<TileCoord> {
        let rectangle = &self.rectangle;
        if !rectangle.contains(longitude, latitude) {
            // outside the bounds of the tiling scheme
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

        // JS `| 0` truncates toward zero; Rust `as i64` does the same.
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

    /// Converts a cartographic position to tile coordinates.
    pub fn cartographic_to_tile_xy(
        &self,
        cartographic: &Cartographic,
        level: u32,
    ) -> Option<TileCoord> {
        self.position_to_tile_xy(cartographic.longitude, cartographic.latitude, level)
    }

    /// Transforms a rectangle to native coordinates (degrees for geographic).
    ///
    /// Maps to `GeographicTilingScheme.rectangleToNativeRectangle`
    pub fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle) -> Rectangle {
        Rectangle::new(
            to_degrees(rectangle.west),
            to_degrees(rectangle.south),
            to_degrees(rectangle.east),
            to_degrees(rectangle.north),
        )
    }

    /// Converts tile x, y, level to a native rectangle (degrees).
    ///
    /// Maps to `GeographicTilingScheme.tileXYToNativeRectangle`
    pub fn tile_xy_to_native_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let rect = self.tile_xy_to_rectangle(x, y, level);
        self.rectangle_to_native_rectangle(&rect)
    }
}

/// Web Mercator (EPSG:3857) tiling scheme.
///
/// Maps to CesiumJS `Core/WebMercatorTilingScheme.js`
#[derive(Debug, Clone)]
pub struct WebMercatorTilingScheme {
    /// The ellipsoid that is tiled by this tiling scheme.
    pub ellipsoid: Ellipsoid,
    /// The map projection used by this tiling scheme.
    pub projection: WebMercatorProjection,
    /// The rectangle covered (radians, clamped to Mercator bounds).
    pub rectangle: Rectangle,
    /// Number of tiles in X at level 0.
    pub number_of_level_zero_tiles_x: u32,
    /// Number of tiles in Y at level 0.
    pub number_of_level_zero_tiles_y: u32,
    /// Southwest corner in projected meters.
    pub rectangle_southwest_in_meters: (f64, f64),
    /// Northeast corner in projected meters.
    pub rectangle_northeast_in_meters: (f64, f64),
}

impl Default for WebMercatorTilingScheme {
    fn default() -> Self {
        Self::with_options(Ellipsoid::WGS84, 1, 1, None, None)
    }
}

impl WebMercatorTilingScheme {
    /// Creates a new Web Mercator tiling scheme with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a Web Mercator tiling scheme for a custom ellipsoid.
    pub fn with_ellipsoid(ellipsoid: Ellipsoid) -> Self {
        Self::with_options(ellipsoid, 1, 1, None, None)
    }

    /// Creates a Web Mercator tiling scheme covering a custom rectangle given
    /// by its southwest/northeast corners in projected meters.
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

    /// Creates a Web Mercator tiling scheme with full custom options.
    ///
    /// Maps to the CesiumJS constructor options (`ellipsoid`,
    /// `numberOfLevelZeroTilesX/Y`, `rectangleSouthwestInMeters`,
    /// `rectangleNortheastInMeters`).
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

    /// Gets the number of tiles in X at a given level.
    pub fn number_of_x_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_x << level
    }

    /// Gets the number of tiles in Y at a given level.
    pub fn number_of_y_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_y << level
    }

    /// Transforms a rectangle to native coordinates (Web Mercator meters).
    ///
    /// Maps to `WebMercatorTilingScheme.rectangleToNativeRectangle`
    pub fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle) -> Rectangle {
        let southwest = self.projection.project(&rectangle.southwest());
        let northeast = self.projection.project(&rectangle.northeast());
        Rectangle::new(southwest.x, southwest.y, northeast.x, northeast.y)
    }

    /// Converts tile x, y, level to a native rectangle (meters).
    ///
    /// Maps to `WebMercatorTilingScheme.tileXYToNativeRectangle`
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

    /// Converts tile x, y, level to a rectangle in radians.
    ///
    /// Maps to `WebMercatorTilingScheme.tileXYToRectangle`
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

    /// Converts a position (radians) to tile coordinates at a given level.
    ///
    /// Maps to `WebMercatorTilingScheme.positionToTileXY`
    pub fn position_to_tile_xy(
        &self,
        longitude: f64,
        latitude: f64,
        level: u32,
    ) -> Option<TileCoord> {
        let rectangle = &self.rectangle;
        if !rectangle.contains(longitude, latitude) {
            // outside the bounds of the tiling scheme
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

        // JS `| 0` truncates toward zero; Rust `as i64` does the same.
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

    /// Converts a cartographic position to tile coordinates.
    pub fn cartographic_to_tile_xy(
        &self,
        cartographic: &Cartographic,
        level: u32,
    ) -> Option<TileCoord> {
        self.position_to_tile_xy(cartographic.longitude, cartographic.latitude, level)
    }
}

impl TilingScheme {
    /// Creates a default geographic tiling scheme.
    pub fn geographic() -> Self {
        Self::Geographic(GeographicTilingScheme::default())
    }

    /// Creates a default Web Mercator tiling scheme.
    pub fn web_mercator() -> Self {
        Self::WebMercator(WebMercatorTilingScheme::default())
    }

    /// Gets the number of tiles in X at a given level.
    pub fn number_of_x_tiles_at_level(&self, level: u32) -> u32 {
        match self {
            Self::Geographic(g) => g.number_of_x_tiles_at_level(level),
            Self::WebMercator(w) => w.number_of_x_tiles_at_level(level),
        }
    }

    /// Gets the number of tiles in Y at a given level.
    pub fn number_of_y_tiles_at_level(&self, level: u32) -> u32 {
        match self {
            Self::Geographic(g) => g.number_of_y_tiles_at_level(level),
            Self::WebMercator(w) => w.number_of_y_tiles_at_level(level),
        }
    }

    /// Converts tile coordinates to a rectangle in radians.
    pub fn tile_xy_to_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        match self {
            Self::Geographic(g) => g.tile_xy_to_rectangle(x, y, level),
            Self::WebMercator(w) => w.tile_xy_to_rectangle(x, y, level),
        }
    }

    /// Converts a position to tile coordinates.
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

    /// Gets the rectangle covered by this tiling scheme.
    pub fn rectangle(&self) -> &Rectangle {
        match self {
            Self::Geographic(g) => &g.rectangle,
            Self::WebMercator(w) => &w.rectangle,
        }
    }
}

// ─── TileAvailability (faithful port of Core/TileAvailability.js) ────────────

/// A rectangle tagged with an availability level.
#[derive(Debug, Clone, Copy)]
struct RectangleWithLevel {
    level: u32,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
}

/// Internal quadtree node (slab-allocated).
#[derive(Debug, Clone)]
struct AvailabilityNode {
    level: u32,
    x: u32,
    y: u32,
    extent: Rectangle,
    rectangles: Vec<RectangleWithLevel>,
    parent: Option<usize>,
    /// Children: [nw, ne, sw, se], lazily created.
    children: [Option<usize>; 4],
}

/// Reports the availability of tiles in a tiling scheme.
///
/// Maps to CesiumJS `Core/TileAvailability.js`
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

/// A simple rectangle used in coverage subtraction.
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
    /// Creates a new tile availability tracker.
    ///
    /// Maps to `new TileAvailability(tilingScheme, maximumLevel)`
    pub fn new(tiling_scheme: TilingScheme, maximum_level: u32) -> Self {
        Self {
            tiling_scheme,
            maximum_level,
            root_nodes: Vec::new(),
            nodes: Vec::new(),
        }
    }

    /// Creates an availability where all tiles are available up to maximum_level.
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

    /// Gets or creates the child node in the given slot (0=nw, 1=ne, 2=sw, 3=se).
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

    /// Marks a rectangular range of tiles in a particular level as being available.
    ///
    /// Maps to `TileAvailability.addAvailableTileRange`
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

    /// Marks a single tile as available.
    pub fn add_available_tile(&mut self, level: u32, x: u32, y: u32) {
        self.add_available_tile_range(level, x, y, x, y);
    }

    fn put_rectangle_in_quadtree(&mut self, root_idx: usize, rectangle: RectangleWithLevel) {
        let max_depth = self.maximum_level;
        let mut node_idx = root_idx;

        while self.nodes[node_idx].level < max_depth {
            // Try each child: nw, ne, sw, se
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
            // Maintain ordering by level when inserting (binarySearch + splice).
            let index = node
                .rectangles
                .partition_point(|r| r.level < rectangle.level);
            node.rectangles.insert(index, rectangle);
        }
    }

    /// Determines the level of the most detailed tile covering the position.
    /// Returns -1 if the position is outside the tiling scheme.
    ///
    /// Maps to `TileAvailability.computeMaximumLevelAtPosition`
    pub fn compute_maximum_level_at_position(&self, position: &Cartographic) -> i32 {
        // Find the root node that contains this position.
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

        // Find the deepest quadtree node containing this point.
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
                // Point is on a boundary between tiles; check all of them.
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

        // Work up the tree until we find a rectangle that contains this point.
        let mut current = Some(node_idx);
        while current != stop_node {
            let idx = current.unwrap();
            let rectangles = &self.nodes[idx].rectangles;

            // Rectangles are sorted by level, lowest first.
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

    /// Finds the most detailed level that is available _everywhere_ within a
    /// given rectangle.
    ///
    /// Maps to `TileAvailability.computeBestAvailableLevelOverRectangle`
    pub fn compute_best_available_level_over_rectangle(&self, rectangle: &Rectangle) -> u32 {
        let mut rectangles_to_cover: Vec<CoverageRect> = Vec::new();

        if rectangle.east < rectangle.west {
            // Rectangle crosses the IDL, make it two rectangles.
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

        // remainingToCoverByLevel: index = level
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

        // Update with child nodes.
        for &child_opt in &node.children {
            if let Some(child_idx) = child_opt {
                self.update_coverage_with_node(remaining, child_idx, rectangles_to_cover);
            }
        }
    }

    /// Determines if a particular tile is available.
    ///
    /// Maps to `TileAvailability.isTileAvailable`
    pub fn is_tile_available(&self, level: u32, x: u32, y: u32) -> bool {
        let rectangle = self.tiling_scheme.tile_xy_to_rectangle(x, y, level);
        let center = rectangle.center();
        self.compute_maximum_level_at_position(&center) >= level as i32
    }

    /// Computes a bit mask indicating which of a tile's four children exist.
    /// Bit 0 (1) = SW, bit 1 (2) = SE, bit 2 (4) = NW, bit 3 (8) = NE.
    ///
    /// Maps to `TileAvailability.computeChildMaskForTile`
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

    /// Gets the best available level for a position (longitude/latitude in radians).
    pub fn best_available_level(&self, longitude: f64, latitude: f64) -> u32 {
        let pos = Cartographic::from_radians(longitude, latitude, 0.0);
        self.compute_maximum_level_at_position(&pos).max(0) as u32
    }

    /// Returns the number of quadtree nodes allocated.
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

        // Level 0, tile (0,0) should be western hemisphere
        let rect = scheme.tile_xy_to_rectangle(0, 0, 0);
        assert!((rect.west - (-PI)).abs() < 1e-10);
        assert!((rect.east - 0.0).abs() < 1e-10);
        assert!((rect.south - (-PI / 2.0)).abs() < 1e-10);
        assert!((rect.north - (PI / 2.0)).abs() < 1e-10);

        // Level 0, tile (1,0) should be eastern hemisphere
        let rect = scheme.tile_xy_to_rectangle(1, 0, 0);
        assert!((rect.west - 0.0).abs() < 1e-10);
        assert!((rect.east - PI).abs() < 1e-10);
    }

    #[test]
    fn test_geographic_position_to_tile() {
        let scheme = GeographicTilingScheme::new();

        // Position at (0, 0) should be in tile (1, 0) at level 0
        let tile = scheme.position_to_tile_xy(0.01, 0.0, 0).unwrap();
        assert_eq!(tile.x, 1);
        assert_eq!(tile.y, 0);

        // Position at (-90°, 45°) should be in tile (0, 0) at level 0
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

        // Position outside the rectangle
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
        // Round-trip test via the scheme's WebMercatorProjection
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

        // Level 0, single tile should cover the full extent
        let rect = scheme.tile_xy_to_rectangle(0, 0, 0);
        assert!((rect.west - (-PI)).abs() < 1e-6);
        assert!((rect.east - PI).abs() < 1e-6);
        assert!(rect.south < -1.4);
        assert!(rect.north > 1.4);
    }

    #[test]
    fn test_web_mercator_position_to_tile() {
        let scheme = WebMercatorTilingScheme::new();

        // At level 1, position (0, 0) should be in tile (1, 1) (bottom-right of center)
        let tile = scheme.position_to_tile_xy(0.01, -0.01, 1).unwrap();
        assert_eq!(tile.x, 1);
        assert_eq!(tile.y, 1);

        // Top-left quadrant
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
        // No new nodes should be created by a duplicate range
        assert_eq!(avail.tile_count(), count_after_first);
    }

    #[test]
    fn test_geographic_level2_tiles() {
        let scheme = GeographicTilingScheme::new();

        // Level 2: 8 x 4 tiles
        assert_eq!(scheme.number_of_x_tiles_at_level(2), 8);
        assert_eq!(scheme.number_of_y_tiles_at_level(2), 4);

        // Tile (0,0) at level 2 should be 1/8 width, 1/4 height
        let rect = scheme.tile_xy_to_rectangle(0, 0, 2);
        let expected_width = 2.0 * PI / 8.0;
        let expected_height = PI / 4.0;
        assert!((rect.width() - expected_width).abs() < 1e-10);
        assert!((rect.height() - expected_height).abs() < 1e-10);
    }
}
