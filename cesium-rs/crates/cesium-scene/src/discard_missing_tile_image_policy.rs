//! Ported from `packages/engine/Source/Scene/DiscardMissingTileImagePolicy.js`.

use cesium_core::cartesian2::Cartesian2;

use crate::tile_discard_policy::TileDiscardPolicy;

/// Options for creating a [`DiscardMissingTileImagePolicy`].
pub struct DiscardMissingTileImagePolicyOptions {
    /// An array of pixel positions to compare against the missing image.
    pub pixels_to_check: Vec<Cartesian2>,
    /// If `true`, the discard check will be disabled when all checked
    /// pixels in the missing image have an alpha value of 0.
    pub disable_check_if_all_pixels_are_transparent: Option<bool>,
}

/// A policy for discarding tile images that match a known image
/// containing a "missing" image.
///
/// In CesiumJS the constructor fetches the missing image asynchronously
/// via `Resource.fetchImage`. In the Rust port the reference pixel data
/// must be supplied after construction via
/// [`set_missing_image_pixels`](Self::set_missing_image_pixels), which
/// decouples the policy from the async resource backend.
///
/// Port of `DiscardMissingTileImagePolicy`.
pub struct DiscardMissingTileImagePolicy {
    pixels_to_check: Vec<Cartesian2>,
    disable_check_if_all_transparent: bool,
    missing_image_pixels: Option<Vec<u8>>,
    missing_image_byte_length: Option<usize>,
    is_ready: bool,
}

impl DiscardMissingTileImagePolicy {
    /// Creates a new `DiscardMissingTileImagePolicy`.
    ///
    /// The policy starts in a "loading" state (`is_ready() == false`).
    /// Call [`set_missing_image_pixels`](Self::set_missing_image_pixels)
    /// once the reference image data is available to transition to ready.
    pub fn new(options: DiscardMissingTileImagePolicyOptions) -> Self {
        Self {
            pixels_to_check: options.pixels_to_check,
            disable_check_if_all_transparent: options
                .disable_check_if_all_pixels_are_transparent
                .unwrap_or(false),
            missing_image_pixels: None,
            missing_image_byte_length: None,
            is_ready: false,
        }
    }

    /// Supply the reference "missing" image pixel data.
    ///
    /// `pixels` is raw RGBA data in row-major order. `byte_length` is
    /// the original encoded size (used for a fast-path blob-size
    /// comparison, matching CesiumJS `image.blob.size`).
    ///
    /// After this call `is_ready()` returns `true`.
    pub fn set_missing_image_pixels(&mut self, pixels: Vec<u8>, byte_length: Option<usize>) {
        self.missing_image_byte_length = byte_length;

        let checked_pixels = &self.pixels_to_check;
        let width = if self.disable_check_if_all_transparent {
            // We need the image width to check alpha values. The caller
            // is expected to pass a square tile; compute from length.
            let side = (pixels.len() / 4) as f64;
            side.sqrt().ceil() as u32
        } else {
            0
        };

        if self.disable_check_if_all_transparent {
            let all_transparent = checked_pixels.iter().all(|pos| {
                let index = (pos.x as u32) * 4 + (pos.y as u32) * width * 4;
                if (index + 3) < pixels.len() as u32 {
                    pixels[index as usize + 3] == 0
                } else {
                    true
                }
            });
            if all_transparent {
                self.missing_image_pixels = None;
                self.is_ready = true;
                return;
            }
        }

        self.missing_image_pixels = Some(pixels);
        self.is_ready = true;
    }

    /// Mark the policy as ready without reference data, effectively
    /// disabling the discard check (mirrors the JS `failure()` path).
    pub fn disable(&mut self) {
        self.missing_image_pixels = None;
        self.is_ready = true;
    }
}

impl TileDiscardPolicy for DiscardMissingTileImagePolicy {
    fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn should_discard_image(&self, image: &[u8], width: u32) -> bool {
        if !self.is_ready {
            debug_assert!(
                self.is_ready,
                "should_discard_image must not be called before the discard policy is ready."
            );
            return false;
        }

        // If missing_image_pixels is None the check has been disabled
        // (either all-transparent or fetch failure).
        let Some(missing_pixels) = &self.missing_image_pixels else {
            return false;
        };

        // Fast-path: byte-length mismatch means different images.
        if let Some(expected_len) = self.missing_image_byte_length {
            if image.len() != expected_len {
                return false;
            }
        }

        for pos in &self.pixels_to_check {
            let index = (pos.x as u32) * 4 + (pos.y as u32) * width * 4;
            for offset in 0..4usize {
                let pixel_idx = index as usize + offset;
                if pixel_idx < image.len() && pixel_idx < missing_pixels.len() {
                    if image[pixel_idx] != missing_pixels[pixel_idx] {
                        return false;
                    }
                }
            }
        }
        true
    }
}

impl Default for DiscardMissingTileImagePolicy {
    fn default() -> Self {
        Self {
            pixels_to_check: Vec::new(),
            disable_check_if_all_transparent: false,
            missing_image_pixels: None,
            missing_image_byte_length: None,
            is_ready: true,
        }
    }
}
