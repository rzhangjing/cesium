//! Ported from `packages/engine/Source/Scene/PolylineColorAppearance.js`.

/// Polyline color appearance.
///
/// Renders polylines with per-vertex colors.
pub struct PolylineColorAppearance {
    /// Whether the appearance is transparent.
    pub transparent: bool,
}

impl PolylineColorAppearance {
    /// Creates a new PolylineColorAppearance.
    pub fn new() -> Self { Self { transparent: false } }
}

impl Default for PolylineColorAppearance {
    fn default() -> Self { Self::new() }
}
