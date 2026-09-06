//! Ported from `packages/engine/Source/Scene/TileDiscardPolicy.js`.
//!
//! A policy for discarding tile images according to some criteria. This
//! trait describes an interface and is not intended to be instantiated
//! directly.

/// A policy for discarding tile images.
///
/// Port of `TileDiscardPolicy`. Implementations:
/// - [`NeverTileDiscardPolicy`](crate::never_tile_discard_policy::NeverTileDiscardPolicy)
/// - [`DiscardEmptyTileImagePolicy`](crate::discard_empty_tile_image_policy::DiscardEmptyTileImagePolicy)
/// - [`DiscardMissingTileImagePolicy`](crate::discard_missing_tile_image_policy::DiscardMissingTileImagePolicy)
pub trait TileDiscardPolicy {
    /// Determines if the discard policy is ready to process images.
    fn is_ready(&self) -> bool;

    /// Given tile image pixel data (RGBA), decide whether to discard.
    ///
    /// The `image` slice contains raw RGBA bytes in row-major order with
    /// the given `width` (in pixels). The total length must be
    /// `width * height * 4`.
    fn should_discard_image(&self, image: &[u8], width: u32) -> bool;
}
