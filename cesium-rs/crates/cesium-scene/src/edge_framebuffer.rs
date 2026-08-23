//! Ported from `packages/engine/Source/Scene/EdgeFramebuffer.js`.

/// Edge framebuffer for edge detection.
///
/// DEVIATION: stub implementation.
pub struct EdgeFramebuffer {
    _private: (),
}

impl EdgeFramebuffer {
    /// Creates a new edge framebuffer.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for EdgeFramebuffer {
    fn default() -> Self { Self::new() }
}
