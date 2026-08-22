//! Ported from `packages/engine/Source/Core/WebMercatorTilingScheme.js`.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::rectangle::Rectangle;
use crate::tiling_scheme::TilingScheme;
use crate::web_mercator_projection::WebMercatorProjection;

/// A tiling scheme for geometry referenced to a [`WebMercatorProjection`], EPSG:3857.
/// This is the tiling scheme used by Google Maps, Microsoft Bing Maps, and most of ESRI ArcGIS Online.
pub struct WebMercatorTilingScheme {
    ellipsoid: Ellipsoid,
    projection: WebMercatorProjection,
    rectangle: Rectangle,
    rectangle_southwest_in_meters: Cartesian2,
    rectangle_northeast_in_meters: Cartesian2,
    number_of_level_zero_tiles_x: i32,
    number_of_level_zero_tiles_y: i32,
}

impl WebMercatorTilingScheme {
    /// Creates a new WebMercatorTilingScheme.
    pub fn new(
        ellipsoid: Option<Ellipsoid>,
        number_of_level_zero_tiles_x: Option<i32>,
        number_of_level_zero_tiles_y: Option<i32>,
        rectangle_southwest_in_meters: Option<Cartesian2>,
        rectangle_northeast_in_meters: Option<Cartesian2>,
    ) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let number_of_level_zero_tiles_x = number_of_level_zero_tiles_x.unwrap_or(1);
        let number_of_level_zero_tiles_y = number_of_level_zero_tiles_y.unwrap_or(1);
        let projection = WebMercatorProjection::new(Some(ellipsoid));

        let (southwest, northeast) =
            if let (Some(sw), Some(ne)) = (rectangle_southwest_in_meters, rectangle_northeast_in_meters)
            {
                (sw, ne)
            } else {
                let semimajor_axis_times_pi = ellipsoid.maximum_radius() * std::f64::consts::PI;
                (
                    Cartesian2::from_elements_new(-semimajor_axis_times_pi, -semimajor_axis_times_pi),
                    Cartesian2::from_elements_new(semimajor_axis_times_pi, semimajor_axis_times_pi),
                )
            };

        // Compute the rectangle in radians from the meter bounds.
        let sw_cartesian3 = Cartesian3::new(southwest.x, southwest.y, 0.0);
        let sw_carto = projection.unproject(&sw_cartesian3);

        let ne_cartesian3 = Cartesian3::new(northeast.x, northeast.y, 0.0);
        let ne_carto = projection.unproject(&ne_cartesian3);

        let rectangle = Rectangle::from_radians(
            sw_carto.longitude,
            sw_carto.latitude,
            ne_carto.longitude,
            ne_carto.latitude,
        );

        Self {
            ellipsoid,
            projection,
            rectangle,
            rectangle_southwest_in_meters: southwest,
            rectangle_northeast_in_meters: northeast,
            number_of_level_zero_tiles_x,
            number_of_level_zero_tiles_y,
        }
    }

    /// Returns the map projection used by this tiling scheme.
    pub fn projection(&self) -> &WebMercatorProjection {
        &self.projection
    }
}

impl TilingScheme for WebMercatorTilingScheme {
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
        let southwest = Rectangle::southwest(rectangle);
        let northeast = Rectangle::northeast(rectangle);
        let sw_projected = self.projection.project(&southwest);
        let ne_projected = self.projection.project(&northeast);

        result.west = sw_projected.x;
        result.south = sw_projected.y;
        result.east = ne_projected.x;
        result.north = ne_projected.y;
    }

    fn tile_xy_to_native_rectangle(&self, x: i32, y: i32, level: i32, result: &mut Rectangle) {
        let x_tiles = self.get_number_of_x_tiles_at_level(level) as f64;
        let y_tiles = self.get_number_of_y_tiles_at_level(level) as f64;

        let x_tile_width =
            (self.rectangle_northeast_in_meters.x - self.rectangle_southwest_in_meters.x) / x_tiles;
        let west = self.rectangle_southwest_in_meters.x + x as f64 * x_tile_width;
        let east = self.rectangle_southwest_in_meters.x + (x as f64 + 1.0) * x_tile_width;

        let y_tile_height =
            (self.rectangle_northeast_in_meters.y - self.rectangle_southwest_in_meters.y) / y_tiles;
        let north = self.rectangle_northeast_in_meters.y - y as f64 * y_tile_height;
        let south = self.rectangle_northeast_in_meters.y - (y as f64 + 1.0) * y_tile_height;

        result.west = west;
        result.south = south;
        result.east = east;
        result.north = north;
    }

    fn tile_xy_to_rectangle(&self, x: i32, y: i32, level: i32, result: &mut Rectangle) {
        self.tile_xy_to_native_rectangle(x, y, level, result);

        let sw_cartesian3 = Cartesian3::new(result.west, result.south, 0.0);
        let southwest = self.projection.unproject(&sw_cartesian3);

        let ne_cartesian3 = Cartesian3::new(result.east, result.north, 0.0);
        let northeast = self.projection.unproject(&ne_cartesian3);

        result.west = southwest.longitude;
        result.south = southwest.latitude;
        result.east = northeast.longitude;
        result.north = northeast.latitude;
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

        let overall_width =
            self.rectangle_northeast_in_meters.x - self.rectangle_southwest_in_meters.x;
        let x_tile_width = overall_width / x_tiles;
        let overall_height =
            self.rectangle_northeast_in_meters.y - self.rectangle_southwest_in_meters.y;
        let y_tile_height = overall_height / y_tiles;

        let web_mercator_position = self.projection.project(position);
        let distance_from_west = web_mercator_position.x - self.rectangle_southwest_in_meters.x;
        let distance_from_north =
            self.rectangle_northeast_in_meters.y - web_mercator_position.y;

        let mut x_tile_coordinate = (distance_from_west / x_tile_width) as i32;
        if x_tile_coordinate >= x_tiles as i32 {
            x_tile_coordinate = x_tiles as i32 - 1;
        }

        let mut y_tile_coordinate = (distance_from_north / y_tile_height) as i32;
        if y_tile_coordinate >= y_tiles as i32 {
            y_tile_coordinate = y_tiles as i32 - 1;
        }

        result.x = x_tile_coordinate as f64;
        result.y = y_tile_coordinate as f64;
        Some(())
    }
}
