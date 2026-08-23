//! Ported from `packages/engine/Source/Scene/I3SSymbology.js`.

/// I3S symbology.
pub struct I3SSymbology {
    _private: (),
}

impl I3SSymbology {
    /// Creates a new I3SSymbology.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SSymbology {
    fn default() -> Self { Self::new() }
}
