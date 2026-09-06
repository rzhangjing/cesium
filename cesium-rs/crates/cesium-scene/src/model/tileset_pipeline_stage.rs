//! Ported from `packages/engine/Source/Scene/Model/TilesetPipelineStage.js`.

/// Pipeline stage for tileset processing.
///
/// Applies 3D Tiles tileset-level effects to model rendering.
pub struct TilesetPipelineStage {
    /// Number of commands processed by this stage.
    pub process_count: u64,
}

impl TilesetPipelineStage {
    /// Creates a new TilesetPipelineStage.
    pub fn new() -> Self { Self { process_count: 0 } }
}

impl Default for TilesetPipelineStage {
    fn default() -> Self { Self::new() }
}
