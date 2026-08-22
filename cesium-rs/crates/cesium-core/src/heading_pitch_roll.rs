//! Ported from packages/engine/Source/Core/HeadingPitchRoll.js
//!
//! A rotation expressed as a heading, pitch, and roll.

use crate::math::CesiumMath;
use crate::quaternion::Quaternion;

/// A rotation expressed as a heading, pitch, and roll.
///
/// Heading is the rotation about the negative z axis.
/// Pitch is the rotation about the negative y axis.
/// Roll is the rotation about the positive x axis.
#[derive(Clone, Copy, Debug, Default)]
pub struct HeadingPitchRoll {
    /// The heading component in radians.
    pub heading: f64,
    /// The pitch component in radians.
    pub pitch: f64,
    /// The roll component in radians.
    pub roll: f64,
}

impl HeadingPitchRoll {
    pub fn new(heading: f64, pitch: f64, roll: f64) -> Self {
        Self { heading, pitch, roll }
    }

    /// Port of `HeadingPitchRoll.fromQuaternion`.
    pub fn from_quaternion(quaternion: &Quaternion, result: &mut Self) {
        let test = 2.0 * (quaternion.w * quaternion.y - quaternion.z * quaternion.x);
        let denominator_roll = 1.0 - 2.0 * (quaternion.x * quaternion.x + quaternion.y * quaternion.y);
        let numerator_roll = 2.0 * (quaternion.w * quaternion.x + quaternion.y * quaternion.z);
        let denominator_heading =
            1.0 - 2.0 * (quaternion.y * quaternion.y + quaternion.z * quaternion.z);
        let numerator_heading =
            2.0 * (quaternion.w * quaternion.z + quaternion.x * quaternion.y);
        result.heading = -numerator_heading.atan2(denominator_heading);
        result.roll = numerator_roll.atan2(denominator_roll);
        result.pitch = -CesiumMath::asin_clamped(test);
    }

    pub fn from_quaternion_new(quaternion: &Quaternion) -> Self {
        let mut result = Self::default();
        Self::from_quaternion(quaternion, &mut result);
        result
    }
}

impl PartialEq for HeadingPitchRoll {
    fn eq(&self, other: &Self) -> bool {
        self.heading == other.heading && self.pitch == other.pitch && self.roll == other.roll
    }
}
