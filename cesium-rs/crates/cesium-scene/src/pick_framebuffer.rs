//! Ported from `packages/engine/Source/Scene/PickFramebuffer.js`.

/// Pick framebuffer.
///
/// Framebuffer for GPU-based object picking.
pub struct PickFramebuffer {
    /// Whether the framebuffer is allocated.
    pub allocated: bool,
}

impl PickFramebuffer {
    /// Creates a new PickFramebuffer.
    pub fn new() -> Self { Self { allocated: false } }
}

impl Default for PickFramebuffer {
    fn default() -> Self { Self::new() }
}
