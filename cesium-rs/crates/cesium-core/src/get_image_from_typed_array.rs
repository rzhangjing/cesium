//! Ported from `packages/engine/Source/Core/getImageFromTypedArray.js`.

/// Gets an image from a typed array.
pub struct GetImageFromTypedArray {
    _private: (),
}

impl GetImageFromTypedArray {
    /// Creates a new GetImageFromTypedArray.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetImageFromTypedArray {
    fn default() -> Self { Self::new() }
}
