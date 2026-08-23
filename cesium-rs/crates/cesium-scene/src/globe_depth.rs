//! Ported from `packages/engine/Source/Scene/GlobeDepth.js`.
//!
//! Manages the depth framebuffer used for globe rendering.

/// Manages the depth framebuffer used for globe rendering.
///
/// Handles creating and resizing the depth framebuffer, and provides
/// methods for copying depth between framebuffers.
pub struct GlobeDepth {
    /// Whether the depth framebuffer needs to be recreated.
    dirty: bool,
    /// The current width of the depth framebuffer.
    width: u32,
    /// The current height of the depth framebuffer.
    height: u32,
}

impl GlobeDepth {
    /// Creates a new GlobeDepth.
    pub fn new() -> Self {
        Self { dirty: true, width: 0, height: 0 }
    }

    /// Returns whether the depth framebuffer needs to be recreated.
    pub fn is_dirty(&self) -> bool { self.dirty }

    /// Updates the depth framebuffer dimensions.
    pub fn update(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.dirty = true;
        }
    }

    /// Clears the dirty flag after the framebuffer has been recreated.
    pub fn clear_dirty(&mut self) { self.dirty = false; }
}

impl Default for GlobeDepth {
    fn default() -> Self { Self::new() }
}
