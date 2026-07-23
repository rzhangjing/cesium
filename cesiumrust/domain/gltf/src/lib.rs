//! cesium-gltf: glTF 2.0 domain models
//!
//! Maps to CesiumJS:
//! - `Scene/GltfLoader.js`
//! - `Scene/Batched3DModel3DTileContent.js`
//! - `Scene/Model/` (model rendering pipeline)
//!
//! # Features
//! - glTF 2.0 JSON structure parsing
//! - GLB binary container format
//! - b3dm (Batched 3D Model) format
//! - PBR material model
//! - Skeletal animation structures

pub mod gltf_model;
pub mod binary_format;

pub use gltf_model::{
    GltfModel, Asset, Scene, Node, GltfMesh, Primitive, PrimitiveMode,
    Accessor, AccessorType, ComponentType, BufferView, Buffer,
    Material, PbrMetallicRoughness, TextureInfo, Texture, Image, Sampler,
    Skin, Animation, AlphaMode,
};
pub use binary_format::{
    GlbData, B3dmData, B3dmFeatureTable, BinaryFormatError,
    GLB_MAGIC, GLB_CHUNK_JSON, GLB_CHUNK_BIN, B3DM_MAGIC,
};
