//! Ported from `packages/engine/Source/Core/writeTextToCanvas.js`.

/// Writes text to a canvas element.
pub struct WriteTextToCanvas {
    _private: (),
}

impl WriteTextToCanvas {
    /// Creates a new WriteTextToCanvas.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for WriteTextToCanvas {
    fn default() -> Self { Self::new() }
}
