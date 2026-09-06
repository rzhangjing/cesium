//! Ported from `packages/engine/Source/Scene/ShadowMode.js`.
//!
//! Whether or not an object casts or receives shadows.

/// Whether or not an object casts or receives shadows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShadowMode {
    /// No shadows.
    Disabled = 0,
    /// Casts and receives shadows.
    Enabled = 1,
    /// Only casts shadows.
    CastOnly = 2,
    /// Only receives shadows.
    ReceiveOnly = 3,
}

impl ShadowMode {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::Enabled),
            2 => Some(Self::CastOnly),
            3 => Some(Self::ReceiveOnly),
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
            Self::Disabled => "DISABLED",
            Self::Enabled => "ENABLED",
            Self::CastOnly => "CAST_ONLY",
            Self::ReceiveOnly => "RECEIVE_ONLY",
        }
    }

    /// Returns whether this mode casts shadows.
    pub fn cast_shadows(&self) -> bool {
        matches!(self, Self::Enabled | Self::CastOnly)
    }

    /// Returns whether this mode receives shadows.
    pub fn receive_shadows(&self) -> bool {
        matches!(self, Self::Enabled | Self::ReceiveOnly)
    }

    /// Creates from cast/receive booleans.
    pub fn from_cast_receive(cast: bool, receive: bool) -> Self {
        match (cast, receive) {
            (true, true) => Self::Enabled,
            (true, false) => Self::CastOnly,
            (false, true) => Self::ReceiveOnly,
            (false, false) => Self::Disabled,
        }
    }

    /// The number of shadow modes.
    pub const NUMBER_OF_SHADOW_MODES: u8 = 4;
}

impl Default for ShadowMode {
    fn default() -> Self {
        Self::Disabled
    }
}
