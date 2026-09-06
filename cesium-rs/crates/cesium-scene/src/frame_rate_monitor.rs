//! Ported from `packages/engine/Source/Scene/FrameRateMonitor.js`.

/// Frame rate monitor.
///
/// Monitors rendering frame rate and raises events for performance thresholds.
pub struct FrameRateMonitor {
    /// The target frame rate.
    pub target_frame_rate: u32,
    /// The measured frame rate.
    pub measured_frame_rate: f64,
    /// Whether monitoring is active.
    pub enabled: bool,
}

impl FrameRateMonitor {
    /// Creates a new FrameRateMonitor.
    pub fn new() -> Self { Self { target_frame_rate: 60, measured_frame_rate: 0.0, enabled: true } }
}

impl Default for FrameRateMonitor {
    fn default() -> Self { Self::new() }
}
