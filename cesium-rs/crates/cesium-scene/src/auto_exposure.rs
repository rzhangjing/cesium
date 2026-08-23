//! Ported from `packages/engine/Source/Scene/AutoExposure.js`.

/// Auto-exposure tone mapping settings.
pub struct AutoExposure {
    _private: (),
}

impl AutoExposure {
    /// Creates a new AutoExposure.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for AutoExposure {
    fn default() -> Self { Self::new() }
}
