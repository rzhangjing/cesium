//! Ported from `packages/engine/Source/DataSources/CorridorGraphics.js`.

/// Graphics properties for a corridor (polyline with width).
#[derive(Clone)]
pub struct CorridorGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl CorridorGraphics {
    /// Creates a new Corridor graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for CorridorGraphics {
    fn default() -> Self { Self::new() }
}
