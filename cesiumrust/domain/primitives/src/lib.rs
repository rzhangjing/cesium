//! cesium-primitives: Geometry instances, appearances, and primitive collections.
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
