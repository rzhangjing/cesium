//! Ported from `packages/engine/Source/Renderer/ComputeCommand.js`.
//!
//! Represents a command to the renderer for GPU Compute (GPGPU).
//! In CesiumJS, this uses a fragment shader on a viewport quad for GPGPU.
//! In the Rust/wgpu port, this maps to wgpu compute shaders.

use crate::shader_program::ShaderProgram;

/// A command to run a compute shader.
///
/// Mirrors the CesiumJS `ComputeCommand` which uses a fragment shader on a
/// viewport quad for GPGPU. In the Rust/wgpu port, this maps to wgpu compute
/// shaders or render-to-texture passes.
pub struct ComputeCommand {
    /// The vertex array (if None, a viewport quad is used).
    pub vertex_array: Option<Box<dyn std::any::Any + Send + Sync>>,
    /// The fragment shader source (for GPGPU via render-to-texture).
    pub fragment_shader_source: Option<String>,
    /// The shader program to apply.
    pub shader_program: Option<ShaderProgram>,
    /// Uniform map function for setting uniform values.
    pub uniform_map: Option<Box<dyn Fn() + Send + Sync>>,
    /// Texture to use for offscreen rendering (output).
    pub output_texture: Option<Box<dyn std::any::Any + Send + Sync>>,
    /// Function called immediately before execution (for resource updates).
    pub pre_execute: Option<Box<dyn Fn(&mut ComputeCommand) + Send + Sync>>,
    /// Function called after execution (receives output texture).
    pub post_execute: Option<Box<dyn Fn() + Send + Sync>>,
    /// Function called when the command is canceled.
    pub canceled: Option<Box<dyn Fn() + Send + Sync>>,
    /// Whether renderer resources persist beyond this call.
    pub persists: bool,
    /// The pass when to render (always compute pass).
    pub pass: u32,
    /// The object who created this command (for debugging).
    pub owner: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl ComputeCommand {
    /// Creates a new compute command with default values.
    pub fn new() -> Self {
        Self {
            vertex_array: None,
            fragment_shader_source: None,
            shader_program: None,
            uniform_map: None,
            output_texture: None,
            pre_execute: None,
            post_execute: None,
            canceled: None,
            persists: false,
            pass: 0, // COMPUTE pass
            owner: None,
        }
    }

    /// Executes the compute command.
    ///
    /// DEVIATION: In CesiumJS, this calls `context.compute(this)`.
    /// In the Rust port, the actual execution is handled by the Context
    /// which translates this to wgpu compute operations.
    pub fn execute(&self, _context: &mut crate::context::Context) {
        // The actual execution is handled by Context
        // This method is a placeholder for the command interface
    }
}

impl Default for ComputeCommand {
    fn default() -> Self { Self::new() }
}
