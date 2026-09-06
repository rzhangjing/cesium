//! Ported from `packages/engine/Source/Scene/PropertyAttributeProperty.js`.
//!
//! A property within a property attribute, mapping a vertex attribute
//! to a metadata property with optional value transforms.

use serde_json::Value;

/// A property within a property attribute.
///
/// Maps a single vertex attribute to a metadata property, with optional
/// offset/scale transforms and value conversion.
/// Mirrors CesiumJS `PropertyAttributeProperty` (~200 lines).
pub struct PropertyAttributeProperty {
    /// The vertex attribute semantic (e.g. `_FEATURE_ID_0`).
    pub attribute: String,
    /// Whether this property has offset/scale value transforms.
    pub has_value_transform: bool,
    /// Offset transform value.
    pub offset: Option<Value>,
    /// Scale transform value.
    pub scale: Option<Value>,
    /// The class property definition this attribute property conforms to.
    pub class_property: Option<Value>,
    /// Extra user-defined data.
    pub extras: Option<Value>,
    /// Extension data.
    pub extensions: Option<Value>,
}

impl PropertyAttributeProperty {
    /// Creates a new `PropertyAttributeProperty`.
    pub fn new(attribute: String) -> Self {
        Self {
            attribute,
            has_value_transform: false,
            offset: None,
            scale: None,
            class_property: None,
            extras: None,
            extensions: None,
        }
    }

    /// Returns whether this property has any value transforms.
    pub fn has_transforms(&self) -> bool {
        self.has_value_transform || self.offset.is_some() || self.scale.is_some()
    }
}

impl Default for PropertyAttributeProperty {
    fn default() -> Self { Self::new(String::new()) }
}
