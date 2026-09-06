//! Ported from `packages/engine/Source/Scene/DirectionalLight.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::developer_error::throw_developer_error;

use crate::light::Light;

/// Options for creating a [`DirectionalLight`].
#[derive(Debug, Clone)]
pub struct DirectionalLightOptions {
    /// The direction in which light gets emitted.
    pub direction: Cartesian3,
    /// The color of the light (default: [`Color::WHITE`]).
    pub color: Option<Color>,
    /// The intensity of the light (default: 1.0).
    pub intensity: Option<f64>,
}

/// A light that gets emitted in a single direction from infinitely far away.
///
/// Port of `DirectionalLight`.
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    /// The direction in which light gets emitted.
    pub direction: Cartesian3,
    /// The color of the light.
    pub color: Color,
    /// The intensity of the light.
    pub intensity: f64,
}

impl DirectionalLight {
    /// Creates a new `DirectionalLight`.
    ///
    /// # Panics
    /// Panics (debug-only) if `options.direction` is a zero-length vector.
    pub fn new(options: DirectionalLightOptions) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if options.direction == Cartesian3::ZERO {
            throw_developer_error("options.direction cannot be zero-length");
        }
        //>>includeEnd('debug');

        Self {
            direction: options.direction,
            color: options.color.unwrap_or(Color::WHITE),
            intensity: options.intensity.unwrap_or(1.0),
        }
    }
}

impl Light for DirectionalLight {
    fn color(&self) -> &Color {
        &self.color
    }

    fn intensity(&self) -> f64 {
        self.intensity
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Cartesian3::new(0.0, 0.0, -1.0),
            color: Color::WHITE,
            intensity: 1.0,
        }
    }
}
