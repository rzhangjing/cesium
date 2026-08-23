//! Ported from `packages/engine/Source/Scene/EllipsoidPrimitive.js`.

/// An ellipsoid primitive.
///
/// DEVIATION: stub implementation.
pub struct EllipsoidPrimitive {
    _private: (),
}

impl EllipsoidPrimitive {
    /// Creates a new ellipsoid primitive.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for EllipsoidPrimitive {
    fn default() -> Self { Self::new() }
}
