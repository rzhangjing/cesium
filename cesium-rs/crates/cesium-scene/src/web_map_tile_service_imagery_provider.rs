//! Ported from `packages/engine/Source/Scene/WebMapTileServiceImageryProvider.js`.

use cesium_core::rectangle::Rectangle;
use crate::imagery_provider::ImageryProvider;

/// An imagery provider for WMTS (Web Map Tile Service) servers.
pub struct WebMapTileServiceImageryProvider {
    url: String,
    layer: String,
    style: String,
    tile_matrix_set_id: String,
    rectangle: Rectangle,
    tile_width: u32,
    tile_height: u32,
    maximum_level: Option<u32>,
    is_ready: bool,
}

impl WebMapTileServiceImageryProvider {
    pub fn new(url: &str, layer: &str, style: &str, tile_matrix_set_id: &str) -> Self {
        Self {
            url: url.to_string(),
            layer: layer.to_string(),
            style: style.to_string(),
            tile_matrix_set_id: tile_matrix_set_id.to_string(),
            rectangle: Rectangle::default(),
            tile_width: 256,
            tile_height: 256,
            maximum_level: None,
            is_ready: true,
        }
    }
}

impl ImageryProvider for WebMapTileServiceImageryProvider {
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
