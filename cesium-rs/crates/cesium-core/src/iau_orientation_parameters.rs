//! Ported from `packages/engine/Source/Core/IauOrientationParameters.js`.

/// A structure containing the orientation data computed at a particular time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IauOrientationParameters {
    /// The right ascension of the north pole, in radians.
    pub right_ascension: f64,
    /// The declination of the north pole, in radians.
    pub declination: f64,
    /// The rotation about the north pole, in radians.
    pub rotation: f64,
    /// The instantaneous rotation rate about the north pole, in radians per second.
    pub rotation_rate: f64,
}

impl Default for IauOrientationParameters {
    fn default() -> Self {
        Self {
            right_ascension: 0.0,
            declination: 0.0,
            rotation: 0.0,
            rotation_rate: 0.0,
        }
    }
}

impl IauOrientationParameters {
    pub fn new(
        right_ascension: f64,
        declination: f64,
        rotation: f64,
        rotation_rate: f64,
    ) -> Self {
        Self {
            right_ascension,
            declination,
            rotation,
            rotation_rate,
        }
    }
}
