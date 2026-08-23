//! Ported from `packages/engine/Source/Scene/GroundPolylinePrimitive.js`.

/// A ground polyline primitive.
pub struct GroundPolylinePrimitive {
    _private: (),
}

impl GroundPolylinePrimitive {
    /// Creates a new GroundPolylinePrimitive.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GroundPolylinePrimitive {
    fn default() -> Self { Self::new() }
}
