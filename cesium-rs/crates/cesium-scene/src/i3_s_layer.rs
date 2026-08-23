//! Ported from `packages/engine/Source/Scene/I3SLayer.js`.

/// An I3S layer.
pub struct I3SLayer {
    _private: (),
}

impl I3SLayer {
    /// Creates a new I3SLayer.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SLayer {
    fn default() -> Self { Self::new() }
}
