//! cesium-implicit-tiling：3D Tiles 1.1 隐式切分。
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
//! - [`implicit_tiling`]：隐式切分坐标、Morton 索引编解码与可用性位流

pub mod implicit_tiling;

pub use implicit_tiling::{
    morton_2d, morton_3d, decode_morton_2d, decode_morton_3d,
    AvailabilityBitstream, ImplicitTileCoord, ImplicitTilingConfig,
    SubdivisionScheme, Subtree,
};
