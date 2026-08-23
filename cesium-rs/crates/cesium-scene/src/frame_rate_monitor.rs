//! Ported from `packages/engine/Source/Scene/FrameRateMonitor.js`.

/// Monitors frame rate.
pub struct FrameRateMonitor {
    _private: (),
}

impl FrameRateMonitor {
    /// Creates a new FrameRateMonitor.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for FrameRateMonitor {
    fn default() -> Self { Self::new() }
}
