//! Ported from `packages/engine/Source/Core/getImageFromTypedArray.js`.
//!
//! Constructs an image from a TypedArray of pixel values.
//! This is a browser/DOM-specific operation. In Rust, this is a skeleton.

/// Skeleton: constructs image dimensions from a pixel array.
/// Actual image creation requires a GPU/rendering backend.
pub struct ImageFromTypedArrayResult {
    pub width: u32,
    pub height: u32,
    pub pixel_data: Vec<u8>,
}

/// Creates an image result from a pixel byte array.
pub fn get_image_from_typed_array(
    typed_array: &[u8],
    width: u32,
    height: u32,
) -> ImageFromTypedArrayResult {
    ImageFromTypedArrayResult {
        width,
        height,
        pixel_data: typed_array.to_vec(),
    }
}
