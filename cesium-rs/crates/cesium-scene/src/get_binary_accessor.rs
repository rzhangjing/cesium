//! Ported from `packages/engine/Source/Scene/getBinaryAccessor.js`.

use serde_json::Value;

/// Lookup table: number of components per attribute type.
pub fn components_per_attribute(type_str: &str) -> Option<u32> {
    match type_str {
        "SCALAR" => Some(1),
        "VEC2" => Some(2),
        "VEC3" => Some(3),
        "VEC4" => Some(4),
        "MAT2" => Some(4),
        "MAT3" => Some(9),
        "MAT4" => Some(16),
        _ => None,
    }
}

/// Result of parsing a binary accessor.
pub struct BinaryAccessorResult {
    /// The number of components per attribute element.
    pub components_per_attribute: u32,
    /// The attribute type string (e.g. "VEC3").
    pub attribute_type: String,
}

/// Parses a glTF accessor and returns the components-per-attribute count
/// and attribute type.
///
/// Mirrors `getBinaryAccessor(accessor)`.
pub fn get_binary_accessor(accessor: &Value) -> Option<BinaryAccessorResult> {
    let type_str = accessor.get("type")?.as_str()?;
    let components = components_per_attribute(type_str)?;

    Some(BinaryAccessorResult {
        components_per_attribute: components,
        attribute_type: type_str.to_string(),
    })
}
