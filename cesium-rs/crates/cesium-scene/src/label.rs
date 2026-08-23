//! Ported from `packages/engine/Source/Scene/Label.js`.
//!
//! A text label positioned in 3D space.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// A text label positioned in 3D space.
///
/// Labels are rendered as textured billboards with font glyphs.
pub struct Label {
    /// The position of the label in world coordinates.
    pub position: Cartesian3,
    /// The text content.
    pub text: String,
    /// The font family and size (CSS-like font string).
    pub font: String,
    /// The fill color.
    pub fill_color: Color,
    /// The outline color.
    pub outline_color: Color,
    /// The outline width in pixels.
    pub outline_width: f64,
    /// The scale factor.
    pub scale: f64,
    /// Whether the label is shown.
    pub show: bool,
    /// The pixel offset from the position.
    pub pixel_offset: cesium_core::cartesian2::Cartesian2,
    /// The horizontal origin.
    pub horizontal_origin: i32,
    /// The vertical origin.
    pub vertical_origin: i32,
    /// The style (FILL, OUTLINE, FILL_AND_OUTLINE).
    pub style: i32,
    /// Whether to show the background.
    pub show_background: bool,
    /// The background color.
    pub background_color: Color,
    /// The background padding.
    pub background_padding: cesium_core::cartesian2::Cartesian2,
}

impl Label {
    /// Creates a new Label with default values.
    pub fn new() -> Self {
        Self {
            position: Cartesian3::default(),
            text: String::new(),
            font: "30px sans-serif".to_string(),
            fill_color: Color::new(1.0, 1.0, 1.0, 1.0),
            outline_color: Color::new(0.0, 0.0, 0.0, 1.0),
            outline_width: 1.0,
            scale: 1.0,
            show: true,
            pixel_offset: cesium_core::cartesian2::Cartesian2::default(),
            horizontal_origin: 0,
            vertical_origin: 0,
            style: 2, // FILL_AND_OUTLINE
            show_background: false,
            background_color: Color::new(0.165, 0.165, 0.165, 0.8),
            background_padding: cesium_core::cartesian2::Cartesian2::new(7.0, 5.0),
        }
    }
}

impl Default for Label {
    fn default() -> Self { Self::new() }
}
