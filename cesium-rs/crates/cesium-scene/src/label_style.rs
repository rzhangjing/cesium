//! Ported from `packages/engine/Source/Scene/LabelStyle.js`.
//!
//! The style of a label.

/// The style of a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LabelStyle {
    /// Fill only.
    Fill = 0,
    /// Outline only.
    Outline = 1,
    /// Fill and outline.
    FillAndOutline = 2,
}

impl LabelStyle {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Fill),
            1 => Some(Self::Outline),
            2 => Some(Self::FillAndOutline),
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
            Self::Fill => "FILL",
            Self::Outline => "OUTLINE",
            Self::FillAndOutline => "FILL_AND_OUTLINE",
        }
    }
}

impl Default for LabelStyle {
    fn default() -> Self {
        Self::Fill
    }
}
