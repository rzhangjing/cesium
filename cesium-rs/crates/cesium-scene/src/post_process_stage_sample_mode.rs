//! Ported from `packages/engine/Source/Scene/PostProcessStageSampleMode.js`.
//!
//! The sampling mode for post-process stages.

/// The sampling mode for post-process stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PostProcessStageSampleMode {
    /// Nearest neighbor sampling.
    Nearest = 0,
    /// Linear filtering.
    Linear = 1,
}

impl PostProcessStageSampleMode {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Nearest),
            1 => Some(Self::Linear),
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
            Self::Nearest => "NEAREST",
            Self::Linear => "LINEAR",
        }
    }
}

impl Default for PostProcessStageSampleMode {
    fn default() -> Self {
        Self::Nearest
    }
}
