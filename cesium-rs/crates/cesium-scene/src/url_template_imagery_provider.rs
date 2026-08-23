//! Ported from `packages/engine/Source/Scene/UrlTemplateImageryProvider.js`.
//!
//! An imagery provider that uses a URL template to request tiles.

use cesium_core::rectangle::Rectangle;
use crate::imagery_provider::ImageryProvider;

/// An imagery provider that uses a URL template to request tiles.
///
/// The URL template can contain placeholders like {x}, {y}, {z}, {s}
/// that are replaced with the tile coordinates.
pub struct UrlTemplateImageryProvider {
    url: String,
    rectangle: Rectangle,
    tile_width: u32,
    tile_height: u32,
    maximum_level: Option<u32>,
    minimum_level: Option<u32>,
    is_ready: bool,
}

impl UrlTemplateImageryProvider {
    /// Creates a new URL template imagery provider.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            rectangle: Rectangle::default(),
            tile_width: 256,
            tile_height: 256,
            maximum_level: None,
            minimum_level: None,
            is_ready: true,
        }
    }

    /// Expands the URL template for the given tile coordinates.
    pub fn expand_url(&self, x: u32, y: u32, level: u32) -> String {
        self.url
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
            .replace("{z}", &level.to_string())
    }
}

impl ImageryProvider for UrlTemplateImageryProvider {
    fn url(&self) -> &str { &self.url }
    fn proxy(&self) -> Option<&str> { None }
    fn rectangle(&self) -> &Rectangle { &self.rectangle }
    fn tile_width(&self) -> u32 { self.tile_width }
    fn tile_height(&self) -> u32 { self.tile_height }
    fn maximum_level(&self) -> Option<u32> { self.maximum_level }
    fn minimum_level(&self) -> Option<u32> { self.minimum_level }
    fn has_water_mask(&self) -> bool { false }
    fn is_ready(&self) -> bool { self.is_ready }
    fn request_image(&self, _x: u32, _y: u32, _level: u32) -> Option<Vec<u8>> { None }
}
