//! Ported from `packages/engine/Source/Scene/AlphaMode.js`.
//!
//! The alpha blending mode for glTF materials and primitives.

/// The alpha blending mode for a primitive.
///
/// CesiumJS uses string values ("OPAQUE", "MASK", "BLEND") matching the
/// glTF 2.0 `alphaMode` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AlphaMode {
    /// Fully opaque — no alpha blending.
    Opaque = 0,
    /// Alpha tested — binary transparency using `alphaCutoff`.
    Mask = 1,
    /// Alpha blended — standard translucent blending.
    Blend = 2,
}

impl AlphaMode {
    /// Parses from the glTF `alphaMode` string.
    pub fn from_str(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "OPAQUE" => Some(Self::Opaque),
            "MASK" => Some(Self::Mask),
            "BLEND" => Some(Self::Blend),
            _ => None,
        }
    }

    /// Returns the glTF `alphaMode` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Opaque => "OPAQUE",
            Self::Mask => "MASK",
            Self::Blend => "BLEND",
        }
    }

    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Opaque),
            1 => Some(Self::Mask),
            2 => Some(Self::Blend),
            _ => None,
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Returns whether this mode requires an alpha cutoff value.
    pub fn needs_alpha_cutoff(&self) -> bool {
        matches!(self, Self::Mask)
    }

    /// Returns whether this mode involves any transparency.
    pub fn is_transparent(&self) -> bool {
        matches!(self, Self::Blend)
    }
}

impl Default for AlphaMode {
    fn default() -> Self {
        Self::Opaque
    }
}
