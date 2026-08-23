//! Ported from `packages/engine/Source/Scene/getMetadataClassProperty.js`.

/// Gets metadata class property.
pub struct GetMetadataClassProperty {
    _private: (),
}

impl GetMetadataClassProperty {
    /// Creates a new GetMetadataClassProperty.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetMetadataClassProperty {
    fn default() -> Self { Self::new() }
}
