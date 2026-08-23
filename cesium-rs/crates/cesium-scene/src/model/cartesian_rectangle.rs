//! Ported from `packages/engine/Source/Scene/Model/CartesianRectangle.js`.

/// A rectangle in Cartesian coordinates.
pub struct CartesianRectangle {
    _private: (),
}

impl CartesianRectangle {
    /// Creates a new CartesianRectangle.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CartesianRectangle {
    fn default() -> Self { Self::new() }
}
