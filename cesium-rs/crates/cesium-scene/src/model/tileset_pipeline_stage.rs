//! Ported from `packages/engine/Source/Scene/Model/TilesetPipelineStage.js`.

/// Pipeline stage for tileset processing.
pub struct TilesetPipelineStage {
    _private: (),
}

impl TilesetPipelineStage {
    /// Creates a new TilesetPipelineStage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TilesetPipelineStage {
    fn default() -> Self { Self::new() }
}
