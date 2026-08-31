//! Ported from `packages/engine/Source/DataSources/GridMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};

/// A material property that defines a grid pattern appearance.
pub struct GridMaterialProperty {
    /// The base color.
    pub color: Color,
    /// The grid line color.
    pub cell_alpha: f64,
    /// The number of horizontal grid lines.
    pub repeat_x: f64,
    /// The number of vertical grid lines.
    pub repeat_y: f64,
}

impl GridMaterialProperty {
    /// Creates a new grid material property.
    pub fn new() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            cell_alpha: 0.75,
            repeat_x: 8.0,
            repeat_y: 8.0,
        }
    }
}

impl Default for GridMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for GridMaterialProperty {
    fn type_name(&self) -> &str { "Grid" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}

/// Port of the CesiumJS `Property` facet of `GridMaterialProperty`.
impl Property for GridMaterialProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: the JS value is the material uniform object; the
        // value model reports the material type name.
        PropertyResult::String("Grid".to_string())
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
            .and_then(|any| any.downcast_ref::<GridMaterialProperty>())
            .map(|other| {
                self.color.red == other.color.red
                    && self.color.green == other.color.green
                    && self.color.blue == other.color.blue
                    && self.color.alpha == other.color.alpha
                    && self.cell_alpha == other.cell_alpha
                    && self.repeat_x == other.repeat_x
                    && self.repeat_y == other.repeat_y
            })
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("Grid")
    }
}
