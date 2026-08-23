//! Ported from `packages/engine/Source/Scene/getMetadataProperty.js`.

/// Gets metadata property.
pub struct GetMetadataProperty {
    _private: (),
}

impl GetMetadataProperty {
    /// Creates a new GetMetadataProperty.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetMetadataProperty {
    fn default() -> Self { Self::new() }
}
