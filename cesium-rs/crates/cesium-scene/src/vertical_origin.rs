//! Ported from `packages/engine/Source/Scene/VerticalOrigin.js`.
//!
//! The vertical origin of a billboard or label.

/// The vertical origin of a billboard or label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum VerticalOrigin {
    /// Center of the billboard/label.
    Center = 0,
    /// Bottom edge.
    Bottom = 1,
    /// Baseline (for text labels).
    Baseline = 2,
    /// Top edge.
    Top = -1,
}

impl VerticalOrigin {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Center),
            1 => Some(Self::Bottom),
            2 => Some(Self::Baseline),
            -1 => Some(Self::Top),
            _ => None,
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

impl Default for VerticalOrigin {
    fn default() -> Self {
        Self::Center
    }
}
