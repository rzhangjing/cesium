//! Ported from `packages/engine/Source/Scene/I3SGeometry.js`.

/// I3S geometry.
pub struct I3SGeometry {
    _private: (),
}

impl I3SGeometry {
    /// Creates a new I3SGeometry.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SGeometry {
    fn default() -> Self { Self::new() }
}
