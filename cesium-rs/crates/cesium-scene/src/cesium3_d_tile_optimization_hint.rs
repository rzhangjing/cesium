//! Ported from `packages/engine/Source/Scene/Cesium3DTileOptimizationHint.js`.
//!
//! Whether an optimization should be applied.

/// Whether an optimization should be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum Cesium3DTileOptimizationHint {
    /// The optimization has not been computed yet.
    NotComputed = -1,
    /// Do not apply the optimization.
    SkipOptimization = 0,
    /// Apply the optimization.
    UseOptimization = 1,
}

impl Cesium3DTileOptimizationHint {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            -1 => Some(Self::NotComputed),
            0 => Some(Self::SkipOptimization),
            1 => Some(Self::UseOptimization),
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
            Self::NotComputed => "NOT_COMPUTED",
            Self::SkipOptimization => "SKIP_OPTIMIZATION",
            Self::UseOptimization => "USE_OPTIMIZATION",
        }
    }
}

impl Default for Cesium3DTileOptimizationHint {
    fn default() -> Self {
        Self::NotComputed
    }
}
