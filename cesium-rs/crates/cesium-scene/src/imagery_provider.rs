//! Ported from `packages/engine/Source/Scene/ImageryProvider.js`.
//!
//! Base trait for all imagery providers.

use cesium_core::rectangle::Rectangle;

/// Base trait for all imagery providers.
///
/// An imagery provider loads image tiles for a specific imagery service.
pub trait ImageryProvider {
    /// Returns the URL of the imagery service.
    fn url(&self) -> &str;

    /// Returns the proxy used by this provider.
    fn proxy(&self) -> Option<&str>;

    /// Returns the rectangle of the imagery.
    fn rectangle(&self) -> &Rectangle;

    /// Returns the width of each tile in pixels.
    fn tile_width(&self) -> u32;

    /// Returns the height of each tile in pixels.
    fn tile_height(&self) -> u32;

    /// Returns the maximum tile level.
    fn maximum_level(&self) -> Option<u32>;

    /// Returns the minimum tile level.
    fn minimum_level(&self) -> Option<u32>;

    /// Returns whether this provider has a watermark.
    fn has_water_mask(&self) -> bool;

    /// Returns whether this provider is ready.
    fn is_ready(&self) -> bool;

    /// Requests a tile image at the given coordinates.
    fn request_image(&self, x: u32, y: u32, level: u32) -> Option<Vec<u8>>;
}
