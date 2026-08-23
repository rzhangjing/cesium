//! Ported from `packages/engine/Source/Scene/GlobeTranslucencyFramebuffer.js`.
//!
//! Framebuffer used for rendering the translucent globe.

/// Framebuffer used for rendering the translucent globe.
pub struct GlobeTranslucencyFramebuffer {
    /// Whether the framebuffer needs to be recreated.
    dirty: bool,
    /// The current width.
    width: u32,
    /// The current height.
    height: u32,
}

impl GlobeTranslucencyFramebuffer {
    /// Creates a new GlobeTranslucencyFramebuffer.
    pub fn new() -> Self {
        Self { dirty: true, width: 0, height: 0 }
    }

    /// Updates the framebuffer dimensions.
    pub fn update(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.dirty = true;
        }
    }

    /// Returns whether the framebuffer needs to be recreated.
    pub fn is_dirty(&self) -> bool { self.dirty }

    /// Clears the dirty flag.
    pub fn clear_dirty(&mut self) { self.dirty = false; }
}

impl Default for GlobeTranslucencyFramebuffer {
    fn default() -> Self { Self::new() }
}
