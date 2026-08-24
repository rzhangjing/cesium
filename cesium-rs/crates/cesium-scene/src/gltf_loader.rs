//! Ported from `packages/engine/Source/Scene/GltfLoader.js`.
//!
//! Loads glTF 2.0 assets and converts them to internal render resources.
//!
//! The `GltfJson` family of types below is the typed Rust representation of
//! a parsed glTF 2.0 JSON asset (CesiumJS operates on raw JS objects; the
//! Rust port uses serde-derived structs with the same property names).

use serde::{Deserialize, Serialize};

/// Loads glTF 2.0 assets and converts them to internal render resources.
///
/// This is the main entry point for loading glTF models. It handles:
/// - JSON parsing
/// - Buffer/image loading
/// - Mesh, material, animation, skin processing
/// - Converting to internal GPU resources (buffers, textures, pipelines)
pub struct GltfLoader {
    /// The glTF JSON data.
    gltf: Option<GltfJson>,
    /// Whether loading is complete.
    complete: bool,
    /// Whether loading has failed.
    failed: bool,
}

/// Parsed glTF JSON structure.
///
/// Mirrors the glTF 2.0 schema top-level properties (`asset`, `scene`,
/// `scenes`, `nodes`, ...). Optional arrays default to empty, matching the
/// effect of CesiumJS `GltfPipeline/addDefaults.js` for the properties the
/// port consumes on the CPU side.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfJson {
    /// The glTF asset metadata (required by the schema).
    #[serde(default)]
    pub asset: GltfAsset,
    /// Index of the default scene.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<u32>,
    /// The scenes.
    #[serde(default)]
    pub scenes: Vec<GltfScene>,
    /// The nodes.
    #[serde(default)]
    pub nodes: Vec<GltfNode>,
    /// The meshes.
    #[serde(default)]
    pub meshes: Vec<GltfMesh>,
    /// The materials.
    #[serde(default)]
    pub materials: Vec<GltfMaterial>,
    /// The accessors.
    #[serde(default)]
    pub accessors: Vec<GltfAccessor>,
    /// The buffer views.
    #[serde(default, rename = "bufferViews")]
    pub buffer_views: Vec<GltfBufferView>,
    /// The buffers.
    #[serde(default)]
    pub buffers: Vec<GltfBuffer>,
    /// The textures.
    #[serde(default)]
    pub textures: Vec<GltfTexture>,
    /// The samplers.
    #[serde(default)]
    pub samplers: Vec<GltfSampler>,
    /// The images.
    #[serde(default)]
    pub images: Vec<GltfImage>,
    /// The animations.
    #[serde(default)]
    pub animations: Vec<GltfAnimation>,
    /// The skins.
    #[serde(default)]
    pub skins: Vec<GltfSkin>,
    /// Names of extensions used in the asset.
    #[serde(default, rename = "extensionsUsed")]
    pub extensions_used: Vec<String>,
    /// Names of extensions required to consume the asset.
    #[serde(default, rename = "extensionsRequired")]
    pub extensions_required: Vec<String>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

/// glTF asset metadata (`asset` property).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfAsset {
    /// The glTF version string, e.g. `"2.0"`.
    #[serde(default)]
    pub version: String,
    /// The minimum glTF version, if specified.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "minVersion")]
    pub min_version: Option<String>,
    /// Tool that generated the asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    /// Copyright notice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

/// A glTF scene (a collection of root nodes).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfScene {
    /// Indices of the root nodes of this scene.
    #[serde(default)]
    pub nodes: Vec<u32>,
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A glTF node (transform + mesh reference).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfNode {
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Index of the mesh in this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<u32>,
    /// Indices of this node's children.
    #[serde(default)]
    pub children: Vec<u32>,
    /// Translation, if not identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<[f64; 3]>,
    /// Rotation as unit quaternion (x, y, z, w), if not identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<[f64; 4]>,
    /// Scale, if not identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<[f64; 3]>,
    /// A 4x4 column-major transformation matrix, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix: Option<[f64; 16]>,
    /// Index of the skin referenced by this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin: Option<u32>,
    /// Index of the camera referenced by this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<u32>,
    /// Weights for morph targets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weights: Vec<f64>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

/// A glTF mesh (a collection of primitives).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfMesh {
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The primitives of this mesh.
    #[serde(default)]
    pub primitives: Vec<GltfPrimitive>,
    /// Default morph target weights.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weights: Vec<f64>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

/// A glTF primitive (geometry + material reference).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfPrimitive {
    /// Map of attribute semantic to accessor index.
    #[serde(default)]
    pub attributes: std::collections::HashMap<String, u32>,
    /// Index of the accessor that contains the vertex indices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indices: Option<u32>,
    /// Index of the material to use to render this primitive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<u32>,
    /// The topology type of primitives to render
    /// (POINTS=0, LINES=1, LINE_LOOP=2, LINE_STRIP=3, TRIANGLES=4, ...).
    /// Defaults to 4 (TRIANGLES) per the glTF spec.
    #[serde(default = "default_primitive_mode")]
    pub mode: u32,
    /// Morph target attribute maps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<std::collections::HashMap<String, u32>>,
    /// Extension objects attached to this primitive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

fn default_primitive_mode() -> u32 {
    4
}

/// A glTF material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfMaterial {
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The PBR metallic-roughness material model.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "pbrMetallicRoughness")]
    pub pbr_metallic_roughness: Option<GltfPbrMetallicRoughness>,
    /// Normal texture reference.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "normalTexture")]
    pub normal_texture: Option<GltfNormalTextureInfo>,
    /// Occlusion texture reference.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "occlusionTexture")]
    pub occlusion_texture: Option<GltfTextureInfo>,
    /// Emissive texture reference.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "emissiveTexture")]
    pub emissive_texture: Option<GltfTextureInfo>,
    /// The factors for the emissive color. Defaults to `[0, 0, 0]`.
    #[serde(default, rename = "emissiveFactor")]
    pub emissive_factor: [f64; 3],
    /// The alpha rendering mode. Defaults to `"OPAQUE"`.
    #[serde(default = "default_alpha_mode", rename = "alphaMode")]
    pub alpha_mode: String,
    /// The alpha cutoff value. Defaults to `0.5`.
    #[serde(default = "default_alpha_cutoff", rename = "alphaCutoff")]
    pub alpha_cutoff: f64,
    /// Whether the material is double sided. Defaults to `false`.
    #[serde(default, rename = "doubleSided")]
    pub double_sided: bool,
    /// Extension objects attached to this material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

impl Default for GltfMaterial {
    fn default() -> Self {
        Self {
            name: None,
            pbr_metallic_roughness: None,
            normal_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            emissive_factor: [0.0, 0.0, 0.0],
            alpha_mode: default_alpha_mode(),
            alpha_cutoff: default_alpha_cutoff(),
            double_sided: false,
            extensions: None,
            extras: None,
        }
    }
}

fn default_alpha_mode() -> String {
    "OPAQUE".to_string()
}

fn default_alpha_cutoff() -> f64 {
    0.5
}

/// PBR metallic-roughness material model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfPbrMetallicRoughness {
    /// The RGBA components of the base color. Defaults to `[1, 1, 1, 1]`.
    #[serde(default = "default_base_color_factor", rename = "baseColorFactor")]
    pub base_color_factor: [f64; 4],
    /// The base color texture reference.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "baseColorTexture")]
    pub base_color_texture: Option<GltfTextureInfo>,
    /// The metallic factor. Defaults to `1.0`.
    #[serde(default = "default_one", rename = "metallicFactor")]
    pub metallic_factor: f64,
    /// The roughness factor. Defaults to `1.0`.
    #[serde(default = "default_one", rename = "roughnessFactor")]
    pub roughness_factor: f64,
    /// The metallic-roughness texture reference.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "metallicRoughnessTexture"
    )]
    pub metallic_roughness_texture: Option<GltfTextureInfo>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

impl Default for GltfPbrMetallicRoughness {
    fn default() -> Self {
        Self {
            base_color_factor: default_base_color_factor(),
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
            extras: None,
        }
    }
}

fn default_base_color_factor() -> [f64; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_one() -> f64 {
    1.0
}

/// A reference to a texture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfTextureInfo {
    /// The index of the texture.
    pub index: u32,
    /// The set index of the texture's TEXCOORD attribute. Defaults to `0`.
    #[serde(default, rename = "texCoord")]
    pub tex_coord: u32,
    /// Extension objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

/// A reference to a normal texture (adds `scale`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfNormalTextureInfo {
    /// The index of the texture.
    pub index: u32,
    /// The set index of the texture's TEXCOORD attribute. Defaults to `0`.
    #[serde(default, rename = "texCoord")]
    pub tex_coord: u32,
    /// Scalar multiplier applied to the sampled texel values. Defaults to `1.0`.
    #[serde(default = "default_one")]
    pub scale: f64,
    /// Extension objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

/// A glTF accessor (typed view into a buffer view).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfAccessor {
    /// Index of the buffer view. `None` when the accessor is filled with zeros.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "bufferView")]
    pub buffer_view: Option<u32>,
    /// The offset relative to the start of the buffer view in bytes. Defaults to `0`.
    #[serde(default, rename = "byteOffset")]
    pub byte_offset: u32,
    /// The datatype of the accessor's components (5120..=5126).
    #[serde(default, rename = "componentType")]
    pub component_type: u32,
    /// The number of elements referenced by this accessor.
    #[serde(default)]
    pub count: u32,
    /// Specifies if the attribute is a scalar, vector, or matrix
    /// (`SCALAR`, `VEC2`, `VEC3`, `VEC4`, `MAT2`, `MAT3`, `MAT4`).
    ///
    /// Named `gl_type` to avoid colliding with the Rust keyword `type`.
    #[serde(rename = "type")]
    pub gl_type: String,
    /// Minimum value of each component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Vec<f64>>,
    /// Maximum value of each component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Vec<f64>>,
    /// Whether integer data values are normalized before usage. Defaults to `false`.
    #[serde(default)]
    pub normalized: bool,
    /// Sparse storage of elements that deviate from their initialization value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sparse: Option<serde_json::Value>,
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

/// A glTF buffer view (a slice of a buffer).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfBufferView {
    /// The index of the buffer.
    #[serde(default)]
    pub buffer: u32,
    /// The offset into the buffer in bytes. Defaults to `0`.
    #[serde(default, rename = "byteOffset")]
    pub byte_offset: u32,
    /// The length of the bufferView in bytes.
    #[serde(default, rename = "byteLength")]
    pub byte_length: u32,
    /// The stride, in bytes, between vertex attributes.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "byteStride")]
    pub byte_stride: Option<u32>,
    /// The target that the GPU buffer should be bound to (34962 or 34963).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u32>,
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Extension objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

/// A glTF buffer (raw binary data).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfBuffer {
    /// The length of the buffer in bytes.
    #[serde(default, rename = "byteLength")]
    pub byte_length: u32,
    /// The URI of the buffer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Application-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
    /// The decoded buffer bytes.
    ///
    /// Rust analogue of CesiumJS `buffer.extras._pipeline.source` (the
    /// in-memory data attached by `parseGlb` / buffer loading). Not part of
    /// the glTF JSON schema, hence skipped during (de)serialization.
    #[serde(skip)]
    pub data: Option<Vec<u8>>,
}

/// A glTF texture sampler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfSampler {
    /// Magnification filter.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "magFilter")]
    pub mag_filter: Option<u32>,
    /// Minification filter.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "minFilter")]
    pub min_filter: Option<u32>,
    /// S (U) wrapping mode. Defaults to 10497 (`REPEAT`).
    #[serde(default = "default_repeat", rename = "wrapS")]
    pub wrap_s: u32,
    /// T (V) wrapping mode. Defaults to 10497 (`REPEAT`).
    #[serde(default = "default_repeat", rename = "wrapT")]
    pub wrap_t: u32,
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

fn default_repeat() -> u32 {
    10497
}

/// A glTF texture.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfTexture {
    /// The index of the sampler used by this texture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampler: Option<u32>,
    /// The index of the image used by this texture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<u32>,
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Extension objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

/// A glTF image.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfImage {
    /// The URI of the image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// The image's media type (required when `buffer_view` is defined).
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "mimeType")]
    pub mime_type: Option<String>,
    /// The index of the buffer view that contains the image.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "bufferView")]
    pub buffer_view: Option<u32>,
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A glTF animation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfAnimation {
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The animation channels.
    #[serde(default)]
    pub channels: Vec<GltfAnimationChannel>,
    /// The animation samplers.
    #[serde(default)]
    pub samplers: Vec<GltfAnimationSampler>,
}

/// An animation channel (target node + property path).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfAnimationChannel {
    /// The index of a sampler in this animation.
    #[serde(default)]
    pub sampler: u32,
    /// The descriptor of the animated property.
    #[serde(default)]
    pub target: GltfAnimationChannelTarget,
}

/// The descriptor of an animated node property.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfAnimationChannelTarget {
    /// The index of the node to animate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<u32>,
    /// The name of the node's property that is animated
    /// (`translation`, `rotation`, `scale`, `weights`).
    #[serde(default)]
    pub path: String,
}

/// An animation sampler (input/output accessors + interpolation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfAnimationSampler {
    /// The index of the accessor containing the keyframe input.
    #[serde(default)]
    pub input: u32,
    /// Interpolation algorithm. Defaults to `"LINEAR"`.
    #[serde(default = "default_interpolation")]
    pub interpolation: String,
    /// The index of the accessor containing the keyframe output.
    #[serde(default)]
    pub output: u32,
}

fn default_interpolation() -> String {
    "LINEAR".to_string()
}

/// A glTF skin (joint hierarchy + inverse bind matrices).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfSkin {
    /// Optional name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Indices of skeleton nodes.
    #[serde(default)]
    pub joints: Vec<u32>,
    /// The index of the node used as a skeleton root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skeleton: Option<u32>,
    /// The index of the accessor containing the 4x4 inverse-bind matrices.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "inverseBindMatrices")]
    pub inverse_bind_matrices: Option<u32>,
}

impl GltfLoader {
    /// Creates a new GltfLoader.
    pub fn new() -> Self {
        Self { gltf: None, complete: false, failed: false }
    }

    /// Returns whether loading is complete.
    pub fn is_complete(&self) -> bool { self.complete }

    /// Returns whether loading has failed.
    pub fn is_failed(&self) -> bool { self.failed }

    /// Returns the loaded glTF data.
    pub fn gltf(&self) -> Option<&GltfJson> { self.gltf.as_ref() }

    /// Stores the parsed glTF JSON and marks the loader complete.
    ///
    /// Rust analogue of the private `_gltf` assignment performed by the
    /// CesiumJS resource-cache pipeline once all sub-loaders resolve.
    pub fn set_gltf(&mut self, gltf: GltfJson) {
        self.gltf = Some(gltf);
        self.complete = true;
        self.failed = false;
    }

    /// Marks the loader as failed.
    pub fn set_failed(&mut self) {
        self.failed = true;
        self.complete = false;
    }
}

impl Default for GltfLoader {
    fn default() -> Self { Self::new() }
}
