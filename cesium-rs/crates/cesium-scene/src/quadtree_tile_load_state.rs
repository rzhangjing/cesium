//! Ported from `packages/engine/Source/Scene/QuadtreeTileLoadState.js`.

/// The loading state of a quadtree tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QuadtreeTileLoadState {
    /// Initial state.
    Start = 0,
    /// Loading.
    Loading = 1,
    /// Done loading.
    Done = 2,
    /// Failed.
    Failed = 3,
}
