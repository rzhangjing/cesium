//! Ported from `packages/engine/Source/Scene/SunPostProcess.js`.

/// Post-processing for the sun.
pub struct SunPostProcess {
    _private: (),
}

impl SunPostProcess {
    /// Creates a new SunPostProcess.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SunPostProcess {
    fn default() -> Self { Self::new() }
}
