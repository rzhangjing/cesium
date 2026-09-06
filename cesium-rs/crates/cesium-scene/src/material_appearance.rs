//! Ported from `packages/engine/Source/Scene/MaterialAppearance.js`.

/// Material appearance.
///
/// Renders primitives using a material for surface shading.
pub struct MaterialAppearance {
    /// Whether the appearance is transparent.
    pub transparent: bool,
    /// Whether to use flat shading.
    pub flat: bool,
}

impl MaterialAppearance {
    /// Creates a new MaterialAppearance.
    pub fn new() -> Self { Self { transparent: false, flat: false } }
}

impl Default for MaterialAppearance {
    fn default() -> Self { Self::new() }
}
