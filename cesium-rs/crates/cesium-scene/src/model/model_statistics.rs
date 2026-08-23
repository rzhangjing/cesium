//! Ported from `packages/engine/Source/Scene/ModelStatistics.js`.
//!
//! Statistics about a model's resources.

/// Statistics about a [`Model`](crate::model::model::Model)'s resources.
///
/// Tracks mesh, material, animation, and memory usage metrics.
/// Mirrors CesiumJS `ModelStatistics` (100 lines).
pub struct ModelStatistics {
    /// The number of meshes in the model.
    pub meshes_length: usize,
    /// The number of materials in the model.
    pub materials_length: usize,
    /// The number of animations in the model.
    pub animations_length: usize,
    /// The number of nodes in the model.
    pub nodes_length: usize,
    /// The number of textures in the model.
    pub textures_length: usize,
    /// The total memory usage in bytes.
    pub memory_usage_in_bytes: u64,
    /// The number of draw commands generated.
    pub draw_commands_count: usize,
}

impl ModelStatistics {
    /// Creates a new ModelStatistics with zero values.
    pub fn new() -> Self {
        Self {
            meshes_length: 0,
            materials_length: 0,
            animations_length: 0,
            nodes_length: 0,
            textures_length: 0,
            memory_usage_in_bytes: 0,
            draw_commands_count: 0,
        }
    }

    /// Resets all statistics to zero.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for ModelStatistics {
    fn default() -> Self { Self::new() }
}
