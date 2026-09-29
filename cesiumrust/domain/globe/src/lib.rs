//! cesium-globe：地球表面渲染与大气。
//!
//! 领域层 - 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：
//! - `Scene/Globe.js` → surface
//! - `Scene/SkyAtmosphere.js` → atmosphere
//! - `Scene/SkyBox.js` → atmosphere
//! - 地球光照（Globe lighting）→ atmosphere

pub mod atmosphere;
pub mod surface;

pub use atmosphere::{
    GlobeLighting, GroundAtmosphere, SkyAtmosphereConfig, SkyBoxConfig,
};
pub use surface::{GlobeConfig, GlobeSurface, GlobeTranslucency, NearFarScalar, ShadowMode};
