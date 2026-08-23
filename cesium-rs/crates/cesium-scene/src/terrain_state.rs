//! Ported from `packages/engine/Source/Scene/TerrainState.js`.

/// The loading state of terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TerrainState {
    /// Initial state.
    Start = 0,
    /// Loading.
    Loading = 1,
    /// Ready.
    Ready = 2,
    /// Failed.
    Failed = 3,
}
