//! Ported from `packages/engine/Source/Scene/EllipsoidPrimitive.js`.

/// Ellipsoid primitive.
///
/// Renders an ellipsoid shape in the scene.
pub struct EllipsoidPrimitive {
    /// Whether the primitive is visible.
    pub show: bool,
    /// The radii.
    pub radii: (f64, f64, f64),
}

impl EllipsoidPrimitive {
    /// Creates a new EllipsoidPrimitive.
    pub fn new() -> Self { Self { show: true, radii: (1.0, 1.0, 1.0) } }
}

impl Default for EllipsoidPrimitive {
    fn default() -> Self { Self::new() }
}
