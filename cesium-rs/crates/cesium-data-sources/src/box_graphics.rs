//! Ported from `packages/engine/Source/DataSources/BoxGraphics.js`.

/// Graphics properties for a box (rectangular parallelepiped).
#[derive(Clone)]
pub struct BoxGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl BoxGraphics {
    /// Creates a new Box graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for BoxGraphics {
    fn default() -> Self { Self::new() }
}
