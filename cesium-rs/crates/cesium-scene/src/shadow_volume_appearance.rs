//! Ported from `packages/engine/Source/Scene/ShadowVolumeAppearance.js`.

/// Shadow volume appearance for extruded geometry.
pub struct ShadowVolumeAppearance {
    _private: (),
}

impl ShadowVolumeAppearance {
    /// Creates a new ShadowVolumeAppearance.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ShadowVolumeAppearance {
    fn default() -> Self { Self::new() }
}
