//! Ported from `packages/engine/Source/Scene/SceneFramebuffer.js`.

/// The framebuffer used for scene rendering.
pub struct SceneFramebuffer {
    /// The width of the framebuffer.
    pub width: u32,
    /// The height of the framebuffer.
    pub height: u32,
}

impl SceneFramebuffer {
    /// Creates a new scene framebuffer.
    pub fn new() -> Self {
        Self { width: 0, height: 0 }
    }

    /// Resizes the framebuffer.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

impl Default for SceneFramebuffer {
    fn default() -> Self { Self::new() }
}
