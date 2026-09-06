//! Ported from `packages/engine/Source/Scene/Cesium3DTilePointCloudColorBlendMode.js`.
//!
//! Color blend modes for 3D Tiles point cloud rendering.

/// Color blend mode for 3D Tiles point clouds.
///
/// Determines how point cloud colors are blended with the scene.
/// Mirrors CesiumJS point cloud color blend behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTilePointCloudColorBlendMode {
    /// Use the point's color directly.
    Replace = 0,
    /// Highlight the point color.
    Highlight = 1,
    /// Mix the point color with the highlight.
    Mix = 2,
}

impl Cesium3DTilePointCloudColorBlendMode {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Replace),
            1 => Some(Self::Highlight),
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
            Self::Replace => "REPLACE",
            Self::Highlight => "HIGHLIGHT",
            Self::Mix => "MIX",
        }
    }
}

impl Default for Cesium3DTilePointCloudColorBlendMode {
    fn default() -> Self {
        Self::Replace
    }
}
