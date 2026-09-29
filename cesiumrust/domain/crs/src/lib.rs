//! cesium-crs：坐标系与投影。
//!
//! 状态（P2 代码健康审计，2026-09-27）：已实现并由 `cesium-specs`
//! 套件覆盖，但**未接入任何生产运行时路径** —— 没有任何
//! adapter 或 application crate 依赖它。作为为未来 adapter 桥接而保留的
//! CesiumJS 功能对齐领域模型留存；不要将其理解为已交付的能力。
//! 参见 docs/ARCHITECTURE.md 中的 "Test-only domain crates"。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! 实现内容：
//! - 地图投影（Web Mercator、UTM、极地球面投影、等距矩形）
//! - 基准（datum）定义与转换（WGS84、CGCS2000、ITRF、NAD83）
//! - Helmert/Molodensky 坐标转换

pub mod datum;
pub mod projections;

pub use datum::{
    Datum, DatumConverter, HelmertTransform, MolodenskyTransform,
    get_helmert_transform, transform_ecef,
};
pub use projections::{
    Equirectangular, GeographicCoordinate, PolarStereographic, ProjectedCoordinate,
    Utm, UtmZone, WebMercator,
};
