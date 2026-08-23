//! Ported from `packages/engine/Source/DataSources/PolylineArrowMaterialProperty.js`.

use crate::material_property::MaterialProperty;

/// A material property that defines an arrow pattern along a polyline.
pub struct PolylineArrowMaterialProperty {
    _private: (),
}

impl PolylineArrowMaterialProperty {
    /// Creates a new PolylineArrow material property.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for PolylineArrowMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for PolylineArrowMaterialProperty {
    fn type_name(&self) -> &str { "PolylineArrow" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
