//! Ported from `packages/engine/Source/Scene/WebMapServiceImageryProvider.js`.

use cesium_core::rectangle::Rectangle;
use crate::imagery_provider::ImageryProvider;

/// An imagery provider for WMS (Web Map Service) servers.
pub struct WebMapServiceImageryProvider {
    url: String,
    layers: String,
    parameters: std::collections::HashMap<String, String>,
    rectangle: Rectangle,
    tile_width: u32,
    tile_height: u32,
    is_ready: bool,
}

impl WebMapServiceImageryProvider {
    pub fn new(url: &str, layers: &str) -> Self {
        Self {
            url: url.to_string(),
            layers: layers.to_string(),
            parameters: std::collections::HashMap::new(),
            rectangle: Rectangle::default(),
            tile_width: 256,
            tile_height: 256,
            is_ready: true,
        }
    }

    pub fn layers(&self) -> &str { &self.layers }
}

impl ImageryProvider for WebMapServiceImageryProvider {
    fn url(&self) -> &str { &self.url }
    fn proxy(&self) -> Option<&str> { None }
    fn rectangle(&self) -> &Rectangle { &self.rectangle }
    fn tile_width(&self) -> u32 { self.tile_width }
    fn tile_height(&self) -> u32 { self.tile_height }
    fn maximum_level(&self) -> Option<u32> { None }
    fn minimum_level(&self) -> Option<u32> { None }
    fn has_water_mask(&self) -> bool { false }
    fn is_ready(&self) -> bool { self.is_ready }
    fn request_image(&self, _x: u32, _y: u32, _level: u32) -> Option<Vec<u8>> { None }
}
