//! cesium-effects: Post-processing effects and particle systems.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/PostProcessStageLibrary.js` → post_process
//! - `Scene/ParticleSystem.js` → particles

pub mod particles;
pub mod post_process;

pub use particles::{
    EmitterShape, Particle, ParticleForce, ParticleSystem, ParticleSystemConfig,
};
pub use post_process::{
    AmbientOcclusionConfig, BloomConfig, ColorCorrectionConfig, FogConfig,
    PostProcessPipeline, PostProcessStageType, ToneMappingConfig, ToneMappingOperator,
};
