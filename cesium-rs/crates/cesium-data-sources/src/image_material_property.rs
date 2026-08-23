//! Ported from `packages/engine/Source/DataSources/ImageMaterialProperty.js`.

use crate::material_property::MaterialProperty;

/// A material property that defines an image/texture appearance.
pub struct ImageMaterialProperty {
    /// The URL of the image.
    pub image: Option<String>,
    /// Whether to repeat the image horizontally.
    pub repeat_x: f64,
    /// Whether to repeat the image vertically.
    pub repeat_y: f64,
}

impl ImageMaterialProperty {
    /// Creates a new image material property.
    pub fn new() -> Self {
        Self {
            image: None,
            repeat_x: 1.0,
            repeat_y: 1.0,
        }
    }
}

impl Default for ImageMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for ImageMaterialProperty {
    fn type_name(&self) -> &str { "Image" }
    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
