//! Ported from `packages/engine/Source/Scene/Model/MetadataPickingPipelineStage.js`.

/// Pipeline stage for metadata picking.
///
/// Enables picking of EXT_structural_metadata features.
pub struct MetadataPickingPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl MetadataPickingPipelineStage {
    /// Creates a new MetadataPickingPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for MetadataPickingPipelineStage {
    fn default() -> Self { Self::new() }
}
