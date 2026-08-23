//! Ported from `packages/engine/Source/DataSources/getElement.js`.

/// Extracts an element from a property value or entity.
///
/// This is a utility function used internally by data source processing.
pub fn get_element(value: &str, _property_name: &str) -> Option<String> {
    // DEVIATION: Requires JSON/CZML parsing
    Some(value.to_string())
}
