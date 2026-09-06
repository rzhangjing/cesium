//! Ported from `packages/engine/Source/Scene/SensorVolumePortionToDisplay.js`.
//!
//! Which portion of a sensor volume to display.

/// Which portion of a sensor volume to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SensorVolumePortionToDisplay {
    /// Display the complete sensor volume.
    Complete = 0,
    /// Display only above the ellipsoid horizon.
    AboveEllipsoidHorizonOnly = 1,
    /// Display only below the ellipsoid horizon.
    BelowEllipsoidHorizonOnly = 2,
}

impl SensorVolumePortionToDisplay {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Complete),
            1 => Some(Self::AboveEllipsoidHorizonOnly),
            2 => Some(Self::BelowEllipsoidHorizonOnly),
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
            Self::Complete => "COMPLETE",
            Self::AboveEllipsoidHorizonOnly => "ABOVE_ELLIPSOID_HORIZON",
            Self::BelowEllipsoidHorizonOnly => "BELOW_ELLIPSOID_HORIZON",
        }
    }

    /// Returns whether the given value is a valid enum variant.
    pub fn validate(value: i32) -> bool {
        Self::from_i32(value).is_some()
    }
}

impl Default for SensorVolumePortionToDisplay {
    fn default() -> Self {
        Self::Complete
    }
}
