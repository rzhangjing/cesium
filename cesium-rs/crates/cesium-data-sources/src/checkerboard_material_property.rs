//! Ported from `packages/engine/Source/DataSources/CheckerboardMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;

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
