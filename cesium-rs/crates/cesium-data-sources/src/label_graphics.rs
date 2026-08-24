//! Ported from `packages/engine/Source/DataSources/LabelGraphics.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::near_far_scalar::NearFarScalar;

/// Graphics properties for a text label.
///
/// DEVIATION: the CesiumJS property objects are time-dynamic `Property`
/// instances; this simplified value model stores the constant subset of each
/// property directly.
#[derive(Clone)]
pub struct LabelGraphics {
    /// Whether this label is shown.
    pub show: bool,
    /// The text content.
    pub text: Option<String>,
    /// The font specification (e.g., "12pt Sans").
    pub font: Option<String>,
    /// The fill color.
    pub fill_color: Color,
    /// The outline color.
    pub outline_color: Color,
    /// The outline width in pixels.
    pub outline_width: f64,
    /// The scale factor.
    pub scale: f64,
    /// The style (mirrors `LabelStyle`: 0 = Fill, 1 = Outline,
    /// 2 = FillAndOutline).
    pub style: i32,
    /// The horizontal origin (mirrors `HorizontalOrigin` discriminant).
    pub horizontal_origin: i32,
    /// The vertical origin (mirrors `VerticalOrigin` discriminant).
    pub vertical_origin: i32,
    /// The eye offset.
    pub eye_offset: Option<Cartesian3>,
    /// The pixel offset.
    pub pixel_offset: Option<(f64, f64)>,
    /// The translucency applied based on the distance to the camera.
    pub translucency_by_distance: Option<NearFarScalar>,
    /// The pixel-offset scale applied based on the distance to the camera.
    pub pixel_offset_scale_by_distance: Option<NearFarScalar>,
}

impl LabelGraphics {
    /// Creates a new label graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            text: None,
            font: None,
            fill_color: Color::new(1.0, 1.0, 1.0, 1.0),
            outline_color: Color::new(0.0, 0.0, 0.0, 1.0),
            outline_width: 1.0,
            scale: 1.0,
            style: 0,
            horizontal_origin: 0,
            vertical_origin: 0,
            eye_offset: None,
            pixel_offset: None,
            translucency_by_distance: None,
            pixel_offset_scale_by_distance: None,
        }
    }
}

impl Default for LabelGraphics {
    fn default() -> Self { Self::new() }
}
