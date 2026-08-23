//! Ported from `packages/engine/Source/Scene/Model/ModelRenderResources.js`.
//!
//! GPU resources for a model.

/// GPU rendering resources for a [`Model`](super::model::Model).
///
/// Manages GPU buffers, textures, and pipelines needed to render the model.
/// Mirrors CesiumJS `ModelRenderResources` (200 lines).
pub struct ModelRenderResources {
    /// The number of GPU buffers allocated.
    pub buffers_count: usize,
    /// The number of GPU textures allocated.
    pub textures_count: usize,
    /// The total memory used in bytes.
    pub memory_bytes: u64,
    /// Whether resources have been uploaded to the GPU.
    uploaded: bool,
}

impl ModelRenderResources {
    /// Creates a new ModelRenderResources.
    pub fn new() -> Self {
        Self {
            buffers_count: 0,
            textures_count: 0,
            memory_bytes: 0,
            uploaded: false,
        }
    }

    /// Returns whether resources have been uploaded to the GPU.
    pub fn is_uploaded(&self) -> bool {
        self.uploaded
    }

    /// Releases all GPU resources.
    pub fn release(&mut self) {
        self.buffers_count = 0;
        self.textures_count = 0;
        self.memory_bytes = 0;
        self.uploaded = false;
    }
}

impl Default for ModelRenderResources {
    fn default() -> Self { Self::new() }
}
