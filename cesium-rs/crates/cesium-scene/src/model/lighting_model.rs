//! Ported from `packages/engine/Source/Scene/Model/LightingModel.js`.
//!
//! The lighting model determines how a model's surface interacts with light.

/// The lighting model used by a model.
///
/// CesiumJS defines two primary lighting models:
/// - `UNLIT` — no lighting calculations; diffuse color is used directly.
/// - `PBR` — physically-based rendering (metallic-roughness or specular-glossiness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LightingModel {
    /// Unlit — no lighting calculations; diffuse color is used directly.
    Unlit = 0,
    /// Physically-based rendering (metallic-roughness + specular-glossiness + IBL).
    Pbr = 1,
}

impl LightingModel {
    /// Parses a lighting model from its CesiumJS string name.
    ///
    /// Accepts `"UNLIT"` and `"PBR"` (case-insensitive).
    pub fn from_str(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "UNLIT" => Some(Self::Unlit),
            "PBR" => Some(Self::Pbr),
            _ => None,
        }
    }

    /// Returns the CesiumJS string name of this lighting model.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unlit => "UNLIT",
            Self::Pbr => "PBR",
        }
    }

    /// Returns `true` if this is a PBR lighting model.
    pub fn is_pbr(&self) -> bool {
        matches!(self, Self::Pbr)
    }

    /// Converts from a numeric representation (i32).
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Unlit),
            1 => Some(Self::Pbr),
            _ => None,
        }
    }
}

impl Default for LightingModel {
    fn default() -> Self {
        Self::Pbr
    }
}
