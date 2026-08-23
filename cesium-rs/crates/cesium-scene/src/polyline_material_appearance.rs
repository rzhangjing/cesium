//! Ported from `packages/engine/Source/Scene/PolylineMaterialAppearance.js`.

/// An appearance for material-textured polylines.
pub struct PolylineMaterialAppearance {
    _private: (),
}

impl PolylineMaterialAppearance {
    /// Creates a new PolylineMaterialAppearance.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PolylineMaterialAppearance {
    fn default() -> Self { Self::new() }
}
