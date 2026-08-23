//! Ported from `packages/engine/Source/DataSources/PlaneGraphics.js`.

/// Graphics properties for a plane.
#[derive(Clone)]
pub struct PlaneGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl PlaneGraphics {
    /// Creates a new Plane graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for PlaneGraphics {
    fn default() -> Self { Self::new() }
}
