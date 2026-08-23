//! Ported from `packages/engine/Source/Scene/I3SField.js`.

/// An I3S field.
pub struct I3SField {
    _private: (),
}

impl I3SField {
    /// Creates a new I3SField.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SField {
    fn default() -> Self { Self::new() }
}
