//! Ported from `packages/engine/Source/Core/resizeImageToNextPowerOfTwo.js`.
//!
//! Resizes an image to the next power of two. In Rust, image resizing is
//! handled by the windowing/rendering system, not the DOM.

use crate::math::CesiumMath;

/// Returns the dimensions (width, height) resized to the next power of two.
pub fn compute_resize_dimensions(width: u32, height: u32) -> (u32, u32) {
    (
        CesiumMath::next_power_of_two(width as f64) as u32,
        CesiumMath::next_power_of_two(height as f64) as u32,
    )
}
