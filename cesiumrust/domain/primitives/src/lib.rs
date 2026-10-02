//! cesium-primitives：几何实例、外观（appearance）与基本体集合。
//!
//! 状态（P2 代码健康审计，2026-09-27）：已实现并由 `cesium-specs`
//! 套件覆盖，但**未接入任何生产运行时路径** —— 没有任何
//! adapter 或 application crate 依赖它。作为为未来 adapter 桥接而保留的
//! CesiumJS 功能对齐领域模型留存；不要将其理解为已交付的能力。
//! 参见 docs/ARCHITECTURE.md 中的 "Test-only domain crates"。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! 子模块：
//! - [`geometry_instance`]：几何实例、外观、材质与绘制状态
//! - [`collection`]：基本体、基本体集合与几何合批

pub mod collection;
pub mod geometry_instance;

pub use collection::{
    batch_instances, compute_bounding_sphere_union, BatchConfig, GeometryBatch, Primitive,
    PrimitiveCollection,
};
pub use geometry_instance::{
    Appearance, CullMode, GeometryInstance, GeometryType, MaterialType, RenderState,
};
