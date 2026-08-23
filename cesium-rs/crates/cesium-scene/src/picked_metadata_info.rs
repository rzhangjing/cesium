//! Ported from `packages/engine/Source/Scene/PickedMetadataInfo.js`.

/// Metadata info from a pick operation.
pub struct PickedMetadataInfo {
    _private: (),
}

impl PickedMetadataInfo {
    /// Creates a new PickedMetadataInfo.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PickedMetadataInfo {
    fn default() -> Self { Self::new() }
}
