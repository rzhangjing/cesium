//! Ported from `packages/engine/Source/Scene/SDFSettings.js`.

/// Settings for signed distance field rendering.
pub struct SdfSettings {
    _private: (),
}

impl SdfSettings {
    /// Creates a new SdfSettings.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SdfSettings {
    fn default() -> Self { Self::new() }
}
