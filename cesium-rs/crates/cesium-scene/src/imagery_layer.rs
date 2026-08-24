//! Ported from `packages/engine/Source/Scene/ImageryLayer.js`.
//!
//! An imagery layer that can be added to a Globe's imagery layer collection.

use crate::imagery_provider::ImageryProvider;

/// An imagery layer that can be added to a Globe's imagery layer collection.
///
/// Each layer wraps an imagery provider and adds display properties like
/// alpha, brightness, contrast, hue, saturation, and gamma.
///
/// DEVIATION (B4-4): CesiumJS constructs the layer with
/// `new ImageryLayer(imageryProvider, options)`; the provider is required.
/// The Rust port keeps [`ImageryLayer::new`] provider-less (display
/// properties only, for spec fidelity of the property defaults) and adds
/// [`ImageryLayer::with_provider`] for the render path.
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
    /// The imagery provider backing this layer (`None` for property-only
    /// layers created via [`ImageryLayer::new`]).
    provider: Option<Box<dyn ImageryProvider>>,
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
            provider: None,
        }
    }

    /// Creates a new ImageryLayer wrapping `provider`, mirroring the
    /// CesiumJS `new ImageryLayer(imageryProvider, options)` constructor.
    pub fn with_provider(provider: Box<dyn ImageryProvider>) -> Self {
        Self {
            provider: Some(provider),
            ..Self::new()
        }
    }

    /// Returns the imagery provider backing this layer, if any.
    pub fn provider(&self) -> Option<&dyn ImageryProvider> {
        self.provider.as_deref()
    }

    /// Returns the imagery provider backing this layer (mutable), if any.
    pub fn provider_mut(&mut self) -> Option<&mut Box<dyn ImageryProvider>> {
        self.provider.as_mut()
    }
}

impl Default for ImageryLayer {
    fn default() -> Self { Self::new() }
}
