//! cesium-gltf：glTF 2.0 领域模型
//!
//! 涵盖 glTF 2.0 的领域侧建模与模型渲染管线：
//! - glTF 加载器与批量 3D 模型（b3dm）内容
//! - 模型组件（PBR material、动画、蒙皮）
//! - 自定义 shader 系统（uniform、varying、变量解析）
//!
//! # 特性
//! - glTF 2.0 JSON 结构解析（含 sparse accessor）
//! - GLB 二进制容器格式
//! - b3dm（Batched 3D Model）格式
//! - 带有全部 KHR 扩展的 PBR material 模型
//! - 骨骼动画运行时（样条、蒙皮、morph target）
//! - 自定义 shader 系统（uniform、varying、变量解析）

pub mod animation_runtime;
pub mod binary_format;
pub mod custom_shader;
pub mod gltf_binary_stage;
pub mod gltf_model;
pub mod gltf_technique_upgrade;
pub mod gltf_upgrade;
pub mod gltf_upgrade_util;
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
    parse_glb_container, B3dmData, B3dmFeatureTable, BinaryFormatError, GlbData, B3DM_MAGIC,
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
pub use gltf_upgrade::{
    detect_version, update_version, update_version_with_buffers, GltfUpgradeError, GltfVersion,
    UpgradeOptions,
};
