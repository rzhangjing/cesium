//! Ported from `packages/engine/Source/Scene/computeFlyToLocationForRectangle.js`.

/// Computes fly-to location for a rectangle.
pub struct ComputeFlyToLocationForRectangle {
    _private: (),
}

impl ComputeFlyToLocationForRectangle {
    /// Creates a new ComputeFlyToLocationForRectangle.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ComputeFlyToLocationForRectangle {
    fn default() -> Self { Self::new() }
}
