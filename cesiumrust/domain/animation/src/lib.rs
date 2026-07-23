//! cesium-animation: Time-dynamic animation and interpolation.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Core/HermitePolynomialApproximation.js` → interpolation
//! - `Core/LagrangePolynomialApproximation.js` → interpolation
//! - `Widgets/Timeline/Timeline.js` → timeline
//! - `Widgets/Animation/AnimationViewModel.js` → timeline

pub mod interpolation;
pub mod timeline;

pub use interpolation::{
    catmull_rom, catmull_rom_vec3, hermite, hermite_vec3, interpolate, lagrange_interpolate,
    lagrange_interpolate_vec3, lerp, lerp_vec3, slerp_vec3, InterpolationType, SamplePoint,
};
pub use timeline::{
    AnimationController, SpeedPreset, TimelineConfig, TimelineState,
};
