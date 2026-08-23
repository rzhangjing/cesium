//! Ported from `packages/engine/Source/Scene/PolylineColorAppearance.js`.

/// An appearance for colored polylines.
pub struct PolylineColorAppearance {
    _private: (),
}

impl PolylineColorAppearance {
    /// Creates a new PolylineColorAppearance.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PolylineColorAppearance {
    fn default() -> Self { Self::new() }
}
