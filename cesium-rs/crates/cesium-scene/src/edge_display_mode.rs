//! Ported from `packages/engine/Source/Scene/EdgeDisplayMode.js`.
//!
//! Edge display mode for classification primitives.

/// Edge display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EdgeDisplayMode {
    /// No edges.
    None = 0,
    /// Flat edges.
    Flat = 1,
    /// Phong edges.
    Phong = 2,
}

impl EdgeDisplayMode {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Flat),
            2 => Some(Self::Phong),
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
            Self::Flat => "FLAT",
            Self::Phong => "PHONG",
        }
    }
}

impl Default for EdgeDisplayMode {
    fn default() -> Self {
        Self::None
    }
}
