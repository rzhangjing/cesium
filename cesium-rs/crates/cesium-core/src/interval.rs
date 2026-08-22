//! Ported from `packages/engine/Source/Core/Interval.js`.

/// Represents the closed interval [start, stop].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    /// The beginning of the interval.
    pub start: f64,
    /// The end of the interval.
    pub stop: f64,
}

impl Default for Interval {
    fn default() -> Self {
        Self {
            start: 0.0,
            stop: 0.0,
        }
    }
}

impl Interval {
    pub fn new(start: f64, stop: f64) -> Self {
        Self { start, stop }
    }
}
