//! Ported from `packages/engine/Source/Scene/MaterialAppearance.js`.

/// A material appearance.
pub struct MaterialAppearance {
    _private: (),
}

impl MaterialAppearance {
    /// Creates a new MaterialAppearance.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MaterialAppearance {
    fn default() -> Self { Self::new() }
}
