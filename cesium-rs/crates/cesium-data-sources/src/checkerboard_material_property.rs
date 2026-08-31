//! Ported from `packages/engine/Source/DataSources/CheckerboardMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};

/// A material property that defines a checkerboard pattern appearance.
pub struct CheckerboardMaterialProperty {
    /// The even color.
    pub even_color: Color,
    /// The odd color.
    pub odd_color: Color,
    /// The number of horizontal repetitions.
    pub repeat_x: f64,
    /// The number of vertical repetitions.
    pub repeat_y: f64,
}

impl CheckerboardMaterialProperty {
    /// Creates a new checkerboard material property.
    pub fn new() -> Self {
        Self {
            even_color: Color::new(1.0, 1.0, 1.0, 1.0),
            odd_color: Color::new(0.0, 0.0, 0.0, 1.0),
            repeat_x: 5.0,
            repeat_y: 5.0,
        }
    }
}

impl Default for CheckerboardMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for CheckerboardMaterialProperty {
    fn type_name(&self) -> &str { "Checkerboard" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}

/// Port of the CesiumJS `Property` facet of `CheckerboardMaterialProperty`.
impl Property for CheckerboardMaterialProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: the JS value is the material uniform object; the
        // value model reports the material type name.
        PropertyResult::String("Checkerboard".to_string())
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
            .and_then(|any| any.downcast_ref::<CheckerboardMaterialProperty>())
            .map(|other| {
                self.even_color.red == other.even_color.red
                    && self.even_color.green == other.even_color.green
                    && self.even_color.blue == other.even_color.blue
                    && self.even_color.alpha == other.even_color.alpha
                    && self.odd_color.red == other.odd_color.red
                    && self.odd_color.green == other.odd_color.green
                    && self.odd_color.blue == other.odd_color.blue
                    && self.odd_color.alpha == other.odd_color.alpha
                    && self.repeat_x == other.repeat_x
                    && self.repeat_y == other.repeat_y
            })
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("Checkerboard")
    }
}
