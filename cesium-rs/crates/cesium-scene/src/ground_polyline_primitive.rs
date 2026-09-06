//! Ported from `packages/engine/Source/Scene/GroundPolylinePrimitive.js`.

/// Ground polyline primitive.
///
/// Renders polylines draped onto terrain or 3D Tiles.
pub struct GroundPolylinePrimitive {
    /// Whether the primitive is visible.
    pub show: bool,
    /// Whether the primitive is ready.
    pub ready: bool,
}

impl GroundPolylinePrimitive {
    /// Creates a new GroundPolylinePrimitive.
    pub fn new() -> Self { Self { show: true, ready: false } }
}

impl Default for GroundPolylinePrimitive {
    fn default() -> Self { Self::new() }
}
