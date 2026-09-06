//! Ported from `packages/engine/Source/Scene/NeverTileDiscardPolicy.js`.

use crate::tile_discard_policy::TileDiscardPolicy;

/// A [`TileDiscardPolicy`] specifying that tile images should never be
/// discarded.
///
/// Port of `NeverTileDiscardPolicy`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverTileDiscardPolicy;

impl NeverTileDiscardPolicy {
    /// Creates a new `NeverTileDiscardPolicy`.
    pub fn new() -> Self {
        Self
    }
}

impl TileDiscardPolicy for NeverTileDiscardPolicy {
    fn is_ready(&self) -> bool {
        true
    }

    fn should_discard_image(&self, _image: &[u8], _width: u32) -> bool {
        false
    }
}
