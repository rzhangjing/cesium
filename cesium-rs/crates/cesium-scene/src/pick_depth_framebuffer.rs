//! Ported from `packages/engine/Source/Scene/PickDepthFramebuffer.js`.

/// Pick depth framebuffer.
///
/// Framebuffer for storing pick depth values.
pub struct PickDepthFramebuffer {
    /// Whether the framebuffer is allocated.
    pub allocated: bool,
}

impl PickDepthFramebuffer {
    /// Creates a new PickDepthFramebuffer.
    pub fn new() -> Self { Self { allocated: false } }
}

impl Default for PickDepthFramebuffer {
    fn default() -> Self { Self::new() }
}
