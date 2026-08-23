//! Ported from `packages/engine/Source/DataSources/PolylineGlowMaterialProperty.js`.

use crate::material_property::MaterialProperty;

/// A material property that defines a glowing pattern along a polyline.
pub struct PolylineGlowMaterialProperty {
    _private: (),
}

impl PolylineGlowMaterialProperty {
    /// Creates a new PolylineGlow material property.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for PolylineGlowMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for PolylineGlowMaterialProperty {
    fn type_name(&self) -> &str { "PolylineGlow" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
