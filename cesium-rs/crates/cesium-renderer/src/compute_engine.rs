//! Ported from `packages/engine/Source/Renderer/ComputeEngine.js`.
//!
//! Manages compute shader execution.

/// Manages compute shader execution.
///
/// DEVIATION: wgpu WebGL2 backend does not support compute shaders.
/// This is a placeholder for when compute is available via WebGPU.
pub struct ComputeEngine {
    is_destroyed: bool,
}

impl ComputeEngine {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Dispatches a compute command.
    pub fn dispatch(&mut self, _groups_x: u32, _groups_y: u32, _groups_z: u32) {
        // DEVIATION: wgpu WebGL2 has no compute support
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for ComputeEngine {
    fn default() -> Self { Self::new() }
}
