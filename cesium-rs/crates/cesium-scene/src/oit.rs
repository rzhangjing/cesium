//! Ported from `packages/engine/Source/Scene/OIT.js`.
//!
//! Order-independent transparency rendering.

use crate::frame_state::FrameState;

/// Order-independent transparency (OIT) rendering.
///
/// Uses a multi-pass approach to correctly render transparent objects
/// regardless of draw order.
/// Mirrors CesiumJS `OIT` (564 lines).
pub struct Oit {
    /// Whether OIT is enabled.
    pub enabled: bool,
    /// The number of accumulation buffers.
    number_of_accumulation_buffers: i32,
    /// The width of the OIT buffers.
    width: u32,
    /// The height of the OIT buffers.
    height: u32,
    /// Whether this has been destroyed.
    is_destroyed: bool,
}

impl Oit {
    /// Creates a new OIT.
    pub fn new() -> Self {
        Self {
            enabled: false,
            number_of_accumulation_buffers: 2,
            width: 0,
            height: 0,
            is_destroyed: false,
        }
    }

    /// Updates the OIT buffers for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires multiple render targets and compositing
    }

    /// Resizes the OIT buffers.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Returns whether this has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this OIT.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Oit {
    fn default() -> Self { Self::new() }
}
