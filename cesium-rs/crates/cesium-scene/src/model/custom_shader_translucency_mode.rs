//! Ported from `packages/engine/Source/Scene/Model/CustomShaderTranslucencyMode.js`.

/// An enum for controlling how `CustomShader` handles translucency
/// compared with the original primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CustomShaderTranslucencyMode {
    /// Inherit translucency settings from the primitive's material.
    /// If the primitive used a translucent material, the custom shader
    /// will also be considered translucent. If the primitive used an
    /// opaque material, the custom shader will be considered opaque.
    Inherit = 0,
    /// Force the primitive to render as opaque, ignoring any material settings.
    Opaque = 1,
    /// Force the primitive to render as translucent, ignoring any material settings.
    Translucent = 2,
}

impl CustomShaderTranslucencyMode {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inherit => "INHERIT",
            Self::Opaque => "OPAQUE",
            Self::Translucent => "TRANSLUCENT",
        }
    }

    /// Parses from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "INHERIT" => Some(Self::Inherit),
            "OPAQUE" => Some(Self::Opaque),
            "TRANSLUCENT" => Some(Self::Translucent),
            _ => None,
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Inherit),
            1 => Some(Self::Opaque),
            2 => Some(Self::Translucent),
            _ => None,
        }
    }
}
