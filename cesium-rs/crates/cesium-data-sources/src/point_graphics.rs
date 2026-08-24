//! Ported from `packages/engine/Source/DataSources/PointGraphics.js`.

use cesium_core::color::Color;
use cesium_core::near_far_scalar::NearFarScalar;

/// Graphics properties for a point.
///
/// DEVIATION: the CesiumJS property objects are time-dynamic `Property`
/// instances; this simplified value model stores the constant subset of each
/// property directly.
#[derive(Clone)]
pub struct PointGraphics {
    /// Whether this point is shown.
    pub show: bool,
    /// The fill color.
    pub color: Color,
    /// The outline color.
    pub outline_color: Color,
    /// The outline width in pixels.
    pub outline_width: f64,
    /// The size in pixels.
    pub pixel_size: f64,
    /// The scale applied based on the distance to the camera.
    pub scale_by_distance: Option<NearFarScalar>,
    /// The translucency applied based on the distance to the camera.
    pub translucency_by_distance: Option<NearFarScalar>,
    /// The height reference (mirrors `HeightReference`, stored as the enum
    /// discriminant: 0 = None, 1 = ClampToGround, 2 = RelativeToGround).
    pub height_reference: i32,
}

impl PointGraphics {
    /// Creates a new point graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            outline_color: Color::new(0.0, 0.0, 0.0, 1.0),
            outline_width: 0.0,
            pixel_size: 5.0,
            scale_by_distance: None,
            translucency_by_distance: None,
            height_reference: 0,
        }
    }
}

impl Default for PointGraphics {
    fn default() -> Self { Self::new() }
}
