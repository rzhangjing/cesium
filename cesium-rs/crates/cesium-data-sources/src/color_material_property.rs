//! Ported from `packages/engine/Source/DataSources/ColorMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;

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
