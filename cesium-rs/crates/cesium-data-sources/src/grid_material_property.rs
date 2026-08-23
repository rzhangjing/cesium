//! Ported from `packages/engine/Source/DataSources/GridMaterialProperty.js`.

use cesium_core::color::Color;
use crate::material_property::MaterialProperty;

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
