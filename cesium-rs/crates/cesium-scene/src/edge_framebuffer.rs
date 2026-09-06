//! Ported from `packages/engine/Source/Scene/EdgeFramebuffer.js`.

/// Edge framebuffer.
///
/// Framebuffer for edge detection rendering.
pub struct EdgeFramebuffer {
    /// Whether the framebuffer is allocated.
    pub allocated: bool,
}

impl EdgeFramebuffer {
    /// Creates a new EdgeFramebuffer.
    pub fn new() -> Self { Self { allocated: false } }
}

impl Default for EdgeFramebuffer {
    fn default() -> Self { Self::new() }
}
