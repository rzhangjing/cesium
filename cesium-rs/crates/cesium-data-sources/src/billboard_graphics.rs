//! Ported from `packages/engine/Source/DataSources/BillboardGraphics.js`.

use cesium_core::color::Color;

/// Graphics properties for a billboard.
///
/// Billboards are 2D images that always face the camera.
#[derive(Clone)]
pub struct BillboardGraphics {
    /// Whether this billboard is shown.
    pub show: bool,
    /// The image URL or data URI.
    pub image: Option<String>,
    /// The scale factor.
    pub scale: f64,
    /// The color tint.
    pub color: Color,
    /// The rotation angle in radians.
    pub rotation: f64,
    /// The horizontal origin (how the billboard is aligned relative to its position).
    pub horizontal_origin: i32,
    /// The vertical origin.
    pub vertical_origin: i32,
    /// The pixel offset.
    pub pixel_offset: Option<(f64, f64)>,
}

impl BillboardGraphics {
    /// Creates a new billboard graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            image: None,
            scale: 1.0,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            rotation: 0.0,
            horizontal_origin: 0,
            vertical_origin: 0,
            pixel_offset: None,
        }
    }
}

impl Default for BillboardGraphics {
    fn default() -> Self { Self::new() }
}
