//! Ported from `packages/engine/Source/DataSources/PathGraphics.js`.

/// Graphics properties for a path (trail behind a moving entity).
#[derive(Clone)]
pub struct PathGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl PathGraphics {
    /// Creates a new Path graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for PathGraphics {
    fn default() -> Self { Self::new() }
}
