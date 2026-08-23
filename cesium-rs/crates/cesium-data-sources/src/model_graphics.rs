//! Ported from `packages/engine/Source/DataSources/ModelGraphics.js`.

use cesium_core::cartesian3::Cartesian3;

/// Graphics properties for a 3D model.
#[derive(Clone)]
pub struct ModelGraphics {
    /// Whether this model is shown.
    pub show: bool,
    /// The URI of the glTF model.
    pub uri: Option<String>,
    /// The scale factor.
    pub scale: f64,
    /// The minimum pixel size.
    pub minimum_pixel_size: f64,
    /// The maximum scale.
    pub maximum_scale: f64,
    /// Whether to show the outline.
    pub show_outline: bool,
    /// Whether to cast shadows.
    pub shadows: i32,
}

impl ModelGraphics {
    /// Creates a new model graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            uri: None,
            scale: 1.0,
            minimum_pixel_size: 0.0,
            maximum_scale: f64::MAX,
            show_outline: true,
            shadows: 0,
        }
    }
}

impl Default for ModelGraphics {
    fn default() -> Self { Self::new() }
}
