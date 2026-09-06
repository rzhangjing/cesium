//! Ported from `packages/engine/Source/Scene/Model/MetadataPipelineStage.js`.

/// Pipeline stage for metadata processing.
///
/// Processes EXT_structural_metadata extension data for rendering.
pub struct MetadataPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl MetadataPipelineStage {
    /// Creates a new MetadataPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for MetadataPipelineStage {
    fn default() -> Self { Self::new() }
}
