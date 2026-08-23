//! Ported from `packages/engine/Source/DataSources/PolylineDashMaterialProperty.js`.

use crate::material_property::MaterialProperty;

/// A material property that defines a dashed pattern along a polyline.
pub struct PolylineDashMaterialProperty {
    _private: (),
}

impl PolylineDashMaterialProperty {
    /// Creates a new PolylineDash material property.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for PolylineDashMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for PolylineDashMaterialProperty {
    fn type_name(&self) -> &str { "PolylineDash" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
