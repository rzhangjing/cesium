//! Ported from `packages/engine/Source/Scene/HeightReference.js`.
//!
//! Height reference mode for billboards, labels, and points.

/// Height reference mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HeightReference {
    /// No height reference.
    None = 0,
    /// Clamp to ground.
    ClampToGround = 1,
    /// Relative to ground.
    RelativeToGround = 2,
}

impl HeightReference {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::ClampToGround),
            2 => Some(Self::RelativeToGround),
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
            Self::None => "NONE",
            Self::ClampToGround => "CLAMP_TO_GROUND",
            Self::RelativeToGround => "RELATIVE_TO_GROUND",
        }
    }

    /// Returns whether this mode clamps to a surface.
    pub fn is_clamp(&self) -> bool {
        matches!(self, Self::ClampToGround)
    }

    /// Returns whether this mode is relative to a surface.
    pub fn is_relative(&self) -> bool {
        matches!(self, Self::RelativeToGround)
    }
}

impl Default for HeightReference {
    fn default() -> Self {
        Self::None
    }
}
