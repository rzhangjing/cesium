//! Ported from `packages/engine/Source/Scene/PolylineMaterialAppearance.js`.

/// Polyline material appearance.
///
/// Renders polylines with material-based shading.
pub struct PolylineMaterialAppearance {
    /// Whether the appearance is transparent.
    pub transparent: bool,
}

impl PolylineMaterialAppearance {
    /// Creates a new PolylineMaterialAppearance.
    pub fn new() -> Self { Self { transparent: false } }
}

impl Default for PolylineMaterialAppearance {
    fn default() -> Self { Self::new() }
}
