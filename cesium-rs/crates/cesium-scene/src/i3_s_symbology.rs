//! Ported from `packages/engine/Source/Scene/I3SSymbology.js`.

/// I3S symbology.
///
/// Defines rendering symbology rules for I3S features.
pub struct I3SSymbology {
    /// Whether symbology rules are loaded.
    pub loaded: bool,
}

impl I3SSymbology {
    /// Creates a new I3SSymbology.
    pub fn new() -> Self { Self { loaded: false } }
}

impl Default for I3SSymbology {
    fn default() -> Self { Self::new() }
}
