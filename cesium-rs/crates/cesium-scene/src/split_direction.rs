//! Ported from `packages/engine/Source/Scene/SplitDirection.js`.
//!
//! The split direction for splitter rendering.

/// The split direction for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum SplitDirection {
    /// Render on the left side of the splitter.
    Left = -1,
    /// Render on both sides (no split).
    Both = 0,
    /// Render on the right side of the splitter.
    Right = 1,
}

impl SplitDirection {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            -1 => Some(Self::Left),
            0 => Some(Self::Both),
            1 => Some(Self::Right),
            _ => None,
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Returns the float value (CesiumJS uses f64).
    pub fn as_f64(&self) -> f64 {
        *self as i8 as f64
    }
}

impl Default for SplitDirection {
    fn default() -> Self {
        Self::Both
    }
}
