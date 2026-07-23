//! glTF 2.0 domain model.
//!
//! Maps to CesiumJS `Scene/GltfLoader.js` and the glTF 2.0 specification.
//! This module defines the core glTF JSON structures for parsing and processing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The root glTF object.
///
/// Maps to the top-level JSON structure of a .gltf file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfModel {
    /// Asset metadata (required).
    pub asset: Asset,

    /// The default scene index.
    #[serde(default)]
    pub scene: Option<usize>,

    /// Array of scenes.
    #[serde(default)]
    pub scenes: Vec<Scene>,

    /// Array of nodes.
    #[serde(default)]
    pub nodes: Vec<Node>,

    /// Array of meshes.
    #[serde(default)]
    pub meshes: Vec<GltfMesh>,

    /// Array of accessors.
    #[serde(default)]
    pub accessors: Vec<Accessor>,

    /// Array of buffer views.
    #[serde(default)]
    pub buffer_views: Vec<BufferView>,

    /// Array of buffers.
    #[serde(default)]
    pub buffers: Vec<Buffer>,

    /// Array of materials.
    #[serde(default)]
    pub materials: Vec<Material>,

    /// Array of textures.
    #[serde(default)]
    pub textures: Vec<Texture>,

    /// Array of images.
    #[serde(default)]
    pub images: Vec<Image>,

    /// Array of samplers.
    #[serde(default)]
    pub samplers: Vec<Sampler>,

    /// Array of skins.
    #[serde(default)]
    pub skins: Vec<Skin>,

    /// Array of animations.
    #[serde(default)]
    pub animations: Vec<Animation>,

    /// Extensions used in this glTF.
    #[serde(default)]
    pub extensions_used: Vec<String>,

    /// Extensions required by this glTF.
    #[serde(default)]
    pub extensions_required: Vec<String>,

    /// Extension-specific data.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,

    /// Application-specific data.
    #[serde(default)]
    pub extras: Option<serde_json::Value>,
}

impl GltfModel {
    /// Parses a glTF model from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Parses a glTF model from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Returns the default scene, or the first scene if no default is set.
    pub fn default_scene(&self) -> Option<&Scene> {
        let index = self.scene.unwrap_or(0);
        self.scenes.get(index)
    }

    /// Returns the total number of triangles across all meshes.
    pub fn triangle_count(&self) -> usize {
        self.meshes
            .iter()
            .flat_map(|m| m.primitives.iter())
            .filter(|p| p.mode == PrimitiveMode::Triangles)
            .map(|p| {
                p.indices
                    .and_then(|i| self.accessors.get(i))
                    .map(|a| a.count / 3)
                    .unwrap_or(0)
            })
            .sum()
    }

    /// Returns the total number of vertices across all meshes.
    pub fn vertex_count(&self) -> usize {
        self.meshes
            .iter()
            .flat_map(|m| m.primitives.iter())
            .filter_map(|p| p.attributes.get("POSITION"))
            .filter_map(|i| self.accessors.get(*i))
            .map(|a| a.count)
            .sum()
    }
}

/// Asset metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// The glTF version (e.g., "2.0").
    pub version: String,

    /// The minimum glTF version required.
    #[serde(default)]
    pub min_version: Option<String>,

    /// Tool that generated this glTF.
    #[serde(default)]
    pub generator: Option<String>,

    /// Copyright information.
    #[serde(default)]
    pub copyright: Option<String>,
}

impl Default for Asset {
    fn default() -> Self {
        Self {
            version: "2.0".to_string(),
            min_version: None,
            generator: None,
            copyright: None,
        }
    }
}

/// A scene containing a list of root nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scene {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Indices of root nodes.
    #[serde(default)]
    pub nodes: Vec<usize>,
}

/// A node in the scene graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Indices of child nodes.
    #[serde(default)]
    pub children: Vec<usize>,

    /// Index of the mesh in this node.
    #[serde(default)]
    pub mesh: Option<usize>,

    /// Index of the skin referenced by this node.
    #[serde(default)]
    pub skin: Option<usize>,

    /// A 4x4 transformation matrix (column-major).
    #[serde(default)]
    pub matrix: Option<[f64; 16]>,

    /// Translation [x, y, z].
    #[serde(default)]
    pub translation: Option<[f64; 3]>,

    /// Rotation as quaternion [x, y, z, w].
    #[serde(default)]
    pub rotation: Option<[f64; 4]>,

    /// Scale [x, y, z].
    #[serde(default)]
    pub scale: Option<[f64; 3]>,

    /// Extension-specific data.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

impl Node {
    /// Computes the local transform matrix from TRS or matrix.
    pub fn local_transform(&self) -> glam::DMat4 {
        if let Some(m) = self.matrix {
            return glam::DMat4::from_cols_array(&m);
        }

        let translation = self
            .translation
            .map(|t| glam::DVec3::new(t[0], t[1], t[2]))
            .unwrap_or(glam::DVec3::ZERO);

        let rotation = self
            .rotation
            .map(|r| glam::DQuat::from_xyzw(r[0], r[1], r[2], r[3]))
            .unwrap_or(glam::DQuat::IDENTITY);

        let scale = self
            .scale
            .map(|s| glam::DVec3::new(s[0], s[1], s[2]))
            .unwrap_or(glam::DVec3::ONE);

        glam::DMat4::from_scale_rotation_translation(scale, rotation, translation)
    }
}

/// A mesh containing primitives.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfMesh {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Array of primitives.
    pub primitives: Vec<Primitive>,

    /// Morph target weights.
    #[serde(default)]
    pub weights: Vec<f64>,
}

/// A primitive (geometry) within a mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Primitive {
    /// Vertex attributes (e.g., "POSITION", "NORMAL", "TEXCOORD_0").
    pub attributes: HashMap<String, usize>,

    /// Index of the accessor containing indices.
    #[serde(default)]
    pub indices: Option<usize>,

    /// Index of the material.
    #[serde(default)]
    pub material: Option<usize>,

    /// The topology type (default: Triangles).
    #[serde(default)]
    pub mode: PrimitiveMode,

    /// Morph targets.
    #[serde(default)]
    pub targets: Vec<HashMap<String, usize>>,

    /// Extension-specific data.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// Primitive topology modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrimitiveMode {
    /// Points.
    Points = 0,
    /// Lines.
    Lines = 1,
    /// Line loop.
    LineLoop = 2,
    /// Line strip.
    LineStrip = 3,
    /// Triangles (default).
    #[default]
    Triangles = 4,
    /// Triangle strip.
    TriangleStrip = 5,
    /// Triangle fan.
    TriangleFan = 6,
}

impl Serialize for PrimitiveMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for PrimitiveMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Ok(match value {
            0 => PrimitiveMode::Points,
            1 => PrimitiveMode::Lines,
            2 => PrimitiveMode::LineLoop,
            3 => PrimitiveMode::LineStrip,
            4 => PrimitiveMode::Triangles,
            5 => PrimitiveMode::TriangleStrip,
            6 => PrimitiveMode::TriangleFan,
            _ => PrimitiveMode::Triangles,
        })
    }
}

/// An accessor for buffer data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accessor {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Index of the buffer view.
    #[serde(default)]
    pub buffer_view: Option<usize>,

    /// Byte offset into the buffer view.
    #[serde(default)]
    pub byte_offset: usize,

    /// The data type of components.
    pub component_type: ComponentType,

    /// Whether data is normalized.
    #[serde(default)]
    pub normalized: bool,

    /// Number of elements.
    pub count: usize,

    /// The type of the accessor (e.g., "VEC3", "SCALAR").
    #[serde(rename = "type")]
    pub accessor_type: AccessorType,

    /// Maximum values.
    #[serde(default)]
    pub max: Vec<f64>,

    /// Minimum values.
    #[serde(default)]
    pub min: Vec<f64>,
}

impl Accessor {
    /// Returns the number of components per element.
    pub fn components_per_element(&self) -> usize {
        match self.accessor_type {
            AccessorType::Scalar => 1,
            AccessorType::Vec2 => 2,
            AccessorType::Vec3 => 3,
            AccessorType::Vec4 => 4,
            AccessorType::Mat2 => 4,
            AccessorType::Mat3 => 9,
            AccessorType::Mat4 => 16,
        }
    }

    /// Returns the byte size of each component.
    pub fn component_byte_size(&self) -> usize {
        match self.component_type {
            ComponentType::I8 | ComponentType::U8 => 1,
            ComponentType::I16 | ComponentType::U16 => 2,
            ComponentType::U32 | ComponentType::F32 => 4,
        }
    }

    /// Returns the total byte stride for one element.
    pub fn element_byte_size(&self) -> usize {
        self.components_per_element() * self.component_byte_size()
    }
}

/// Component data types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Signed 8-bit integer (5120).
    I8,
    /// Unsigned 8-bit integer (5121).
    U8,
    /// Signed 16-bit integer (5122).
    I16,
    /// Unsigned 16-bit integer (5123).
    U16,
    /// Unsigned 32-bit integer (5125).
    U32,
    /// 32-bit float (5126).
    F32,
}

impl Serialize for ComponentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value: u32 = match self {
            ComponentType::I8 => 5120,
            ComponentType::U8 => 5121,
            ComponentType::I16 => 5122,
            ComponentType::U16 => 5123,
            ComponentType::U32 => 5125,
            ComponentType::F32 => 5126,
        };
        serializer.serialize_u32(value)
    }
}

impl<'de> Deserialize<'de> for ComponentType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Ok(match value {
            5120 => ComponentType::I8,
            5121 => ComponentType::U8,
            5122 => ComponentType::I16,
            5123 => ComponentType::U16,
            5125 => ComponentType::U32,
            5126 => ComponentType::F32,
            _ => ComponentType::F32, // Default to F32 for unknown
        })
    }
}

/// Accessor element types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessorType {
    /// Single scalar value.
    #[serde(rename = "SCALAR")]
    Scalar,
    /// 2D vector.
    #[serde(rename = "VEC2")]
    Vec2,
    /// 3D vector.
    #[serde(rename = "VEC3")]
    Vec3,
    /// 4D vector.
    #[serde(rename = "VEC4")]
    Vec4,
    /// 2x2 matrix.
    #[serde(rename = "MAT2")]
    Mat2,
    /// 3x3 matrix.
    #[serde(rename = "MAT3")]
    Mat3,
    /// 4x4 matrix.
    #[serde(rename = "MAT4")]
    Mat4,
}

/// A view into a buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferView {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Index of the buffer.
    pub buffer: usize,

    /// Byte offset into the buffer.
    #[serde(default)]
    pub byte_offset: usize,

    /// Length in bytes.
    pub byte_length: usize,

    /// Byte stride (for interleaved data).
    #[serde(default)]
    pub byte_stride: Option<usize>,

    /// Target buffer type.
    #[serde(default)]
    pub target: Option<BufferTarget>,
}

/// Buffer target types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BufferTarget {
    /// Array buffer (34962).
    ArrayBuffer,
    /// Element array buffer (34963).
    ElementArrayBuffer,
}

/// A binary data buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Buffer {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// URI to the buffer data (or None for embedded GLB data).
    #[serde(default)]
    pub uri: Option<String>,

    /// Length in bytes.
    pub byte_length: usize,
}

/// A material definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// PBR metallic-roughness parameters.
    #[serde(default)]
    pub pbr_metallic_roughness: Option<PbrMetallicRoughness>,

    /// Normal map texture info.
    #[serde(default)]
    pub normal_texture: Option<TextureInfo>,

    /// Occlusion map texture info.
    #[serde(default)]
    pub occlusion_texture: Option<TextureInfo>,

    /// Emissive map texture info.
    #[serde(default)]
    pub emissive_texture: Option<TextureInfo>,

    /// Emissive color [r, g, b].
    #[serde(default)]
    pub emissive_factor: Option<[f64; 3]>,

    /// Alpha mode.
    #[serde(default)]
    pub alpha_mode: Option<AlphaMode>,

    /// Alpha cutoff value.
    #[serde(default)]
    pub alpha_cutoff: Option<f64>,

    /// Whether the material is double-sided.
    #[serde(default)]
    pub double_sided: bool,

    /// Extension-specific data.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// PBR metallic-roughness material model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PbrMetallicRoughness {
    /// Base color [r, g, b, a].
    #[serde(default)]
    pub base_color_factor: Option<[f64; 4]>,

    /// Base color texture.
    #[serde(default)]
    pub base_color_texture: Option<TextureInfo>,

    /// Metallic factor (0.0 to 1.0).
    #[serde(default)]
    pub metallic_factor: Option<f64>,

    /// Roughness factor (0.0 to 1.0).
    #[serde(default)]
    pub roughness_factor: Option<f64>,

    /// Metallic-roughness texture.
    #[serde(default)]
    pub metallic_roughness_texture: Option<TextureInfo>,
}

/// Texture reference with coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureInfo {
    /// Index of the texture.
    pub index: usize,

    /// Texture coordinate set.
    #[serde(default)]
    pub tex_coord: usize,
}

/// A texture combining image and sampler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Texture {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Index of the sampler.
    #[serde(default)]
    pub sampler: Option<usize>,

    /// Index of the image.
    #[serde(default)]
    pub source: Option<usize>,
}

/// An image resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// URI to the image file.
    #[serde(default)]
    pub uri: Option<String>,

    /// MIME type.
    #[serde(default)]
    pub mime_type: Option<String>,

    /// Index of the buffer view containing the image.
    #[serde(default)]
    pub buffer_view: Option<usize>,
}

/// A texture sampler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sampler {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Magnification filter.
    #[serde(default)]
    pub mag_filter: Option<u32>,

    /// Minification filter.
    #[serde(default)]
    pub min_filter: Option<u32>,

    /// S (U) wrapping mode.
    #[serde(default)]
    pub wrap_s: Option<u32>,

    /// T (V) wrapping mode.
    #[serde(default)]
    pub wrap_t: Option<u32>,
}

/// A skin for skeletal animation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Index of the accessor containing inverse bind matrices.
    #[serde(default)]
    pub inverse_bind_matrices: Option<usize>,

    /// Index of the skeleton root node.
    #[serde(default)]
    pub skeleton: Option<usize>,

    /// Indices of joint nodes.
    pub joints: Vec<usize>,
}

/// An animation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Animation {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,

    /// Animation channels.
    pub channels: Vec<AnimationChannel>,

    /// Animation samplers.
    pub samplers: Vec<AnimationSampler>,
}

/// An animation channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationChannel {
    /// Index of the sampler.
    pub sampler: usize,

    /// Target of the animation.
    pub target: AnimationTarget,
}

/// Animation target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTarget {
    /// Index of the node to animate.
    pub node: usize,

    /// Property to animate.
    pub path: AnimationPath,
}

/// Animation property paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationPath {
    /// Translation.
    Translation,
    /// Rotation.
    Rotation,
    /// Scale.
    Scale,
    /// Morph weights.
    Weights,
}

/// An animation sampler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSampler {
    /// Index of the accessor containing keyframe timestamps.
    pub input: usize,

    /// Index of the accessor containing keyframe values.
    pub output: usize,

    /// Interpolation method.
    #[serde(default)]
    pub interpolation: Interpolation,
}

/// Interpolation methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Interpolation {
    /// Linear interpolation (default).
    #[default]
    Linear,
    /// Step interpolation.
    Step,
    /// Cubic spline interpolation.
    CubicSpline,
}

/// Alpha blending modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AlphaMode {
    /// Opaque (default).
    #[default]
    Opaque,
    /// Mask (binary transparency).
    Mask,
    /// Blend (alpha blending).
    Blend,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_gltf_json() -> &'static str {
        r#"{
            "asset": { "version": "2.0" },
            "scene": 0,
            "scenes": [{ "nodes": [0] }],
            "nodes": [{ "mesh": 0, "name": "TestNode" }],
            "meshes": [{
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "indices": 1,
                    "mode": 4
                }]
            }],
            "accessors": [
                { "componentType": 5126, "count": 3, "type": "VEC3" },
                { "componentType": 5123, "count": 3, "type": "SCALAR" }
            ],
            "bufferViews": [{ "buffer": 0, "byteLength": 44 }],
            "buffers": [{ "byteLength": 44 }]
        }"#
    }

    #[test]
    fn test_parse_minimal_gltf() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        assert_eq!(model.asset.version, "2.0");
        assert_eq!(model.scenes.len(), 1);
        assert_eq!(model.nodes.len(), 1);
        assert_eq!(model.meshes.len(), 1);
    }

    #[test]
    fn test_default_scene() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        let scene = model.default_scene().unwrap();
        assert_eq!(scene.nodes, vec![0]);
    }

    #[test]
    fn test_vertex_count() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        assert_eq!(model.vertex_count(), 3);
    }

    #[test]
    fn test_triangle_count() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        assert_eq!(model.triangle_count(), 1);
    }

    #[test]
    fn test_node_local_transform_identity() {
        let node = Node::default();
        assert_eq!(node.local_transform(), glam::DMat4::IDENTITY);
    }

    #[test]
    fn test_node_local_transform_trs() {
        let node = Node {
            translation: Some([1.0, 2.0, 3.0]),
            ..Default::default()
        };

        let transform = node.local_transform();
        let translation = transform.w_axis.truncate();
        assert!((translation.x - 1.0).abs() < 1e-10);
        assert!((translation.y - 2.0).abs() < 1e-10);
        assert!((translation.z - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_accessor_element_size() {
        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 100,
            accessor_type: AccessorType::Vec3,
            max: vec![],
            min: vec![],
        };

        assert_eq!(accessor.components_per_element(), 3);
        assert_eq!(accessor.component_byte_size(), 4);
        assert_eq!(accessor.element_byte_size(), 12);
    }

    #[test]
    fn test_material_parsing() {
        let json = r#"{
            "asset": { "version": "2.0" },
            "materials": [{
                "name": "TestMaterial",
                "pbrMetallicRoughness": {
                    "baseColorFactor": [1.0, 0.0, 0.0, 1.0],
                    "metallicFactor": 0.5,
                    "roughnessFactor": 0.8
                },
                "doubleSided": true
            }]
        }"#;

        let model = GltfModel::from_json(json).unwrap();
        assert_eq!(model.materials.len(), 1);

        let mat = &model.materials[0];
        assert_eq!(mat.name, Some("TestMaterial".to_string()));
        assert!(mat.double_sided);

        let pbr = mat.pbr_metallic_roughness.as_ref().unwrap();
        assert_eq!(pbr.metallic_factor, Some(0.5));
        assert_eq!(pbr.roughness_factor, Some(0.8));
    }

    #[test]
    fn test_serde_roundtrip() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        let serialized = serde_json::to_string(&model).unwrap();
        let reparsed = GltfModel::from_json(&serialized).unwrap();

        assert_eq!(model.asset.version, reparsed.asset.version);
        assert_eq!(model.nodes.len(), reparsed.nodes.len());
    }
}
