//! Ported from `packages/engine/Source/Scene/BlendOption.js`.
//!
//! The blending option for a primitive.

/// The blending option for a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendOption {
    /// Blending is disabled.
    Disabled = 0,
    /// Standard alpha blending.
    AlphaBlend = 1,
    /// Premultiplied alpha blending.
    Premultiplied = 2,
    /// Additive blending.
    Additive = 3,
}

impl BlendOption {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::AlphaBlend),
            2 => Some(Self::Premultiplied),
            3 => Some(Self::Additive),
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
            Self::AlphaBlend => "ALPHA_BLEND",
            Self::Premultiplied => "PREMULTIPLIED",
            Self::Additive => "ADDITIVE",
        }
    }

    /// Returns whether this mode involves any transparency.
    pub fn is_transparent(&self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

impl Default for BlendOption {
    fn default() -> Self {
        Self::Disabled
    }
}
