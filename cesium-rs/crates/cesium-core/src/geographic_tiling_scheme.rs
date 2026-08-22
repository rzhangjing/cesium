//! Ported from `packages/engine/Source/Core/GeographicTilingScheme.js`.

use crate::cartesian2::Cartesian2;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::geographic_projection::GeographicProjection;
use crate::math::CesiumMath;
use crate::rectangle::Rectangle;
use crate::tiling_scheme::TilingScheme;

/// A tiling scheme for geometry referenced to a simple [`GeographicProjection`] where
/// longitude and latitude are directly mapped to X and Y. This projection is commonly
/// known as geographic, equirectangular, equidistant cylindrical, or plate carrée.
pub struct GeographicTilingScheme {
    ellipsoid: Ellipsoid,
    rectangle: Rectangle,
    projection: GeographicProjection,
    number_of_level_zero_tiles_x: i32,
    number_of_level_zero_tiles_y: i32,
}

impl GeographicTilingScheme {
    /// Creates a new GeographicTilingScheme.
    pub fn new(
        ellipsoid: Option<Ellipsoid>,
        rectangle: Option<Rectangle>,
        number_of_level_zero_tiles_x: Option<i32>,
        number_of_level_zero_tiles_y: Option<i32>,
    ) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let rectangle = rectangle.unwrap_or(Rectangle::MAX_VALUE);
        let projection = GeographicProjection::new(Some(ellipsoid));
        Self {
            ellipsoid,
            rectangle,
            projection,
            number_of_level_zero_tiles_x: number_of_level_zero_tiles_x.unwrap_or(2),
            number_of_level_zero_tiles_y: number_of_level_zero_tiles_y.unwrap_or(1),
        }
    }

    /// Returns the map projection used by this tiling scheme.
    pub fn projection(&self) -> &GeographicProjection {
        &self.projection
    }
}

impl TilingScheme for GeographicTilingScheme {
    fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    fn rectangle(&self) -> &Rectangle {
        &self.rectangle
    }

    fn get_number_of_x_tiles_at_level(&self, level: i32) -> i32 {
        self.number_of_level_zero_tiles_x << level
    }

    fn get_number_of_y_tiles_at_level(&self, level: i32) -> i32 {
        self.number_of_level_zero_tiles_y << level
    }

    fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle, result: &mut Rectangle) {
        result.west = CesiumMath::to_degrees(rectangle.west);
        result.south = CesiumMath::to_degrees(rectangle.south);
        result.east = CesiumMath::to_degrees(rectangle.east);
        result.north = CesiumMath::to_degrees(rectangle.north);
    }

    fn tile_xy_to_native_rectangle(&self, x: i32, y: i32, level: i32, result: &mut Rectangle) {
        self.tile_xy_to_rectangle(x, y, level, result);
        result.west = CesiumMath::to_degrees(result.west);
        result.south = CesiumMath::to_degrees(result.south);
        result.east = CesiumMath::to_degrees(result.east);
        result.north = CesiumMath::to_degrees(result.north);
    }

    fn tile_xy_to_rectangle(&self, x: i32, y: i32, level: i32, result: &mut Rectangle) {
        let rectangle = &self.rectangle;

        let x_tiles = self.get_number_of_x_tiles_at_level(level) as f64;
        let y_tiles = self.get_number_of_y_tiles_at_level(level) as f64;

        let x_tile_width = rectangle.width() / x_tiles;
        let west = x as f64 * x_tile_width + rectangle.west;
        let east = (x as f64 + 1.0) * x_tile_width + rectangle.west;

        let y_tile_height = rectangle.height() / y_tiles;
        let north = rectangle.north - y as f64 * y_tile_height;
        let south = rectangle.north - (y as f64 + 1.0) * y_tile_height;

        result.west = west;
        result.south = south;
        result.east = east;
        result.north = north;
    }

    fn position_to_tile_xy(
        &self,
        position: &Cartographic,
        level: i32,
        result: &mut Cartesian2,
    ) -> Option<()> {
        let rectangle = &self.rectangle;
        if !Rectangle::contains(rectangle, position) {
            return None;
        }

        let x_tiles = self.get_number_of_x_tiles_at_level(level) as f64;
        let y_tiles = self.get_number_of_y_tiles_at_level(level) as f64;

        let x_tile_width = rectangle.width() / x_tiles;
        let y_tile_height = rectangle.height() / y_tiles;

        let mut longitude = position.longitude;
        if rectangle.east < rectangle.west {
            longitude += CesiumMath::TWO_PI;
        }

        let mut x_tile_coordinate = ((longitude - rectangle.west) / x_tile_width) as i32;
        if x_tile_coordinate >= x_tiles as i32 {
            x_tile_coordinate = x_tiles as i32 - 1;
        }

        let mut y_tile_coordinate = ((rectangle.north - position.latitude) / y_tile_height) as i32;
        if y_tile_coordinate >= y_tiles as i32 {
            y_tile_coordinate = y_tiles as i32 - 1;
        }

        result.x = x_tile_coordinate as f64;
        result.y = y_tile_coordinate as f64;
        Some(())
    }
}
