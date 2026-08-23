//! Ported from `packages/engine/Source/Core/ITwinPlatform.js`.

/// iTwin platform integration.
pub struct ITwinPlatform {
    _private: (),
}

impl ITwinPlatform {
    /// Creates a new ITwinPlatform.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ITwinPlatform {
    fn default() -> Self { Self::new() }
}
