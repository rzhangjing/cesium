//! Ported from `packages/engine/Source/Scene/I3SField.js`.

/// An I3S field.
///
/// Represents a data field within I3S feature attributes.
pub struct I3SField {
    /// The field name.
    pub name: String,
    /// The field type string.
    pub field_type: String,
}

impl I3SField {
    /// Creates a new I3SField.
    pub fn new() -> Self { Self { name: String::new(), field_type: String::new() } }
}

impl Default for I3SField {
    fn default() -> Self { Self::new() }
}
