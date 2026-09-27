//! cesium-implicit-tiling: 3D Tiles 1.1 implicit tiling.
//!
//! STATUS (P2 code-health audit, 2026-09-27): implemented and covered by the
//! `cesium-specs` suite, but **not wired into any production runtime path** — no
//! adapter or application crate depends on it. Retained as a CesiumJS
//! feature-parity domain model reserved for future adapter bridging; do NOT read
//! it as a shipped capability. See docs/ARCHITECTURE.md "Test-only domain crates".
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/Implicit3DTileContent.js` → implicit_tiling

pub mod implicit_tiling;

pub use implicit_tiling::{
    morton_2d, morton_3d, decode_morton_2d, decode_morton_3d,
    AvailabilityBitstream, ImplicitTileCoord, ImplicitTilingConfig,
    SubdivisionScheme, Subtree,
};
