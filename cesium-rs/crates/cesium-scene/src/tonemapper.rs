//! Ported from `packages/engine/Source/Scene/Tonemapper.js`.

/// A tonemapping algorithm when rendering with high dynamic range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tonemapper {
    /// Use the Reinhard tonemapping.
    Reinhard,
    /// Use the modified Reinhard tonemapping.
    ModifiedReinhard,
    /// Use the Filmic tonemapping.
    Filmic,
    /// Use the ACES tonemapping.
    Aces,
    /// Use the PBR Neutral tonemapping.
    PbrNeutral,
}

impl Tonemapper {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reinhard => "REINHARD",
            Self::ModifiedReinhard => "MODIFIED_REINHARD",
            Self::Filmic => "FILMIC",
            Self::Aces => "ACES",
            Self::PbrNeutral => "PBR_NEUTRAL",
        }
    }

    /// Parses from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "REINHARD" => Some(Self::Reinhard),
            "MODIFIED_REINHARD" => Some(Self::ModifiedReinhard),
            "FILMIC" => Some(Self::Filmic),
            "ACES" => Some(Self::Aces),
            "PBR_NEUTRAL" => Some(Self::PbrNeutral),
            _ => None,
        }
    }

    /// Validate whether the provided value is a known Tonemapper type.
    pub fn is_valid(s: &str) -> bool {
        Self::from_str(s).is_some()
    }
}
