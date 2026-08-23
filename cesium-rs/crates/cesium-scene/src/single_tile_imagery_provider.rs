//! Ported from `packages/engine/Source/Scene/SingleTileImageryProvider.js`.

use cesium_core::rectangle::Rectangle;
use crate::imagery_provider::ImageryProvider;

/// An imagery provider that uses a single image for the entire globe.
pub struct SingleTileImageryProvider {
    url: String,
    rectangle: Rectangle,
    tile_width: u32,
    tile_height: u32,
    is_ready: bool,
}

impl SingleTileImageryProvider {
    pub fn new(url: &str, rectangle: Rectangle) -> Self {
        Self {
            url: url.to_string(),
            rectangle,
            tile_width: 256,
            tile_height: 256,
            is_ready: true,
        }
    }
}

impl ImageryProvider for SingleTileImageryProvider {
    fn url(&self) -> &str { &self.url }
    fn proxy(&self) -> Option<&str> { None }
    fn rectangle(&self) -> &Rectangle { &self.rectangle }
    fn tile_width(&self) -> u32 { self.tile_width }
    fn tile_height(&self) -> u32 { self.tile_height }
    fn maximum_level(&self) -> Option<u32> { Some(0) }
    fn minimum_level(&self) -> Option<u32> { Some(0) }
    fn has_water_mask(&self) -> bool { false }
    fn is_ready(&self) -> bool { self.is_ready }
    fn request_image(&self, _x: u32, _y: u32, _level: u32) -> Option<Vec<u8>> { None }
}
