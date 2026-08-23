//! Ported from `packages/engine/Source/Scene/PerformanceDisplay.js`.

/// Displays performance statistics (FPS, etc.).
pub struct PerformanceDisplay {
    /// Whether the display is visible.
    pub show: bool,
}

impl PerformanceDisplay {
    /// Creates a new performance display.
    pub fn new() -> Self {
        Self { show: false }
    }
}

impl Default for PerformanceDisplay {
    fn default() -> Self { Self::new() }
}
