//! Ported from `packages/engine/Source/Core/Fullscreen.js`.

/// Fullscreen API support.
pub struct Fullscreen {
    _private: (),
}

impl Fullscreen {
    /// Creates a new Fullscreen.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Fullscreen {
    fn default() -> Self { Self::new() }
}
