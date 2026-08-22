//! Ported from packages/engine/Source/Core/getImagePixels.js
//!
//! DEVIATION: CesiumJS draws the image into a reused 2D canvas and reads the
//! pixels back (`getImageData`). Native builds decode images ahead of time
//! (via the `image` crate at the IO boundary), so the port receives already
//! decoded RGBA pixels and returns them, keeping the width/height defaulting
//! semantics of the original signature. See docs/deviations.md.

/// Extract a pixel array from a loaded image.
///
/// Port of CesiumJS `getImagePixels(image, width, height)`: `width` /
/// `height` default to the image dimensions when not given.
#[must_use]
pub fn get_image_pixels(
    rgba: &[u8],
    image_width: u32,
    image_height: u32,
    width: Option<u32>,
    height: Option<u32>,
) -> Vec<u8> {
    let width = width.unwrap_or(image_width);
    let height = height.unwrap_or(image_height);

    if width == image_width && height == image_height {
        return rgba.to_vec();
    }

    // Sub-rectangle read (JS draws into a width×height canvas; drawing
    // scales the image into that canvas — nearest-source-row sampling for
    // the native port).
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for y in 0..height.min(image_height) {
        let src_row = (y * image_width * 4) as usize;
        let dst_row = (y * width * 4) as usize;
        let row_bytes = (width.min(image_width) * 4) as usize;
        pixels[dst_row..dst_row + row_bytes]
            .copy_from_slice(&rgba[src_row..src_row + row_bytes]);
    }
    pixels
}
