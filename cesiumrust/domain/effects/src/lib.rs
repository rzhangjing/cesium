//! cesium-effects: Post-processing effects and particle systems.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/PostProcessStageLibrary.js` → post_process
//! - `Scene/PostProcessStage.js` → post_process_stage
//! - `Scene/PostProcessStageCollection.js` → post_process_stage
//! - `Scene/OIT.js` → oit
//! - `Scene/ImageBasedLighting.js` → ibl
//! - `Scene/ClippingPlaneCollection.js` → clipping
//! - `Scene/ParticleSystem.js` → particles
//! - `Scene/CumulusCloud.js` → cloud
//! - `Scene/CloudCollection.js` → cloud
//! - `Scene/EquirectangularPanorama.js` → panorama
//! - `Scene/CubeMapPanorama.js` → panorama
//! - `Core/GeocoderService.js` → geocoder
//! - `Scene/SplitDirection.js` → split

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
pub use ibl::{ImageBasedLighting, default_spherical_harmonics, SH_COEFFICIENT_COUNT};
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
