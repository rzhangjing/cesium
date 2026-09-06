//! Ported from `packages/engine/Source/Scene/JobType.js`.
//!
//! Type of job for the job scheduler.

/// Type of job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum JobType {
    /// Default.
    Default = 0,
    /// Terrain.
    Terrain = 1,
    /// Imagery.
    Imagery = 2,
}

impl JobType {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Default),
            1 => Some(Self::Terrain),
            2 => Some(Self::Imagery),
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
            Self::Default => "DEFAULT",
            Self::Terrain => "TERRAIN",
            Self::Imagery => "IMAGERY",
        }
    }

    /// The number of job types.
    pub const NUMBER_OF_JOB_TYPES: u8 = 3;
}

impl Default for JobType {
    fn default() -> Self {
        Self::Default
    }
}
