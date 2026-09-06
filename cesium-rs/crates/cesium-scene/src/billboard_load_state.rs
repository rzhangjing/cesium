//! Ported from `packages/engine/Source/Scene/BillboardLoadState.js`.
//!
//! The loading state of a billboard.

/// The loading state of a billboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BillboardLoadState {
    /// Not yet loaded.
    Unloaded = 0,
    /// Currently loading.
    Loading = 1,
    /// Loaded and ready to render.
    Ready = 2,
    /// Loading failed.
    Failed = 3,
}

impl BillboardLoadState {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Unloaded),
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
            Self::Unloaded => "UNLOADED",
            Self::Loading => "LOADING",
            Self::Ready => "READY",
            Self::Failed => "FAILED",
        }
    }

    /// Returns whether the billboard is ready to render.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

impl Default for BillboardLoadState {
    fn default() -> Self {
        Self::Unloaded
    }
}
