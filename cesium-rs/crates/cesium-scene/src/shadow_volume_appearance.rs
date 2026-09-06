//! Ported from `packages/engine/Source/Scene/ShadowVolumeAppearance.js`.

/// Shadow volume appearance.
///
/// Manages appearance data for shadow volume rendering.
pub struct ShadowVolumeAppearance {
    /// Whether the appearance is active.
    pub active: bool,
}

impl ShadowVolumeAppearance {
    /// Creates a new ShadowVolumeAppearance.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for ShadowVolumeAppearance {
    fn default() -> Self { Self::new() }
}
