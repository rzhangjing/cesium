//! Ported from `packages/engine/Source/DataSources/PolylineGlowMaterialProperty.js`.

use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};

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

/// Port of the CesiumJS `Property` facet of `PolylineGlowMaterialProperty`.
impl Property for PolylineGlowMaterialProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: the JS value is the material uniform object; the
        // value model reports the material type name.
        PropertyResult::String("PolylineGlow".to_string())
    }

    fn is_constant(&self) -> bool {
        true
    }

    fn is_destroyed(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Property) -> bool {
        other
            .as_any()
            .and_then(|any| any.downcast_ref::<PolylineGlowMaterialProperty>())
            .is_some()
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("PolylineGlow")
    }
}
