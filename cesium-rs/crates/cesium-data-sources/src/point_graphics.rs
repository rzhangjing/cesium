//! Ported from `packages/engine/Source/DataSources/PointGraphics.js`.

use cesium_core::color::Color;

/// Graphics properties for a point.
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
        }
    }
}

impl Default for PointGraphics {
    fn default() -> Self { Self::new() }
}
