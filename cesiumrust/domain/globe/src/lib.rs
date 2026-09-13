//! cesium-globe: Globe surface rendering and atmosphere.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/Globe.js` → surface
//! - `Scene/SkyAtmosphere.js` → atmosphere
//! - `Scene/SkyBox.js` → atmosphere
//! - Globe lighting → atmosphere

pub mod atmosphere;
pub mod surface;

pub use atmosphere::{
    GlobeLighting, GroundAtmosphere, SkyAtmosphereConfig, SkyBoxConfig,
};
pub use surface::{GlobeConfig, GlobeSurface, GlobeTranslucency, NearFarScalar, ShadowMode};
