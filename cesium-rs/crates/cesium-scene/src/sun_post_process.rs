//! Ported from `packages/engine/Source/Scene/SunPostProcess.js`.

/// Sun post-process.
///
/// Applies sun glow/lens flare post-processing effects.
pub struct SunPostProcess {
    /// Whether the effect is enabled.
    pub enabled: bool,
}

impl SunPostProcess {
    /// Creates a new SunPostProcess.
    pub fn new() -> Self { Self { enabled: true } }
}

impl Default for SunPostProcess {
    fn default() -> Self { Self::new() }
}
