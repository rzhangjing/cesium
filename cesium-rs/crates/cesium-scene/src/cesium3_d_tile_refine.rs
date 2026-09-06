//! Ported from `packages/engine/Source/Scene/Cesium3DTileRefine.js`.
//!
//! 3D tile refinement strategy.

/// 3D tile refinement strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTileRefine {
    /// Add children.
    Add = 0,
    /// Replace with children.
    Replace = 1,
}

impl Cesium3DTileRefine {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Add),
            1 => Some(Self::Replace),
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
            Self::Add => "ADD",
            Self::Replace => "REPLACE",
        }
    }
}

impl Default for Cesium3DTileRefine {
    fn default() -> Self {
        Self::Replace
    }
}
