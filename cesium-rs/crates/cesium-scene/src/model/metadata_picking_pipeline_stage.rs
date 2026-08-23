//! Ported from `packages/engine/Source/Scene/Model/MetadataPickingPipelineStage.js`.

/// Pipeline stage for metadata picking.
pub struct MetadataPickingPipelineStage {
    _private: (),
}

impl MetadataPickingPipelineStage {
    /// Creates a new MetadataPickingPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataPickingPipelineStage {
    fn default() -> Self { Self::new() }
}
