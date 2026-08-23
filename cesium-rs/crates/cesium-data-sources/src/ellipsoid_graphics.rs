//! Ported from `packages/engine/Source/DataSources/EllipsoidGraphics.js`.

/// Graphics properties for an ellipsoid.
#[derive(Clone)]
pub struct EllipsoidGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl EllipsoidGraphics {
    /// Creates a new Ellipsoid graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for EllipsoidGraphics {
    fn default() -> Self { Self::new() }
}
