//! Ported from `packages/engine/Source/Scene/QuadtreeTileProvider.js`.
//!
//! Interface for providing tiles to a QuadtreePrimitive.

use crate::frame_state::FrameState;
use crate::quadtree_tile::QuadtreeTile;

/// Interface for providing tiles to a QuadtreePrimitive.
///
/// In CesiumJS, GlobeSurfaceTileProvider implements this interface.
/// The QuadtreePrimitive calls these methods during tile traversal.
pub trait QuadtreeTileProvider {
    /// Called at the beginning of each frame.
    fn begin_frame(&mut self, frame_state: &FrameState);

    /// Called during tile traversal to determine if a tile should be refined.
    /// Returns the screen-space error for the given tile.
    fn compute_screen_space_error(&self, tile: &QuadtreeTile, frame_state: &FrameState) -> f64;

    /// Called to start loading a tile's data.
    fn load_tile(&mut self, tile: &mut QuadtreeTile, frame_state: &FrameState);

    /// Called to render a tile.
    fn render_tile(&self, tile: &QuadtreeTile, frame_state: &FrameState);

    /// Called to show a tile that was previously hidden.
    fn show_tile(&self, tile: &QuadtreeTile);

    /// Called to hide a tile that was previously shown.
    fn hide_tile(&self, tile: &QuadtreeTile);

    /// Called at the end of each frame.
    fn end_frame(&mut self, frame_state: &FrameState);
}
