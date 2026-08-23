//! Ported from `packages/engine/Source/Scene/PropertyAttribute.js`.

/// A property attribute in structured metadata.
pub struct PropertyAttribute {
    _private: (),
}

impl PropertyAttribute {
    /// Creates a new PropertyAttribute.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PropertyAttribute {
    fn default() -> Self { Self::new() }
}
