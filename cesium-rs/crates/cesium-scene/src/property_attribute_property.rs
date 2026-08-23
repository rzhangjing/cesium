//! Ported from `packages/engine/Source/Scene/PropertyAttributeProperty.js`.

/// A property within a property attribute.
pub struct PropertyAttributeProperty {
    _private: (),
}

impl PropertyAttributeProperty {
    /// Creates a new PropertyAttributeProperty.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PropertyAttributeProperty {
    fn default() -> Self { Self::new() }
}
