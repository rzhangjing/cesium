//! Tiling schemes for map tile organization.
//!
//! Maps to CesiumJS:
//! - `Core/GeographicTilingScheme.js`
//! - `Core/WebMercatorTilingScheme.js`

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::rectangle::Rectangle;
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
    /// The rectangle covered by the tiling scheme (radians).
    pub rectangle: Rectangle,
    /// Number of tiles in X at level 0.
    pub number_of_level_zero_tiles_x: u32,
    /// Number of tiles in Y at level 0.
    pub number_of_level_zero_tiles_y: u32,
}

impl Default for GeographicTilingScheme {
    fn default() -> Self {
        Self {
            rectangle: Rectangle::MAX_VALUE,
            number_of_level_zero_tiles_x: 2,
            number_of_level_zero_tiles_y: 1,
        }
    }
}

impl GeographicTilingScheme {
    /// Creates a new geographic tiling scheme with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a geographic tiling scheme with custom parameters.
    pub fn with_options(
        rectangle: Rectangle,
        tiles_x: u32,
        tiles_y: u32,
    ) -> Self {
        Self {
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
    pub fn tile_xy_to_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let x_tile_width = self.rectangle.width() / x_tiles as f64;
        let west = self.rectangle.west + x as f64 * x_tile_width;

        let y_tile_height = self.rectangle.height() / y_tiles as f64;
        let north = self.rectangle.north - y as f64 * y_tile_height;

        Rectangle::new(
            west,
            north - y_tile_height,
            west + x_tile_width,
            north,
        )
    }

    /// Converts a position (radians) to tile coordinates at a given level.
    pub fn position_to_tile_xy(
        &self,
        longitude: f64,
        latitude: f64,
        level: u32,
    ) -> Option<TileCoord> {
        if !self.rectangle.contains(longitude, latitude) {
            return None;
        }

        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let x_tile_width = self.rectangle.width() / x_tiles as f64;
        let y_tile_height = self.rectangle.height() / y_tiles as f64;

        let x = ((longitude - self.rectangle.west) / x_tile_width) as u32;
        let y = ((self.rectangle.north - latitude) / y_tile_height) as u32;

        // Clamp to valid range
        let x = x.min(x_tiles - 1);
        let y = y.min(y_tiles - 1);

        Some(TileCoord::new(x, y, level))
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
    pub fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle) -> Rectangle {
        Rectangle::new(
            rectangle.west.to_degrees(),
            rectangle.south.to_degrees(),
            rectangle.east.to_degrees(),
            rectangle.north.to_degrees(),
        )
    }

    /// Converts tile x, y, level to a native rectangle (degrees).
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

/// Maximum latitude for Web Mercator projection (radians).
const MAXIMUM_LATITUDE: f64 = 1.4844222297453324; // ~85.051129 degrees

/// Earth radius for Web Mercator (semi-major axis).
const EARTH_RADIUS: f64 = 6378137.0;

impl Default for WebMercatorTilingScheme {
    fn default() -> Self {
        let rectangle = Rectangle::new(
            -PI,
            -MAXIMUM_LATITUDE,
            PI,
            MAXIMUM_LATITUDE,
        );

        let extent = PI * EARTH_RADIUS;
        Self {
            rectangle,
            number_of_level_zero_tiles_x: 1,
            number_of_level_zero_tiles_y: 1,
            rectangle_southwest_in_meters: (-extent, -extent),
            rectangle_northeast_in_meters: (extent, extent),
        }
    }
}

impl WebMercatorTilingScheme {
    /// Creates a new Web Mercator tiling scheme with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the number of tiles in X at a given level.
    pub fn number_of_x_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_x << level
    }

    /// Gets the number of tiles in Y at a given level.
    pub fn number_of_y_tiles_at_level(&self, level: u32) -> u32 {
        self.number_of_level_zero_tiles_y << level
    }

    /// Projects longitude/latitude (radians) to Web Mercator meters.
    pub fn project(longitude: f64, latitude: f64) -> (f64, f64) {
        let x = longitude * EARTH_RADIUS;
        let y = (PI / 4.0 + latitude / 2.0).tan().ln() * EARTH_RADIUS;
        (x, y)
    }

    /// Unprojects Web Mercator meters to longitude/latitude (radians).
    pub fn unproject(x: f64, y: f64) -> (f64, f64) {
        let longitude = x / EARTH_RADIUS;
        let latitude = (2.0 * (y / EARTH_RADIUS).exp().atan()) - PI / 2.0;
        (longitude, latitude)
    }

    /// Converts tile x, y, level to a rectangle in radians.
    pub fn tile_xy_to_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let (sw_x, sw_y) = self.rectangle_southwest_in_meters;
        let (ne_x, ne_y) = self.rectangle_northeast_in_meters;

        let x_tile_width = (ne_x - sw_x) / x_tiles as f64;
        let y_tile_height = (ne_y - sw_y) / y_tiles as f64;

        let tile_west_m = sw_x + x as f64 * x_tile_width;
        let tile_south_m = ne_y - (y + 1) as f64 * y_tile_height;
        let tile_east_m = tile_west_m + x_tile_width;
        let tile_north_m = tile_south_m + y_tile_height;

        let (west, south) = Self::unproject(tile_west_m, tile_south_m);
        let (east, north) = Self::unproject(tile_east_m, tile_north_m);

        Rectangle::new(west, south, east, north)
    }

    /// Converts a position (radians) to tile coordinates at a given level.
    pub fn position_to_tile_xy(
        &self,
        longitude: f64,
        latitude: f64,
        level: u32,
    ) -> Option<TileCoord> {
        let latitude = latitude.clamp(-MAXIMUM_LATITUDE, MAXIMUM_LATITUDE);

        let (mx, my) = Self::project(longitude, latitude);

        let (sw_x, sw_y) = self.rectangle_southwest_in_meters;
        let (ne_x, ne_y) = self.rectangle_northeast_in_meters;

        if mx < sw_x || mx > ne_x || my < sw_y || my > ne_y {
            return None;
        }

        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let x_tile_width = (ne_x - sw_x) / x_tiles as f64;
        let y_tile_height = (ne_y - sw_y) / y_tiles as f64;

        let x = ((mx - sw_x) / x_tile_width) as u32;
        let y = ((ne_y - my) / y_tile_height) as u32;

        let x = x.min(x_tiles - 1);
        let y = y.min(y_tiles - 1);

        Some(TileCoord::new(x, y, level))
    }

    /// Converts a cartographic position to tile coordinates.
    pub fn cartographic_to_tile_xy(
        &self,
        cartographic: &Cartographic,
        level: u32,
    ) -> Option<TileCoord> {
        self.position_to_tile_xy(cartographic.longitude, cartographic.latitude, level)
    }

    /// Transforms a rectangle to native coordinates (Web Mercator meters).
    pub fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle) -> Rectangle {
        let (sw_x, sw_y) = Self::project(rectangle.west, rectangle.south);
        let (ne_x, ne_y) = Self::project(rectangle.east, rectangle.north);
        Rectangle::new(sw_x, sw_y, ne_x, ne_y)
    }

    /// Converts tile x, y, level to a native rectangle (meters).
    pub fn tile_xy_to_native_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let x_tiles = self.number_of_x_tiles_at_level(level);
        let y_tiles = self.number_of_y_tiles_at_level(level);

        let (sw_x, sw_y) = self.rectangle_southwest_in_meters;
        let (ne_x, ne_y) = self.rectangle_northeast_in_meters;

        let x_tile_width = (ne_x - sw_x) / x_tiles as f64;
        let y_tile_height = (ne_y - sw_y) / y_tiles as f64;

        let west = sw_x + x as f64 * x_tile_width;
        let north = ne_y - y as f64 * y_tile_height;

        Rectangle::new(
            west,
            north - y_tile_height,
            west + x_tile_width,
            north,
        )
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

/// Tracks tile availability across levels.
///
/// Maps to CesiumJS `Core/TileAvailability.js`
#[derive(Debug, Clone)]
pub struct TileAvailability {
    /// Maximum level tracked.
    pub maximum_level: u32,
    /// Available tiles per level: (level, x, y) tuples.
    available_tiles: Vec<(u32, u32, u32)>,
    /// Whether all tiles are assumed available.
    all_available: bool,
}

impl TileAvailability {
    /// Creates a new tile availability tracker.
    pub fn new(maximum_level: u32) -> Self {
        Self {
            maximum_level,
            available_tiles: Vec::new(),
            all_available: false,
        }
    }

    /// Creates an availability where all tiles are available.
    pub fn all(maximum_level: u32) -> Self {
        Self {
            maximum_level,
            available_tiles: Vec::new(),
            all_available: true,
        }
    }

    /// Marks a tile as available.
    pub fn add_available_tile(&mut self, level: u32, x: u32, y: u32) {
        if level <= self.maximum_level {
            let entry = (level, x, y);
            if !self.available_tiles.contains(&entry) {
                self.available_tiles.push(entry);
            }
        }
    }

    /// Checks if a tile is available.
    pub fn is_tile_available(&self, level: u32, x: u32, y: u32) -> bool {
        if self.all_available {
            return level <= self.maximum_level;
        }
        self.available_tiles.contains(&(level, x, y))
    }

    /// Gets the best available level for a position.
    pub fn best_available_level(&self, _longitude: f64, _latitude: f64) -> u32 {
        if self.all_available {
            return self.maximum_level;
        }
        self.available_tiles
            .iter()
            .map(|(level, _, _)| *level)
            .max()
            .unwrap_or(0)
    }

    /// Returns the number of explicitly tracked tiles.
    pub fn tile_count(&self) -> usize {
        self.available_tiles.len()
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
        // Round-trip test
        let lon = 0.5;
        let lat = 0.3;
        let (x, y) = WebMercatorTilingScheme::project(lon, lat);
        let (lon2, lat2) = WebMercatorTilingScheme::unproject(x, y);

        assert!((lon - lon2).abs() < 1e-10);
        assert!((lat - lat2).abs() < 1e-10);
    }

    #[test]
    fn test_web_mercator_project_origin() {
        let (x, y) = WebMercatorTilingScheme::project(0.0, 0.0);
        assert!(x.abs() < 1e-6);
        assert!(y.abs() < 1e-6);
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

        let extent = PI * EARTH_RADIUS;
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
        let mut avail = TileAvailability::new(10);
        avail.add_available_tile(0, 0, 0);
        avail.add_available_tile(1, 0, 0);
        avail.add_available_tile(1, 1, 0);

        assert!(avail.is_tile_available(0, 0, 0));
        assert!(avail.is_tile_available(1, 0, 0));
        assert!(avail.is_tile_available(1, 1, 0));
        assert!(!avail.is_tile_available(1, 0, 1));
        assert!(!avail.is_tile_available(2, 0, 0));
        assert_eq!(avail.tile_count(), 3);
    }

    #[test]
    fn test_tile_availability_no_duplicates() {
        let mut avail = TileAvailability::new(10);
        avail.add_available_tile(0, 0, 0);
        avail.add_available_tile(0, 0, 0);
        assert_eq!(avail.tile_count(), 1);
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
