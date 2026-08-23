//! Ported from `packages/engine/Source/DataSources/LabelGraphics.js`.

use cesium_core::color::Color;

/// Graphics properties for a text label.
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
    /// The style (FILL, OUTLINE, FILL_AND_OUTLINE).
    pub style: i32,
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
        }
    }
}

impl Default for LabelGraphics {
    fn default() -> Self { Self::new() }
}
