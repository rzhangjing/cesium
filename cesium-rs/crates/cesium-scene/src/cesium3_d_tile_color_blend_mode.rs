//! Ported from `packages/engine/Source/Scene/Cesium3DTileColorBlendMode.js`.
//!
//! The color blend mode for 3D Tiles.

/// The color blend mode for 3D Tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTileColorBlendMode {
    /// Replace the color with the highlight color.
    Highlight = 0,
    /// Mix between the source color and highlight.
    Replace = 1,
    /// Mix between the source color and highlight based on distance.
    Mix = 2,
}

impl Cesium3DTileColorBlendMode {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Highlight),
            1 => Some(Self::Replace),
            2 => Some(Self::Mix),
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
            Self::Highlight => "HIGHLIGHT",
            Self::Replace => "REPLACE",
            Self::Mix => "MIX",
        }
    }
}

impl Default for Cesium3DTileColorBlendMode {
    fn default() -> Self {
        Self::Highlight
    }
}
