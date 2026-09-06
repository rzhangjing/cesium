//! Ported from `packages/engine/Source/Scene/CullFace.js`.
//!
//! Face culling mode. Values match WebGL `GL_FRONT` / `GL_BACK` constants.

/// Face culling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum CullFace {
    /// Front face (WebGL `GL_FRONT` = 0x0404).
    Front = 0x0404,
    /// Back face (WebGL `GL_BACK` = 0x0405).
    Back = 0x0405,
}

impl CullFace {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value as u32 {
            0x0404 => Some(Self::Front),
            0x0405 => Some(Self::Back),
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
            Self::Front => "FRONT",
            Self::Back => "BACK",
        }
    }
}

impl Default for CullFace {
    fn default() -> Self {
        Self::Back
    }
}
