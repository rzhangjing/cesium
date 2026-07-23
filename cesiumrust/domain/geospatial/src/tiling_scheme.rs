//! Tiling scheme - divides the globe into a grid of tiles.
//! Maps to CesiumJS `Core/GeographicTilingScheme.js`, `Core/WebMercatorTilingScheme.js`

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::projection::{GeographicProjection, MapProjection, WebMercatorProjection};
use crate::rectangle::Rectangle;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Defines how the globe is subdivided into tiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilingScheme {
    /// The projection used by this tiling scheme.
    projection: TilingProjection,
    /// The rectangle covered by the tiling scheme.
    rectangle: Rectangle,
    /// Number of tiles in the X direction at level 0.
    root_tiles_x: u32,
    /// Number of tiles in the Y direction at level 0.
    root_tiles_y: u32,
}

/// Projection variants for tiling schemes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TilingProjection {
    Geographic(GeographicProjection),
    WebMercator(WebMercatorProjection),
}

impl TilingScheme {
    /// Creates a geographic tiling scheme (2 tiles wide, 1 tile tall at level 0).
    /// Maps to `GeographicTilingScheme`
    pub fn geographic(ellipsoid: Ellipsoid) -> Self {
        Self {
            projection: TilingProjection::Geographic(GeographicProjection::new(ellipsoid)),
            rectangle: Rectangle::MAX_VALUE,
            root_tiles_x: 2,
            root_tiles_y: 1,
        }
    }

    /// Creates a Web Mercator tiling scheme (1 tile wide, 1 tile tall at level 0).
    /// Maps to `WebMercatorTilingScheme`
    pub fn web_mercator(ellipsoid: Ellipsoid) -> Self {
        let max_lat = WebMercatorProjection::MAXIMUM_LATITUDE;
        Self {
            projection: TilingProjection::WebMercator(WebMercatorProjection::new(ellipsoid)),
            rectangle: Rectangle::new(-PI, -max_lat, PI, max_lat),
            root_tiles_x: 1,
            root_tiles_y: 1,
        }
    }

    /// Creates a custom tiling scheme.
    pub fn new(
        projection: TilingProjection,
        rectangle: Rectangle,
        root_tiles_x: u32,
        root_tiles_y: u32,
    ) -> Self {
        Self {
            projection,
            rectangle,
            root_tiles_x,
            root_tiles_y,
        }
    }

    /// Gets the projection used by this tiling scheme.
    pub fn projection(&self) -> &TilingProjection {
        &self.projection
    }

    /// Gets the rectangle covered by this tiling scheme.
    pub fn rectangle(&self) -> &Rectangle {
        &self.rectangle
    }

    /// Gets the number of tiles in X at level 0.
    pub fn root_tiles_x(&self) -> u32 {
        self.root_tiles_x
    }

    /// Gets the number of tiles in Y at level 0.
    pub fn root_tiles_y(&self) -> u32 {
        self.root_tiles_y
    }

    /// Computes the number of tiles in X and Y at a given level.
    /// Maps to `TilingScheme.getNumberOfXTilesAtLevel` / `getNumberOfYTilesAtLevel`
    pub fn tiles_at_level(&self, level: u32) -> (u32, u32) {
        let scale = 1u32 << level;
        (self.root_tiles_x * scale, self.root_tiles_y * scale)
    }

    /// Computes the rectangle covered by a tile at the given x, y, level.
    /// Maps to `TilingScheme.tileXYToRectangle`
    pub fn tile_to_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let (tiles_x, tiles_y) = self.tiles_at_level(level);
        let tile_width = self.rectangle.width() / tiles_x as f64;
        let tile_height = self.rectangle.height() / tiles_y as f64;

        let west = self.rectangle.west + x as f64 * tile_width;
        let north = self.rectangle.north - y as f64 * tile_height;

        Rectangle::new(west, north - tile_height, west + tile_width, north)
    }

    /// Computes the native rectangle for a tile (in projected coordinates).
    pub fn tile_to_native_rectangle(&self, x: u32, y: u32, level: u32) -> Rectangle {
        let geo_rect = self.tile_to_rectangle(x, y, level);
        let sw = self.project(&Cartographic::from_radians(geo_rect.west, geo_rect.south, 0.0));
        let ne = self.project(&Cartographic::from_radians(geo_rect.east, geo_rect.north, 0.0));
        Rectangle::new(sw.x, sw.y, ne.x, ne.y)
    }

    /// Determines which tile contains the given cartographic position at a level.
    /// Maps to `TilingScheme.positionToTileXY`
    pub fn position_to_tile(&self, position: &Cartographic, level: u32) -> Option<(u32, u32)> {
        let (tiles_x, tiles_y) = self.tiles_at_level(level);
        let tile_width = self.rectangle.width() / tiles_x as f64;
        let tile_height = self.rectangle.height() / tiles_y as f64;

        let x = ((position.longitude - self.rectangle.west) / tile_width).floor() as i64;
        let y = ((self.rectangle.north - position.latitude) / tile_height).floor() as i64;

        if x < 0 || x >= tiles_x as i64 || y < 0 || y >= tiles_y as i64 {
            return None;
        }

        Some((x as u32, y as u32))
    }

    /// Projects a cartographic position using this tiling scheme's projection.
    pub fn project(&self, cartographic: &Cartographic) -> glam::DVec3 {
        match &self.projection {
            TilingProjection::Geographic(p) => p.project(cartographic),
            TilingProjection::WebMercator(p) => p.project(cartographic),
        }
    }

    /// Unprojects coordinates using this tiling scheme's projection.
    pub fn unproject(&self, projected: glam::DVec3) -> Cartographic {
        match &self.projection {
            TilingProjection::Geographic(p) => p.unproject(projected),
            TilingProjection::WebMercator(p) => p.unproject(projected),
        }
    }

    /// Gets the ellipsoid used by this tiling scheme.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        match &self.projection {
            TilingProjection::Geographic(p) => p.ellipsoid(),
            TilingProjection::WebMercator(p) => p.ellipsoid(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geographic_tiling_scheme_level0() {
        let ts = TilingScheme::geographic(Ellipsoid::WGS84);
        let (nx, ny) = ts.tiles_at_level(0);
        assert_eq!(nx, 2);
        assert_eq!(ny, 1);
    }

    #[test]
    fn test_geographic_tiling_scheme_level1() {
        let ts = TilingScheme::geographic(Ellipsoid::WGS84);
        let (nx, ny) = ts.tiles_at_level(1);
        assert_eq!(nx, 4);
        assert_eq!(ny, 2);
    }

    #[test]
    fn test_web_mercator_tiling_scheme_level0() {
        let ts = TilingScheme::web_mercator(Ellipsoid::WGS84);
        let (nx, ny) = ts.tiles_at_level(0);
        assert_eq!(nx, 1);
        assert_eq!(ny, 1);
    }

    #[test]
    fn test_tile_to_rectangle() {
        let ts = TilingScheme::geographic(Ellipsoid::WGS84);
        // Level 0, tile (0,0) should be the western hemisphere
        let rect = ts.tile_to_rectangle(0, 0, 0);
        assert!((rect.west - (-PI)).abs() < 1e-10);
        assert!((rect.east - 0.0).abs() < 1e-10);
        assert!((rect.south - (-PI / 2.0)).abs() < 1e-10);
        assert!((rect.north - (PI / 2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_position_to_tile() {
        let ts = TilingScheme::geographic(Ellipsoid::WGS84);
        // Position at (0, 0) at level 0 should be tile (1, 0)
        let pos = Cartographic::from_radians(0.001, 0.0, 0.0);
        let (x, y) = ts.position_to_tile(&pos, 0).unwrap();
        assert_eq!(x, 1);
        assert_eq!(y, 0);
    }

    #[test]
    fn test_position_to_tile_out_of_bounds() {
        let ts = TilingScheme::geographic(Ellipsoid::WGS84);
        // Position way outside should return None
        let pos = Cartographic::from_radians(PI + 1.0, 0.0, 0.0);
        assert!(ts.position_to_tile(&pos, 0).is_none());
    }
}
