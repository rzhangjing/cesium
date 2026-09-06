//! Ported from `packages/engine/Source/Scene/BoundingVolumeSemantics.js`.
//!
//! Semantics for bounding volumes in 3D Tiles.

/// The semantics of a bounding volume in 3D Tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BoundingVolumeSemantics {
    /// The tile's bounding volume.
    BoundingVolume = 0,
    /// The content's bounding volume.
    ContentBoundingVolume = 1,
}

impl BoundingVolumeSemantics {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::BoundingVolume),
            1 => Some(Self::ContentBoundingVolume),
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
            Self::BoundingVolume => "BOUNDING_VOLUME",
            Self::ContentBoundingVolume => "CONTENT_BOUNDING_VOLUME",
        }
    }
}

impl Default for BoundingVolumeSemantics {
    fn default() -> Self {
        Self::BoundingVolume
    }
}
