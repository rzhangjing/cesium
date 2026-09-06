//! Ported from `packages/engine/Source/Scene/Model/StyleCommandsNeeded.js`.

/// An enum describing what commands (opaque or translucent) are required
/// by a `Cesium3DTileStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StyleCommandsNeeded {
    /// All features are opaque.
    AllOpaque = 0,
    /// All features are translucent.
    AllTranslucent = 1,
    /// Some features are opaque and some are translucent.
    OpaqueAndTranslucent = 2,
}

impl StyleCommandsNeeded {
    /// Determines which style commands are needed based on the feature counts.
    ///
    /// - If `translucent_features_length` is 0, returns `AllOpaque`.
    /// - If `translucent_features_length` equals `features_length`, returns `AllTranslucent`.
    /// - Otherwise returns `OpaqueAndTranslucent`.
    pub fn from_feature_counts(features_length: usize, translucent_features_length: usize) -> Self {
        if translucent_features_length == 0 {
            Self::AllOpaque
        } else if translucent_features_length == features_length {
            Self::AllTranslucent
        } else {
            Self::OpaqueAndTranslucent
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::AllOpaque),
            1 => Some(Self::AllTranslucent),
            2 => Some(Self::OpaqueAndTranslucent),
            _ => None,
        }
    }
}
