//! Ported from `packages/engine/Source/Core/TranslationRotationScale.js`.

use crate::cartesian3::Cartesian3;
use crate::quaternion::Quaternion;

/// An affine transformation defined by a translation, rotation, and scale.
#[derive(Debug, Clone, PartialEq)]
pub struct TranslationRotationScale {
    /// The (x, y, z) translation.
    pub translation: Cartesian3,
    /// The (x, y, z, w) rotation quaternion.
    pub rotation: Quaternion,
    /// The (x, y, z) scaling.
    pub scale: Cartesian3,
}

impl Default for TranslationRotationScale {
    fn default() -> Self {
        Self {
            translation: Cartesian3::ZERO,
            rotation: Quaternion::IDENTITY,
            scale: Cartesian3::new(1.0, 1.0, 1.0),
        }
    }
}

impl TranslationRotationScale {
    pub fn new(translation: Cartesian3, rotation: Quaternion, scale: Cartesian3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Compares this instance against the provided instance.
    pub fn equals(&self, right: &Self) -> bool {
        self == right
    }
}
