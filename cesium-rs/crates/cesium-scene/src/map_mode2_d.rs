//! Ported from `packages/engine/Source/Scene/MapMode2D.js`.
//!
//! Defines how 2D mode handles wrapping around the international date line.

/// Defines how 2D mode handles wrapping around the international date line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MapMode2D {
    /// 2D mode does not wrap.
    Rotate = 0,
    /// 2D mode wraps around the date line.
    InfiniteScroll = 1,
}

impl MapMode2D {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Rotate),
            1 => Some(Self::InfiniteScroll),
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
            Self::Rotate => "ROTATE",
            Self::InfiniteScroll => "INFINITE_SCROLL",
        }
    }
}

impl Default for MapMode2D {
    fn default() -> Self {
        Self::Rotate
    }
}
