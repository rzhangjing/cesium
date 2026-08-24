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

    /// Compares the provided HeadingPitchRolls componentwise and returns
    /// `true` if they pass an absolute or relative tolerance test, `false`
    /// otherwise.
    ///
    /// Port of `HeadingPitchRoll.equalsEpsilon`. `None` mirrors JS
    /// `undefined` (the JS `left === right` identity short-circuit is
    /// subsumed by the componentwise comparison).
    pub fn equals_epsilon(
        left: Option<&Self>,
        right: Option<&Self>,
        relative_epsilon: Option<f64>,
        absolute_epsilon: Option<f64>,
    ) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => {
                CesiumMath::equals_epsilon(
                    left.heading,
                    right.heading,
                    relative_epsilon,
                    absolute_epsilon,
                ) && CesiumMath::equals_epsilon(
                    left.pitch,
                    right.pitch,
                    relative_epsilon,
                    absolute_epsilon,
                ) && CesiumMath::equals_epsilon(
                    left.roll,
                    right.roll,
                    relative_epsilon,
                    absolute_epsilon,
                )
            }
            (None, None) => true,
            _ => false,
        }
    }

    /// Compares this HeadingPitchRoll against the provided one
    /// componentwise within the given tolerances.
    ///
    /// Port of `HeadingPitchRoll.prototype.equalsEpsilon`.
    pub fn equals_epsilon_method(
        &self,
        right: &Self,
        relative_epsilon: Option<f64>,
        absolute_epsilon: Option<f64>,
    ) -> bool {
        Self::equals_epsilon(Some(self), Some(right), relative_epsilon, absolute_epsilon)
    }
}

impl PartialEq for HeadingPitchRoll {
    fn eq(&self, other: &Self) -> bool {
        self.heading == other.heading && self.pitch == other.pitch && self.roll == other.roll
    }
}
