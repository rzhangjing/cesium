//! Ported from `packages/engine/Source/Scene/ImageryLayer.js`.
//!
//! An imagery layer that can be added to a Globe's imagery layer collection.

/// An imagery layer that can be added to a Globe's imagery layer collection.
///
/// Each layer wraps an imagery provider and adds display properties like
/// alpha, brightness, contrast, hue, saturation, and gamma.
pub struct ImageryLayer {
    /// The alpha blending value of this layer (0.0 = transparent, 1.0 = opaque).
    pub alpha: f64,
    /// The brightness of this layer (0.0 = black, 1.0 = original, 2.0 = white).
    pub brightness: f64,
    /// The contrast of this layer (0.0 = gray, 1.0 = original, 2.0 = full contrast).
    pub contrast: f64,
    /// The hue shift applied to this layer (in radians, -1.0 to 1.0).
    pub hue: f64,
    /// The saturation shift applied to this layer (0.0 = grayscale, 1.0 = original, 2.0 = oversaturated).
    pub saturation: f64,
    /// The gamma correction applied to this layer.
    pub gamma: f64,
    /// Whether this layer is shown.
    pub show: bool,
    /// The minimum terrain level-of-detail at which this layer is shown.
    pub minimum_imagery_level: Option<i32>,
    /// The maximum terrain level-of-detail at which this layer is shown.
    pub maximum_imagery_level: Option<i32>,
    /// The minimum ground resolution (in meters per pixel) at which this layer is shown.
    pub minimum_texture_ratio: f64,
    /// The maximum ground resolution (in meters per pixel) at which this layer is shown.
    pub maximum_texture_ratio: f64,
}

impl ImageryLayer {
    /// Creates a new ImageryLayer with default display properties.
    pub fn new() -> Self {
        Self {
            alpha: 1.0,
            brightness: 1.0,
            contrast: 1.0,
            hue: 0.0,
            saturation: 1.0,
            gamma: 1.0,
            show: true,
            minimum_imagery_level: None,
            maximum_imagery_level: None,
            minimum_texture_ratio: 0.0,
            maximum_texture_ratio: f64::INFINITY,
        }
    }
}

impl Default for ImageryLayer {
    fn default() -> Self { Self::new() }
}
