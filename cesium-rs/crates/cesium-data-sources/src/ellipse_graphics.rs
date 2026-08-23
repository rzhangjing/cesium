//! Ported from `packages/engine/Source/DataSources/EllipseGraphics.js`.

/// Graphics properties for an ellipse or disk.
#[derive(Clone)]
pub struct EllipseGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl EllipseGraphics {
    /// Creates a new Ellipse graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for EllipseGraphics {
    fn default() -> Self { Self::new() }
}
