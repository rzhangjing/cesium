//! Ported from `packages/engine/Source/DataSources/PolylineOutlineMaterialProperty.js`.

use crate::material_property::MaterialProperty;

/// A material property that defines an outlined pattern along a polyline.
pub struct PolylineOutlineMaterialProperty {
    _private: (),
}

impl PolylineOutlineMaterialProperty {
    /// Creates a new PolylineOutline material property.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for PolylineOutlineMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for PolylineOutlineMaterialProperty {
    fn type_name(&self) -> &str { "PolylineOutline" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
