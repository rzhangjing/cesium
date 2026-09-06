//! Ported from `packages/engine/Source/Scene/AutoExposure.js`.

/// Auto exposure.
///
/// Automatically adjusts scene exposure based on luminance.
pub struct AutoExposure {
    /// Whether auto exposure is enabled.
    pub enabled: bool,
    /// The exposure compensation value.
    pub compensation: f32,
}

impl AutoExposure {
    /// Creates a new AutoExposure.
    pub fn new() -> Self { Self { enabled: false, compensation: 0.0 } }
}

impl Default for AutoExposure {
    fn default() -> Self { Self::new() }
}
