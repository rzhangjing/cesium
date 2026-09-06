//! Ported from `packages/engine/Source/Scene/ClassificationType.js`.
//!
//! The type of geometry that a primitive classifies.

/// Type of classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ClassificationType {
    /// Classifies 3D Tiles geometry.
    Cesium3DTiles = 0,
    /// Classifies terrain.
    Terrain = 1,
    /// Classifies both 3D Tiles and terrain.
    Both = 2,
}

impl ClassificationType {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Cesium3DTiles),
            1 => Some(Self::Terrain),
            2 => Some(Self::Both),
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
            Self::Cesium3DTiles => "CESIUM_3D_TILE",
            Self::Terrain => "TERRAIN",
            Self::Both => "BOTH",
        }
    }

    /// The number of classification types.
    pub const NUMBER_OF_CLASSIFICATION_TYPES: u8 = 3;
}

impl Default for ClassificationType {
    fn default() -> Self {
        Self::Both
    }
}
