//! Ported from `packages/engine/Source/DataSources/WallGraphics.js`.

/// Graphics properties for a wall.
#[derive(Clone)]
pub struct WallGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl WallGraphics {
    /// Creates a new Wall graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for WallGraphics {
    fn default() -> Self { Self::new() }
}
