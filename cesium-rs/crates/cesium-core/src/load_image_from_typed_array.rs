//! Ported from `packages/engine/Source/Core/loadImageFromTypedArray.js`.
//!
//! Loads an image from a typed array. Browser/DOM-specific; skeleton in Rust.

/// Options for loading an image from a typed array.
pub struct LoadImageFromTypedArrayOptions {
    pub uint8_array: Vec<u8>,
    pub format: String,
    pub flip_y: bool,
    pub skip_color_space_conversion: bool,
}

/// Skeleton: loads an image from a typed array.
/// In Rust this requires a GPU/rendering backend.
pub fn load_image_from_typed_array(
    _options: LoadImageFromTypedArrayOptions,
) -> Result<Vec<u8>, String> {
    // Skeleton: actual image loading requires a rendering backend
    Err("load_image_from_typed_array requires a rendering backend".into())
}
