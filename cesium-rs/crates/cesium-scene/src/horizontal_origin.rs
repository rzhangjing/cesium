//! Ported from `packages/engine/Source/Scene/HorizontalOrigin.js`.
//!
//! The horizontal origin of a billboard or label.

/// The horizontal origin of a billboard or label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum HorizontalOrigin {
    /// Center.
    Center = 0,
    /// Left.
    Left = 1,
    /// Right.
    Right = -1,
}

impl HorizontalOrigin {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Center),
            1 => Some(Self::Left),
            -1 => Some(Self::Right),
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
            Self::Center => "CENTER",
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
        }
    }
}

impl Default for HorizontalOrigin {
    fn default() -> Self {
        Self::Center
    }
}
