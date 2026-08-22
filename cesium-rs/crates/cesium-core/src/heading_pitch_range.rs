//! Ported from `packages/engine/Source/Core/HeadingPitchRange.js`.

/// Defines a heading angle, pitch angle, and range in a local frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadingPitchRange {
    /// The heading angle in radians.
    pub heading: f64,
    /// The pitch angle in radians.
    pub pitch: f64,
    /// The distance from the center in meters.
    pub range: f64,
}

impl Default for HeadingPitchRange {
    fn default() -> Self {
        Self {
            heading: 0.0,
            pitch: 0.0,
            range: 0.0,
        }
    }
}

impl HeadingPitchRange {
    pub fn new(heading: f64, pitch: f64, range: f64) -> Self {
        Self {
            heading,
            pitch,
            range,
        }
    }

    /// Duplicates a HeadingPitchRange instance.
    pub fn clone_hpr(&self) -> Self {
        *self
    }
}
