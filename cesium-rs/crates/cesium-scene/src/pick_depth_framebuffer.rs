//! Ported from `packages/engine/Source/Scene/PickDepthFramebuffer.js`.

/// Framebuffer for pick depth rendering.
pub struct PickDepthFramebuffer {
    _private: (),
}

impl PickDepthFramebuffer {
    /// Creates a new PickDepthFramebuffer.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PickDepthFramebuffer {
    fn default() -> Self { Self::new() }
}
