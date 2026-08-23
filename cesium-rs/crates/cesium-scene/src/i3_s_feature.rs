//! Ported from `packages/engine/Source/Scene/I3SFeature.js`.

/// An I3S feature.
pub struct I3SFeature {
    _private: (),
}

impl I3SFeature {
    /// Creates a new I3SFeature.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SFeature {
    fn default() -> Self { Self::new() }
}
