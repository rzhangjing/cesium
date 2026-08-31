//! Ported from `packages/engine/Source/DataSources/RectangleGraphics.js`.
//!
//! DEVIATION (simplified value model): the JS time-dynamic `Property`
//! fields are stored as plain constant values, mirroring the rest of the
//! data-sources port.

use cesium_core::color::Color;
use cesium_core::rectangle::Rectangle;

/// Graphics properties for a rectangle.
#[derive(Clone)]
pub struct RectangleGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
    /// The rectangle coordinates (mirrors `coordinates`).
    pub coordinates: Option<Rectangle>,
    /// The rotation of the rectangle, in radians (mirrors `rotation`).
    pub rotation: Option<f64>,
    /// The texture rotation of the rectangle, in radians (mirrors
    /// `stRotation`).
    pub st_rotation: Option<f64>,
    /// The height above the ellipsoid (mirrors `height`).
    pub height: Option<f64>,
    /// The draw order (mirrors `zIndex`).
    pub z_index: Option<f64>,
    /// The image url of an image material (mirrors
    /// `ImageMaterialProperty.image`; `None` mirrors a color material or
    /// no material).
    pub material_image: Option<String>,
    /// The material color (mirrors `material.color` for both image and
    /// color materials).
    pub material_color: Option<Color>,
    /// Whether the image material is transparent (mirrors
    /// `ImageMaterialProperty.transparent`; the KML loader always sets it
    /// for image materials).
    pub material_transparent: bool,
}

impl RectangleGraphics {
    /// Creates a new Rectangle graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            coordinates: None,
            rotation: None,
            st_rotation: None,
            height: None,
            z_index: None,
            material_image: None,
            material_color: None,
            material_transparent: false,
        }
    }
}

impl Default for RectangleGraphics {
    fn default() -> Self { Self::new() }
}
