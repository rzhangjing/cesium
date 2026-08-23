//! Ported from `packages/engine/Source/Scene/I3SSublayer.js`.

/// An I3S sublayer.
pub struct I3SSublayer {
    _private: (),
}

impl I3SSublayer {
    /// Creates a new I3SSublayer.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SSublayer {
    fn default() -> Self { Self::new() }
}
