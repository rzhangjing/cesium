//! Ported from `packages/engine/Source/Scene/DiscardEmptyTileImagePolicy.js`.

use crate::tile_discard_policy::TileDiscardPolicy;

/// A policy for discarding tile images that contain no data (and so
/// aren't actually images).
///
/// In CesiumJS the policy compares against a sentinel `EMPTY_IMAGE`
/// (a 1×1 transparent PNG loaded via a data-URI). In the Rust port the
/// check is adapted: an image is considered "empty" when **all RGBA
/// bytes are zero** (fully transparent black), which covers the same
/// 1×1 transparent pixel sentinel and any other all-transparent tile.
///
/// Port of `DiscardEmptyTileImagePolicy`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiscardEmptyTileImagePolicy;

impl DiscardEmptyTileImagePolicy {
    /// Creates a new `DiscardEmptyTileImagePolicy`.
    pub fn new() -> Self {
        Self
    }
}

impl TileDiscardPolicy for DiscardEmptyTileImagePolicy {
    fn is_ready(&self) -> bool {
        true
    }

    fn should_discard_image(&self, image: &[u8], _width: u32) -> bool {
        // Mirror CesiumJS `EMPTY_IMAGE === image` semantics: an empty
        // tile is one whose pixel data is entirely zero (transparent).
        image.iter().all(|&b| b == 0)
    }
}
