//! Ported from `packages/engine/Source/Scene/TerrainState.js`.
//!
//! The loading state of terrain.

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

impl TerrainState {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Start),
            1 => Some(Self::Loading),
            2 => Some(Self::Ready),
            3 => Some(Self::Failed),
            _ => None,
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Returns the CesiumJS string name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "START",
            Self::Loading => "LOADING",
            Self::Ready => "READY",
            Self::Failed => "FAILED",
        }
    }

    /// Returns whether the terrain is ready.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Returns whether the terrain has failed to load.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

impl Default for TerrainState {
    fn default() -> Self {
        Self::Start
    }
}
