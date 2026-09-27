//! cesium-primitives: Geometry instances, appearances, and primitive collections.
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
//! - `Scene/GeometryInstance.js` → geometry_instance
//! - `Scene/Appearance.js` → geometry_instance
//! - `Scene/Primitive.js` → collection
//! - `Scene/PrimitiveCollection.js` → collection

pub mod collection;
pub mod geometry_instance;

pub use collection::{
    batch_instances, compute_bounding_sphere_union, BatchConfig, GeometryBatch, Primitive,
    PrimitiveCollection,
};
pub use geometry_instance::{
    Appearance, CullMode, GeometryInstance, GeometryType, MaterialType, RenderState,
};
