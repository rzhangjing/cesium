//! Ported from `packages/engine/Source/Scene/PickFramebuffer.js`.

/// Framebuffer for pick rendering.
pub struct PickFramebuffer {
    _private: (),
}

impl PickFramebuffer {
    /// Creates a new PickFramebuffer.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PickFramebuffer {
    fn default() -> Self { Self::new() }
}
