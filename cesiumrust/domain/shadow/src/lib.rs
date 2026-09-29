//! cesium-shadow：阴影贴图与水/海洋效果。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：
//! - `Scene/ShadowMap.js` → shadow_map
//! - 水/海洋渲染 → water

pub mod shadow_map;
pub mod water;

pub use shadow_map::{
    PcfConfig, ShadowBias, ShadowBiasType, ShadowCameraParams, ShadowCascade,
    ShadowLightType, ShadowMap, ShadowMapConfig, ShadowMapType,
    SHADOW_MAP_MAXIMUM_DISTANCE,
};
pub use water::{GerstnerWave, OceanConfig, OceanSurface};
