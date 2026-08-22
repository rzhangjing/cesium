//! Ported from `packages/engine/Source/Core/Iau2006XysSample.js`.

/// An IAU 2006 XYS value sampled at a particular time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Iau2006XysSample {
    /// The X value.
    pub x: f64,
    /// The Y value.
    pub y: f64,
    /// The S value.
    pub s: f64,
}

impl Iau2006XysSample {
    pub fn new(x: f64, y: f64, s: f64) -> Self {
        Self { x, y, s }
    }
}
