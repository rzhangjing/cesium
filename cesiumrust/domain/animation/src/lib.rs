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
pub mod spline;
pub mod timeline;
pub mod tween;

pub use interpolation::{
    catmull_rom, catmull_rom_vec3, hermite, hermite_vec3, interpolate, lagrange_interpolate,
    lagrange_interpolate_vec3, lerp, lerp_vec3, slerp_vec3, InterpolationType, SamplePoint,
};
pub use spline::{
    tridiagonal_solve, CatmullRomSpline, ConstantSpline, HermiteSpline, LinearSpline,
    MorphWeightSpline, QuaternionSpline, ScalarSpline, Spline, SteppedSpline,
};
pub use timeline::{
    AnimationController, SpeedPreset, TimelineConfig, TimelineState,
};
pub use tween::{EasingFunction, Tween, TweenCollection, TweenOptions};
