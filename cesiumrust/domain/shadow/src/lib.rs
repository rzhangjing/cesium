//! cesium-shadow: Shadow mapping and water/ocean effects.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/ShadowMap.js` → shadow_map
//! - Water/ocean rendering → water

pub mod shadow_map;
pub mod water;

pub use shadow_map::{
    PcfConfig, ShadowBias, ShadowBiasType, ShadowCameraParams, ShadowCascade,
    ShadowLightType, ShadowMap, ShadowMapConfig, ShadowMapType,
    SHADOW_MAP_MAXIMUM_DISTANCE,
};
pub use water::{GerstnerWave, OceanConfig, OceanSurface};
