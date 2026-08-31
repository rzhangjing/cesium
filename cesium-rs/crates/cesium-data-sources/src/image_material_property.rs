//! Ported from `packages/engine/Source/DataSources/ImageMaterialProperty.js`.

use crate::material_property::MaterialProperty;
use crate::property::{Property, PropertyResult};

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

/// Port of the CesiumJS `Property` facet of `ImageMaterialProperty`.
impl Property for ImageMaterialProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        // DEVIATION: the JS value is the material uniform object; the
        // value model reports the material type name.
        PropertyResult::String("Image".to_string())
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
            .and_then(|any| any.downcast_ref::<ImageMaterialProperty>())
            .map(|other| {
                self.image == other.image
                    && self.repeat_x == other.repeat_x
                    && self.repeat_y == other.repeat_y
            })
            .unwrap_or(false)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn material_type_name(&self) -> Option<&'static str> {
        Some("Image")
    }
}
