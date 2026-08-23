//! Ported from `packages/engine/Source/Scene/Model/MetadataPipelineStage.js`.

/// Pipeline stage for metadata processing.
pub struct MetadataPipelineStage {
    _private: (),
}

impl MetadataPipelineStage {
    /// Creates a new MetadataPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataPipelineStage {
    fn default() -> Self { Self::new() }
}
