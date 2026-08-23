//! Ported from `packages/engine/Source/DataSources/createRawPropertyDescriptor.js`.

/// Creates a raw property descriptor that wraps a non-Property value.
pub fn create_raw_property_descriptor(_property_name: &str) -> Option<String> {
    // DEVIATION: Requires dynamic property descriptor creation
    None
}
