//! Ported from `packages/engine/Source/Scene/SunLight.js`.

use cesium_core::color::Color;

use crate::light::Light;

/// Options for creating a [`SunLight`].
#[derive(Debug, Clone, Default)]
pub struct SunLightOptions {
    /// The light's color (default: [`Color::WHITE`]).
    pub color: Option<Color>,
    /// The light's intensity (default: 2.0).
    pub intensity: Option<f64>,
}

/// A directional light source that originates from the Sun.
///
/// Port of `SunLight`.
#[derive(Debug, Clone)]
pub struct SunLight {
    /// The color of the light.
    pub color: Color,
    /// The intensity of the light.
    pub intensity: f64,
}

impl SunLight {
    /// Creates a new `SunLight`.
    pub fn new(options: Option<SunLightOptions>) -> Self {
        let opts = options.unwrap_or_default();
        Self {
            color: opts.color.unwrap_or(Color::WHITE),
            intensity: opts.intensity.unwrap_or(2.0),
        }
    }
}

impl Light for SunLight {
    fn color(&self) -> &Color {
        &self.color
    }

    fn intensity(&self) -> f64 {
        self.intensity
    }
}

impl Default for SunLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 2.0,
        }
    }
}
