//! Ported from `packages/engine/Source/Scene/Billboard.js`.
//!
//! A single billboard in a BillboardCollection.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// A single billboard in a BillboardCollection.
///
/// A billboard is a 2D image that always faces the camera, positioned at
/// a 3D world coordinate.
pub struct Billboard {
    /// The position of the billboard in world coordinates.
    pub position: Cartesian3,
    /// The pixel offset from the position.
    pub pixel_offset: Cartesian2,
    /// The size of the billboard in pixels.
    pub size: Option<Cartesian2>,
    /// The color of the billboard.
    pub color: Color,
    /// The rotation angle in radians.
    pub rotation: f64,
    /// The scale factor.
    pub scale: f64,
    /// Whether the billboard is shown.
    pub show: bool,
    /// The image URL or data.
    pub image: Option<String>,
    /// The horizontal origin (how the billboard is anchored horizontally).
    pub horizontal_origin: i32,
    /// The vertical origin (how the billboard is anchored vertically).
    pub vertical_origin: i32,
    /// The eye offset for stereo rendering.
    pub eye_offset: Cartesian3,
    /// The translucency by distance.
    pub translucency_by_distance: Option<cesium_core::near_far_scalar::NearFarScalar>,
    /// The scale by distance.
    pub scale_by_distance: Option<cesium_core::near_far_scalar::NearFarScalar>,
    /// The distance display condition.
    pub distance_display_condition: Option<()>,
}

impl Billboard {
    /// Creates a new Billboard with default values.
    pub fn new() -> Self {
        Self {
            position: Cartesian3::default(),
            pixel_offset: Cartesian2::default(),
            size: None,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            rotation: 0.0,
            scale: 1.0,
            show: true,
            image: None,
            horizontal_origin: 0, // CENTER
            vertical_origin: 0,   // CENTER
            eye_offset: Cartesian3::default(),
            translucency_by_distance: None,
            scale_by_distance: None,
            distance_display_condition: None,
        }
    }
}

impl Default for Billboard {
    fn default() -> Self { Self::new() }
}
