//! Ported from `packages/engine/Source/Scene/TileMapServiceImageryProvider.js`.

use cesium_core::rectangle::Rectangle;
use crate::imagery_provider::ImageryProvider;

/// An imagery provider for TMS (Tile Map Service) servers.
pub struct TileMapServiceImageryProvider {
    url: String,
    rectangle: Rectangle,
    tile_width: u32,
    tile_height: u32,
    maximum_level: Option<u32>,
    file_extension: String,
    is_ready: bool,
}

impl TileMapServiceImageryProvider {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            rectangle: Rectangle::default(),
            tile_width: 256,
            tile_height: 256,
            maximum_level: None,
            file_extension: "png".to_string(),
            is_ready: false,
        }
    }
}

impl ImageryProvider for TileMapServiceImageryProvider {
    fn url(&self) -> &str { &self.url }
    fn proxy(&self) -> Option<&str> { None }
    fn rectangle(&self) -> &Rectangle { &self.rectangle }
    fn tile_width(&self) -> u32 { self.tile_width }
    fn tile_height(&self) -> u32 { self.tile_height }
    fn maximum_level(&self) -> Option<u32> { self.maximum_level }
    fn minimum_level(&self) -> Option<u32> { None }
    fn has_water_mask(&self) -> bool { false }
    fn is_ready(&self) -> bool { self.is_ready }
    fn request_image(&self, _x: u32, _y: u32, _level: u32) -> Option<Vec<u8>> { None }
}
