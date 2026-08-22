//! Ported from `packages/engine/Source/Core/EarthOrientationParametersSample.js`.

/// A set of Earth Orientation Parameters (EOP) sampled at a time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthOrientationParametersSample {
    /// The pole wander about the X axis, in radians.
    pub x_pole_wander: f64,
    /// The pole wander about the Y axis, in radians.
    pub y_pole_wander: f64,
    /// The offset to the CIP about the X axis, in radians.
    pub x_pole_offset: f64,
    /// The offset to the CIP about the Y axis, in radians.
    pub y_pole_offset: f64,
    /// The difference in time standards, UT1 - UTC, in seconds.
    pub ut1_minus_utc: f64,
}

impl EarthOrientationParametersSample {
    pub fn new(
        x_pole_wander: f64,
        y_pole_wander: f64,
        x_pole_offset: f64,
        y_pole_offset: f64,
        ut1_minus_utc: f64,
    ) -> Self {
        Self {
            x_pole_wander,
            y_pole_wander,
            x_pole_offset,
            y_pole_offset,
            ut1_minus_utc,
        }
    }
}
