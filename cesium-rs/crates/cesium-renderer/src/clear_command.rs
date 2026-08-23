//! Ported from `packages/engine/Source/Renderer/ClearCommand.js`.
//!
//! Represents a command to the renderer for clearing a framebuffer.

use crate::render_state::RenderState;

/// A command to clear the framebuffer.
///
/// Mirrors the CesiumJS `ClearCommand` which specifies clear values for
/// color, depth, and stencil buffers, along with the target framebuffer.
pub struct ClearCommand {
    /// The value to clear the color buffer to (RGBA).
    /// When `None`, the color buffer is not cleared.
    pub color: Option<[f32; 4]>,
    /// The value to clear the depth buffer to.
    /// When `None`, the depth buffer is not cleared.
    pub depth: Option<f64>,
    /// The value to clear the stencil buffer to.
    /// When `None`, the stencil buffer is not cleared.
    pub stencil: Option<u32>,
    /// The render state to apply when executing the clear command.
    /// The following states affect clearing: scissor test, color mask,
    /// depth mask, and stencil mask.
    pub render_state: Option<RenderState>,
    /// The framebuffer to clear (None = default framebuffer).
    pub framebuffer: Option<Box<dyn std::any::Any + Send + Sync>>,
    /// The object who created this command (for debugging).
    pub owner: Option<Box<dyn std::any::Any + Send + Sync>>,
    /// The pass in which to run this command.
    pub pass: Option<u32>,
}

impl ClearCommand {
    /// Creates a new clear command with default values.
    pub fn new() -> Self {
        Self {
            color: None,
            depth: None,
            stencil: None,
            render_state: None,
            framebuffer: None,
            owner: None,
            pass: None,
        }
    }

    /// Creates a clear command that clears all buffers.
    ///
    /// Mirrors `ClearCommand.ALL` which clears color to (0,0,0,0),
    /// depth to 1.0, and stencil to 0.
    pub fn all() -> Self {
        Self {
            color: Some([0.0, 0.0, 0.0, 0.0]),
            depth: Some(1.0),
            stencil: Some(0),
            render_state: None,
            framebuffer: None,
            owner: None,
            pass: None,
        }
    }

    /// Creates a clear command with the specified color.
    pub fn with_color(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: Some([r, g, b, a]),
            ..Self::new()
        }
    }

    /// Creates a clear command with the specified depth value.
    pub fn with_depth(depth: f64) -> Self {
        Self {
            depth: Some(depth),
            ..Self::new()
        }
    }

    /// Returns whether this command clears the color buffer.
    pub fn clears_color(&self) -> bool {
        self.color.is_some()
    }

    /// Returns whether this command clears the depth buffer.
    pub fn clears_depth(&self) -> bool {
        self.depth.is_some()
    }

    /// Returns whether this command clears the stencil buffer.
    pub fn clears_stencil(&self) -> bool {
        self.stencil.is_some()
    }
}

impl Default for ClearCommand {
    fn default() -> Self { Self::new() }
}
