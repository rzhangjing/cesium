//! Ported from `packages/engine/Source/Scene/TimeDynamicImagery.js`.

/// Time-dynamic imagery.
///
/// Manages imagery layers that change over time.
pub struct TimeDynamicImagery {
    /// The current clock time.
    pub clock: Option<String>,
    /// Whether the imagery is ready.
    pub ready: bool,
}

impl TimeDynamicImagery {
    /// Creates a new TimeDynamicImagery.
    pub fn new() -> Self { Self { clock: None, ready: false } }
}

impl Default for TimeDynamicImagery {
    fn default() -> Self { Self::new() }
}
