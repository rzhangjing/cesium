//! Ported from `packages/widgets/Source/PerformanceWatchdog/PerformanceWatchdog.js`.

/// The PerformanceWatchdog widget that provides a frame rate monitor and warning display.
pub struct PerformanceWatchdog {
    is_destroyed: bool,
}

impl PerformanceWatchdog {
    /// Creates a new PerformanceWatchdog widget.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this widget.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for PerformanceWatchdog {
    fn default() -> Self { Self::new() }
}
