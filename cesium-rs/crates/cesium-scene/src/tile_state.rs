//! Ported from `packages/engine/Source/Scene/TileState.js`.

/// The loading state of a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TileState {
    /// Initial state.
    Start = 0,
    /// Content is loading.
    Loading = 1,
    /// Content is being processed.
    Processing = 2,
    /// Ready to render.
    Ready = 3,
    /// Temporary failure.
    FailedTemporary = 4,
    /// Permanent failure.
    FailedPermanent = 5,
}
