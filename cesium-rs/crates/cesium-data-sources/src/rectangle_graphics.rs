//! Ported from `packages/engine/Source/DataSources/RectangleGraphics.js`.

/// Graphics properties for a rectangle.
#[derive(Clone)]
pub struct RectangleGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl RectangleGraphics {
    /// Creates a new Rectangle graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for RectangleGraphics {
    fn default() -> Self { Self::new() }
}
