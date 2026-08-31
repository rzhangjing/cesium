//! Ported from `packages/engine/Source/DataSources/ColorMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};

/// A material property that defines a solid color appearance.
pub struct ColorMaterialProperty {
    /// The color of the material.
    pub color: Color,
}

impl ColorMaterialProperty {
    /// Creates a new color material property.
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl Default for ColorMaterialProperty {
    fn default() -> Self {
        Self { color: Color::new(1.0, 1.0, 1.0, 1.0) }
    }
}

impl MaterialProperty for ColorMaterialProperty {
    fn type_name(&self) -> &str { "Color" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}

/// Port of the CesiumJS `Property` facet of `ColorMaterialProperty`
/// (materials are properties; their value is the material uniform
/// payload).
impl Property for ColorMaterialProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        PropertyResult::Color(
            self.color.red,
            self.color.green,
            self.color.blue,
            self.color.alpha,
        )
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
            .and_then(|any| any.downcast_ref::<ColorMaterialProperty>())
            .map(|other| {
                self.color.red == other.color.red
                    && self.color.green == other.color.green
                    && self.color.blue == other.color.blue
                    && self.color.alpha == other.color.alpha
            })
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("Color")
    }
}
