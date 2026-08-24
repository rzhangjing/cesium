//! Ported from `packages/engine/Source/DataSources/BillboardGraphics.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::near_far_scalar::NearFarScalar;

/// Graphics properties for a billboard.
///
/// Billboards are 2D images that always face the camera.
///
/// DEVIATION: the CesiumJS property objects are time-dynamic `Property`
/// instances; this simplified value model stores the constant subset of each
/// property directly.
#[derive(Clone)]
pub struct BillboardGraphics {
    /// Whether this billboard is shown.
    pub show: bool,
    /// The image URL or data URI.
    pub image: Option<String>,
    /// The scale factor.
    pub scale: f64,
    /// The color tint (`None` mirrors an unset/unknown color).
    pub color: Option<Color>,
    /// The rotation angle in radians.
    pub rotation: f64,
    /// The horizontal origin (how the billboard is aligned relative to its position).
    pub horizontal_origin: i32,
    /// The vertical origin.
    pub vertical_origin: i32,
    /// The pixel offset.
    pub pixel_offset: Option<(f64, f64)>,
    /// The eye offset.
    pub eye_offset: Option<Cartesian3>,
    /// The axis the billboard is aligned to.
    pub aligned_axis: Option<Cartesian3>,
    /// Whether `width`/`height` are in meters rather than pixels.
    pub size_in_meters: Option<bool>,
    /// The width in pixels (or meters, see `size_in_meters`).
    pub width: Option<f64>,
    /// The height in pixels (or meters, see `size_in_meters`).
    pub height: Option<f64>,
    /// The scale applied based on the distance to the camera.
    pub scale_by_distance: Option<NearFarScalar>,
    /// The translucency applied based on the distance to the camera.
    pub translucency_by_distance: Option<NearFarScalar>,
    /// The pixel-offset scale applied based on the distance to the camera.
    pub pixel_offset_scale_by_distance: Option<NearFarScalar>,
    /// The sub-region of the image used for the billboard
    /// (left, top, width, height; mirrors `BoundingRectangle`).
    pub image_sub_region: Option<(f64, f64, f64, f64)>,
    /// The height reference (mirrors `HeightReference`, stored as the enum
    /// discriminant: 0 = None, 1 = ClampToGround, 2 = RelativeToGround).
    pub height_reference: i32,
}

impl BillboardGraphics {
    /// Creates a new billboard graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            image: None,
            scale: 1.0,
            color: None,
            rotation: 0.0,
            horizontal_origin: 0,
            vertical_origin: 0,
            pixel_offset: None,
            eye_offset: None,
            aligned_axis: None,
            size_in_meters: None,
            width: None,
            height: None,
            scale_by_distance: None,
            translucency_by_distance: None,
            pixel_offset_scale_by_distance: None,
            image_sub_region: None,
            height_reference: 0,
        }
    }
}

impl Default for BillboardGraphics {
    fn default() -> Self { Self::new() }
}
