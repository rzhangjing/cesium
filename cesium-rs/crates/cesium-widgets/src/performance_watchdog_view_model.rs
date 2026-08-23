//! Ported from `packages/widgets/Source/PerformanceWatchdog/PerformanceWatchdogViewModel.js`.

/// The view model for the PerformanceWatchdog widget.
///
/// Monitors frame rate and displays a warning if performance is poor.
pub struct PerformanceWatchdogViewModel {
    /// Whether the warning message is visible.
    pub low_frame_rate_message_visible: bool,
    /// The minimum frame rate threshold.
    pub minimum_frame_rate: f64,
}

impl PerformanceWatchdogViewModel {
    /// Creates a new performance watchdog view model.
    pub fn new() -> Self {
        Self {
            low_frame_rate_message_visible: false,
            minimum_frame_rate: 5.0,
        }
    }
}

impl Default for PerformanceWatchdogViewModel {
    fn default() -> Self { Self::new() }
}
