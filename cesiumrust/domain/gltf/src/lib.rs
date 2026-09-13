//! cesium-gltf: glTF 2.0 domain models
//!
//! Maps to CesiumJS:
//! - `Scene/GltfLoader.js`
//! - `Scene/Batched3DModel3DTileContent.js`
//! - `Scene/Model/` (model rendering pipeline)
//! - `Scene/ModelComponents.js` (PBR materials, animation, skinning)
//! - `Scene/Model/CustomShader.js` (custom shader system)
//!
//! # Features
//! - glTF 2.0 JSON structure parsing (with sparse accessors)
//! - GLB binary container format
//! - b3dm (Batched 3D Model) format
//! - PBR material model with all KHR extensions
//! - Skeletal animation runtime (splines, skinning, morph targets)
//! - Custom shader system (uniforms, varyings, variable parsing)

pub mod animation_runtime;
pub mod binary_format;
pub mod custom_shader;
pub mod gltf_model;
pub mod material_ext;

pub use gltf_model::{
    Accessor, AccessorSparse, AccessorSparseIndices, AccessorSparseValues,
    AccessorType, AlphaMode, Animation, AnimationChannel, AnimationPath,
    AnimationSampler, AnimationTarget, Asset, Buffer, BufferTarget, BufferView,
    ComponentType, GltfMesh, GltfModel, Image, Interpolation, Material, Node,
    PbrMetallicRoughness, Primitive, PrimitiveMode, Sampler, Scene, Skin,
    Texture, TextureInfo,
};
pub use binary_format::{
    B3dmData, B3dmFeatureTable, BinaryFormatError, GlbData, B3DM_MAGIC,
    GLB_CHUNK_BIN, GLB_CHUNK_JSON, GLB_MAGIC,
};
pub use material_ext::{
    Anisotropy, Clearcoat, EmissiveStrength, ExtendedMaterial, Ior,
    MetallicRoughness, NormalTextureInfo, Specular, SpecularGlossiness,
    Sheen, TextureTransform, TextureTransformExtensions, TextureTransformInfo,
    Transmission, Volume, parse_material_extensions,
};
pub use animation_runtime::{
    AnimationLoop, AnimationSpline, AnimationState, ConstantSpline,
    CubicSpline, LinearSpline, MorphTargetBlender, QuaternionSpline,
    RuntimeAnimation, RuntimeChannel, RuntimeSkin, StepSpline,
    compute_duration,
};
pub use custom_shader::{
    CustomShader, CustomShaderMode, CustomShaderTranslucencyMode,
    ShaderError, UniformDeclaration, UniformType, UniformValue,
    UsedVariables, VaryingType,
};
