//! Ported from `packages/engine/Source/DataSources/PolylineArrowMaterialProperty.js`.

use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};

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

/// Port of the CesiumJS `Property` facet of `PolylineArrowMaterialProperty`.
impl Property for PolylineArrowMaterialProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: the JS value is the material uniform object; the
        // value model reports the material type name.
        PropertyResult::String("PolylineArrow".to_string())
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
            .and_then(|any| any.downcast_ref::<PolylineArrowMaterialProperty>())
            .is_some()
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("PolylineArrow")
    }
}
