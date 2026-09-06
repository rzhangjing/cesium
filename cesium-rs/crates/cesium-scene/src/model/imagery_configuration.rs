//! Ported from `packages/engine/Source/Scene/Model/ImageryConfiguration.js`.

use cesium_core::color::Color;

/// A class containing the values that affect the appearance of an
/// `ImageryLayer`.
///
/// This is used in the `ModelImagery` to detect changes in the imagery
/// settings: The `ModelImagery` stores one instance per imagery layer.
/// During the `update` call, it checks whether any of the settings was
/// changed. If this is the case, the draw commands of the model are reset.
#[derive(Debug, Clone)]
pub struct ImageryConfiguration {
    /// Whether the imagery layer is shown.
    pub show: bool,
    /// The alpha (transparency) value.
    pub alpha: f64,
    /// The brightness adjustment.
    pub brightness: f64,
    /// The contrast adjustment.
    pub contrast: f64,
    /// The hue adjustment.
    pub hue: f64,
    /// The saturation adjustment.
    pub saturation: f64,
    /// The gamma correction value.
    pub gamma: f64,
    /// The color-to-alpha value.
    pub color_to_alpha: Option<Color>,
}
