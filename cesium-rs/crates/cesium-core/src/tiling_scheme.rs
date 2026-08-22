//! Ported from `packages/engine/Source/Core/TilingScheme.js`.

use crate::cartesian2::Cartesian2;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::rectangle::Rectangle;

/// A tiling scheme for geometry or imagery on the surface of an ellipsoid.
///
/// This is the Rust trait equivalent of the JS `TilingScheme` abstract class.
/// Implementors: [`GeographicTilingScheme`](crate::geographic_tiling_scheme::GeographicTilingScheme),
/// [`WebMercatorTilingScheme`](crate::web_mercator_tiling_scheme::WebMercatorTilingScheme).
pub trait TilingScheme {
    /// Gets the ellipsoid that is tiled by the tiling scheme.
    fn ellipsoid(&self) -> &Ellipsoid;

    /// Gets the rectangle, in radians, covered by this tiling scheme.
    fn rectangle(&self) -> &Rectangle;

    /// Gets the total number of tiles in the X direction at a specified level-of-detail.
    fn get_number_of_x_tiles_at_level(&self, level: i32) -> i32;

    /// Gets the total number of tiles in the Y direction at a specified level-of-detail.
    fn get_number_of_y_tiles_at_level(&self, level: i32) -> i32;

    /// Transforms a rectangle specified in geodetic radians to the native coordinate system
    /// of this tiling scheme.
    fn rectangle_to_native_rectangle(&self, rectangle: &Rectangle, result: &mut Rectangle);

    /// Converts tile x, y coordinates and level to a rectangle expressed in the native coordinates
    /// of the tiling scheme.
    fn tile_xy_to_native_rectangle(&self, x: i32, y: i32, level: i32, result: &mut Rectangle);

    /// Converts tile x, y coordinates and level to a cartographic rectangle in radians.
    fn tile_xy_to_rectangle(&self, x: i32, y: i32, level: i32, result: &mut Rectangle);

    /// Calculates the tile x, y coordinates of the tile containing a given cartographic position.
    fn position_to_tile_xy(
        &self,
        position: &Cartographic,
        level: i32,
        result: &mut Cartesian2,
    ) -> Option<()>;
}
