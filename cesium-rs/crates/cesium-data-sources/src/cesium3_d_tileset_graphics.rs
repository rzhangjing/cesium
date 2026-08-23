//! Ported from `packages/engine/Source/DataSources/Cesium3DTilesetGraphics.js`.

/// Graphics properties for a 3D Tiles tileset.
#[derive(Clone)]
pub struct Cesium3DTilesetGraphics {
    /// Whether this tileset is shown.
    pub show: bool,
    /// The URI of the 3D Tiles tileset JSON.
    pub uri: Option<String>,
    /// The maximum screen space error in pixels.
    pub maximum_screen_space_error: f64,
}

impl Cesium3DTilesetGraphics {
    /// Creates a new 3D Tiles graphics.
    pub fn new() -> Self {
        Self {
            show: true,
            uri: None,
            maximum_screen_space_error: 16.0,
        }
    }
}

impl Default for Cesium3DTilesetGraphics {
    fn default() -> Self { Self::new() }
}
