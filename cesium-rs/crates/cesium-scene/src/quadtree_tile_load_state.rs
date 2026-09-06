//! Ported from `packages/engine/Source/Scene/QuadtreeTileLoadState.js`.
//!
//! The loading state of a quadtree tile.

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

impl QuadtreeTileLoadState {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Start),
            1 => Some(Self::Loading),
            2 => Some(Self::Done),
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
            Self::Done => "DONE",
            Self::Failed => "FAILED",
        }
    }

    /// Returns whether the tile is ready for rendering.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Done)
    }
}

impl Default for QuadtreeTileLoadState {
    fn default() -> Self {
        Self::Start
    }
}
