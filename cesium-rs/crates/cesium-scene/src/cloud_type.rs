//! Ported from `packages/engine/Source/Scene/CloudType.js`.

/// Specifies the type of the cloud that is added to a `CloudCollection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CloudType {
    /// Cumulus cloud.
    Cumulus = 0,
}

impl CloudType {
    /// Validates that the provided cloud type is a valid `CloudType`.
    pub fn validate(cloud_type: Self) -> bool {
        matches!(cloud_type, Self::Cumulus)
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Cumulus),
            _ => None,
        }
    }
}
