//! # cesium-material
//!
//! The Fabric material system: a faithful Rust port of CesiumJS
//! `Scene/Material.js` and its built-in material library.
//!
//! A *Fabric* material is described declaratively in JSON (a
//! [`FabricTemplate`]) and assembled into GLSL shader source plus a set of
//! uniform values ([`Material`]). The domain layer performs the exact same
//! textual assembly that CesiumJS does (token renaming, sub-material splicing,
//! channel substitution, `czm_gammaCorrect` wrapping); the render adapter is
//! responsible for translating the resulting GLSL to the target shading
//! language.
//!
//! ## Bounded context
//!
//! This crate is the **material** bounded context (BC-16 in the architecture
//! plan). It depends only on `glam`-free pure Rust (`serde`, `serde_json`,
//! `thiserror`) and has no framework coupling, so every behaviour is unit
//! testable.
//!
//! ## Entry points
//!
//! - [`MaterialSystem::with_builtin_materials`] — a cache pre-populated with
//!   the 25 built-in CesiumJS materials.
//! - [`MaterialSystem::from_type`] — build a material from a cached type
//!   (maps to `Material.fromType`).
//! - [`MaterialSystem::create_material`] — build a material from a full Fabric
//!   template (maps to `new Material(...)`).
//! - [`FabricTemplate::from_json_str`] — parse a Fabric JSON document.

pub mod cache;
pub mod error;
pub mod fabric;
pub mod glsl;
pub mod material;
pub mod translucent;
pub mod uniform;

pub use cache::{CachedMaterial, MaterialSystem, BUILTIN_MATERIAL_TYPES};
pub use error::MaterialError;
pub use fabric::{FabricTemplate, MaterialComponents, COMPONENT_PROPERTIES, TEMPLATE_PROPERTIES};
pub use material::{Material, MaterialOptions};
pub use translucent::TranslucentSpec;
pub use uniform::{
    is_channel_string, uniform_value_from_json, CubeMapFaces, UniformValue, DEFAULT_CUBEMAP_ID,
    DEFAULT_IMAGE_ID,
};
