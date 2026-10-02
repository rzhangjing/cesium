//! cesium-effects：后处理效果与粒子系统。
//!
//! 领域层——纯 Rust，f64 精度。
//!
//! 模块划分：
//! - `post_process`：后处理管线与各效果配置（AO/泛光/雾/色调映射/颜色校正）
//! - `post_process_stage`：后处理阶段、阶段集合与复合体的组装模型
//! - `oit`：顺序无关透明（OIT）混合方程与能力配置
//! - `ibl`：基于图像的照明（球谐/重要性采样/预滤波等）
//! - `clipping`：裁剪平面与裁剪平面集合的求交
//! - `particles`：粒子系统与发射器形状/力场配置
//! - `cloud`：积云实体与云集合的批量管理
//! - `panorama`：等矩形与立方体贴图全景投影
//! - `geocoder`：地名搜索服务接口与结果模型
//! - `split`：分割方向（左右对比视图）枚举

pub mod clipping;
pub mod cloud;
pub mod geocoder;
pub mod ibl;
pub mod oit;
pub mod panorama;
pub mod particles;
pub mod post_process;
pub mod post_process_stage;
pub mod split;

pub use clipping::{ClippingPlane, ClippingPlaneCollection, Intersect};
pub use cloud::{CloudCollection, CloudType, CumulusCloud};
pub use geocoder::{
    GeocodeType, GeocoderAttribution, GeocoderDestination, GeocoderResult,
    GeocoderService, MockGeocoderService, get_credits_from_result,
};
pub use ibl::{
    default_spherical_harmonics, fibonacci_sphere, fresnel_schlick2, ggx_ndf, hammersley2d,
    importance_sample_ggx, integrate_brdf, prefilter_specular, project_irradiance_to_sh,
    radical_inverse_vdc, sh_polynomial_basis, smith_visibility_ggx, spherical_harmonics,
    texture_ibl, IblMaterial, ImageBasedLighting, IRRADIANCE_ZONAL_BY_BAND,
    SH_COEFFICIENT_COUNT, SH_ORTHONORMAL_CONSTANTS,
};
pub use oit::{BlendEquation, BlendFunction, OitCapabilities, OitConfig, OitMode};
pub use panorama::{
    CubeMapPanorama, EquirectangularPanorama, PanoramaProvider, DEFAULT_PANORAMA_RADIUS,
};
pub use particles::{
    EmitterShape, Particle, ParticleBurst, ParticleForce, ParticleSystem, ParticleSystemConfig,
};
pub use post_process::{
    AmbientOcclusionConfig, BloomConfig, ColorCorrectionConfig, FogConfig,
    PostProcessPipeline, PostProcessStageType, ToneMappingConfig, ToneMappingOperator,
};
pub use post_process_stage::{
    PixelFormat, PostProcessStage, PostProcessStageCollection, PostProcessStageComposite,
    SampleMode, StageRef, Tonemapper, UniformValue,
    create_ambient_occlusion_composite, create_auto_exposure_stage,
    create_bloom_composite, create_fxaa_stage,
};
pub use split::{SplitDirection, SplitterConfig};
