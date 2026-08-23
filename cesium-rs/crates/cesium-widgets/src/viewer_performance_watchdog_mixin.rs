//! Ported from `packages/widgets/Source/Viewer/viewerPerformanceWatchdogMixin.js`.
//!
//! A mixin that monitors rendering performance.

/// Trait for Viewer performance monitoring.
///
/// In CesiumJS, this mixin monitors frame rate and displays a warning
/// message if the frame rate drops below a threshold.
pub trait ViewerPerformanceWatchdogMixin {
    /// Returns the minimum frame rate threshold.
    fn minimum_frame_rate(&self) -> f64;

    /// Sets the minimum frame rate threshold.
    fn set_minimum_frame_rate(&mut self, fps: f64);

    /// Returns whether the performance warning is currently showing.
    fn is_warning_visible(&self) -> bool;

    /// Dismisses the performance warning.
    fn dismiss_warning(&mut self);
}

/// Configuration for the performance watchdog mixin.
pub struct PerformanceWatchdogOptions {
    /// The minimum frame rate before showing a warning.
    pub minimum_frame_rate: f64,
    /// The warning message to display.
    pub message: String,
}

impl Default for PerformanceWatchdogOptions {
    fn default() -> Self {
        Self {
            minimum_frame_rate: 5.0,
            message: String::from(
                "This application is performing poorly. Consider closing other applications \
                 or trying a different browser.",
            ),
        }
    }
}
