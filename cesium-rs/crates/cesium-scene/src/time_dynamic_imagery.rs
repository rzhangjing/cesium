//! Ported from `packages/engine/Source/Scene/TimeDynamicImagery.js`.

/// Time-dynamic imagery layers.
pub struct TimeDynamicImagery {
    _private: (),
}

impl TimeDynamicImagery {
    /// Creates a new TimeDynamicImagery.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TimeDynamicImagery {
    fn default() -> Self { Self::new() }
}
