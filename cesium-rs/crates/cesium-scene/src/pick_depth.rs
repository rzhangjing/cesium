//! Ported from `packages/engine/Source/Scene/PickDepth.js`.

/// Pick depth.
///
/// Tracks depth values during picking operations.
pub struct PickDepth {
    /// The depth value.
    pub depth: f64,
}

impl PickDepth {
    /// Creates a new PickDepth.
    pub fn new() -> Self { Self { depth: 0.0 } }
}

impl Default for PickDepth {
    fn default() -> Self { Self::new() }
}
