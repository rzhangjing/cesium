//! Ported from `packages/engine/Source/Scene/Imagery.js`.
//!
//! An imagery tile corresponding to a single terrain tile.

use cesium_core::rectangle::Rectangle;

/// An imagery tile corresponding to a single terrain tile.
///
/// Each Imagery represents one imagery layer's contribution to one terrain tile.
/// It tracks the load state and holds the texture data once loaded.
pub struct Imagery {
    /// The imagery layer this tile belongs to.
    pub layer_index: usize,
    /// The x coordinate of the tile.
    pub x: i32,
    /// The y coordinate of the tile.
    pub y: i32,
    /// The level of the tile.
    pub level: i32,
    /// The rectangle covered by this imagery tile.
    pub rectangle: Rectangle,

    /// Whether this imagery tile has been loaded.
    pub loaded: bool,
    /// Whether this imagery tile failed to load.
    pub failed: bool,
    /// Whether this imagery tile is in the process of being loaded.
    pub loading: bool,

    /// The texture data (once loaded). In full port, this would be a GPU texture handle.
    pub texture_data: Option<Vec<u8>>,
    /// The texture width.
    pub texture_width: u32,
    /// The texture height.
    pub texture_height: u32,
}

impl Imagery {
    /// Creates a new Imagery tile.
    pub fn new(layer_index: usize, x: i32, y: i32, level: i32, rectangle: Rectangle) -> Self {
        Self {
            layer_index,
            x,
            y,
            level,
            rectangle,
            loaded: false,
            failed: false,
            loading: false,
            texture_data: None,
            texture_width: 0,
            texture_height: 0,
        }
    }

    /// Returns whether this imagery tile is ready (loaded and not failed).
    pub fn is_ready(&self) -> bool {
        self.loaded && !self.failed
    }

    /// Marks this imagery as loading.
    pub fn start_loading(&mut self) {
        self.loading = true;
    }

    /// Marks this imagery as loaded with the given texture data.
    pub fn set_loaded(&mut self, data: Vec<u8>, width: u32, height: u32) {
        self.texture_data = Some(data);
        self.texture_width = width;
        self.texture_height = height;
        self.loaded = true;
        self.loading = false;
    }

    /// Marks this imagery as failed.
    pub fn set_failed(&mut self) {
        self.failed = true;
        self.loading = false;
    }
}

impl Default for Imagery {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, Rectangle::default())
    }
}
