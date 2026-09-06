//! Ported from `packages/engine/Source/Scene/Light.js`.
//!
//! A light source. This trait describes an interface and is not intended
//! to be instantiated directly. Together, `color` and `intensity` produce
//! a high-dynamic-range light color.

use cesium_core::color::Color;

/// A light source.
///
/// Port of `Light`. This type describes an interface — use
/// [`DirectionalLight`](crate::directional_light::DirectionalLight) or
/// [`SunLight`](crate::sun_light::SunLight) for concrete instances.
pub trait Light {
    /// The color of the light.
    fn color(&self) -> &Color;

    /// The intensity controls the strength of the light. `intensity` has
    /// a minimum value of 0.0 and no maximum value.
    fn intensity(&self) -> f64;
}
