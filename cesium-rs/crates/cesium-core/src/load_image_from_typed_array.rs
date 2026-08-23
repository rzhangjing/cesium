//! Ported from `packages/engine/Source/Core/loadImageFromTypedArray.js`.

/// Loads an image from a typed array.
pub struct LoadImageFromTypedArray {
    _private: (),
}

impl LoadImageFromTypedArray {
    /// Creates a new LoadImageFromTypedArray.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for LoadImageFromTypedArray {
    fn default() -> Self { Self::new() }
}
