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

/// Sparse accessor data for overriding specific elements.
///
/// Maps to glTF 2.0 `accessor.sparse`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessorSparse {
    /// Number of elements overridden.
    pub count: usize,

    /// Indices of elements to override.
    pub indices: AccessorSparseIndices,

    /// Replacement values.
    pub values: AccessorSparseValues,
}

/// Sparse accessor indices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessorSparseIndices {
    /// Index of the buffer view.
    pub buffer_view: usize,

    /// Byte offset into the buffer view.
    #[serde(default)]
    pub byte_offset: usize,

    /// Component type of indices (5121=u8, 5123=u16, 5125=u32).
    pub component_type: ComponentType,
}

/// Sparse accessor values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessorSparseValues {
    /// Index of the buffer view.
    pub buffer_view: usize,

    /// Byte offset into the buffer view.
    #[serde(default)]
    pub byte_offset: usize,
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

    /// Sparse accessor overrides.
    #[serde(default)]
    pub sparse: Option<AccessorSparse>,
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

    /// Returns true if this accessor has sparse overrides.
    pub fn is_sparse(&self) -> bool {
        self.sparse.is_some()
    }

    /// Reads f32 data from a binary buffer using this accessor.
    ///
    /// Maps to CesiumJS `GltfLoaderUtility.getAccessorData`
    pub fn read_f32_data(&self, buffers: &[Vec<u8>], buffer_views: &[BufferView]) -> Vec<f32> {
        let total_components = self.count * self.components_per_element();
        let mut data = vec![0.0f32; total_components];

        // Read base data from buffer view
        if let Some(bv_idx) = self.buffer_view {
            if let Some(bv) = buffer_views.get(bv_idx) {
                if let Some(buffer) = buffers.get(bv.buffer) {
                    let stride = bv.byte_stride.unwrap_or(self.element_byte_size());
                    let base_offset = bv.byte_offset + self.byte_offset;

                    for i in 0..self.count {
                        let elem_offset = base_offset + i * stride;
                        for c in 0..self.components_per_element() {
                            let byte_pos = elem_offset + c * 4;
                            if byte_pos + 4 <= buffer.len() {
                                let bytes = [
                                    buffer[byte_pos],
                                    buffer[byte_pos + 1],
                                    buffer[byte_pos + 2],
                                    buffer[byte_pos + 3],
                                ];
                                data[i * self.components_per_element() + c] =
                                    f32::from_le_bytes(bytes);
                            }
                        }
                    }
                }
            }
        }

        // Apply sparse overrides
        if let Some(ref sparse) = self.sparse {
            self.apply_sparse_f32(&mut data, sparse, buffers, buffer_views);
        }

        data
    }

    /// Reads u16 index data from a binary buffer.
    pub fn read_u16_data(&self, buffers: &[Vec<u8>], buffer_views: &[BufferView]) -> Vec<u16> {
        let mut data = vec![0u16; self.count];

        if let Some(bv_idx) = self.buffer_view {
            if let Some(bv) = buffer_views.get(bv_idx) {
                if let Some(buffer) = buffers.get(bv.buffer) {
                    let stride = bv.byte_stride.unwrap_or(2);
                    let base_offset = bv.byte_offset + self.byte_offset;

                    for (i, item) in data.iter_mut().enumerate().take(self.count) {
                        let byte_pos = base_offset + i * stride;
                        if byte_pos + 2 <= buffer.len() {
                            *item = u16::from_le_bytes([
                                buffer[byte_pos],
                                buffer[byte_pos + 1],
                            ]);
                        }
                    }
                }
            }
        }

        data
    }

    /// Reads u32 index data from a binary buffer.
    pub fn read_u32_data(&self, buffers: &[Vec<u8>], buffer_views: &[BufferView]) -> Vec<u32> {
        let mut data = vec![0u32; self.count];

        if let Some(bv_idx) = self.buffer_view {
            if let Some(bv) = buffer_views.get(bv_idx) {
                if let Some(buffer) = buffers.get(bv.buffer) {
                    let stride = bv.byte_stride.unwrap_or(4);
                    let base_offset = bv.byte_offset + self.byte_offset;

                    for (i, item) in data.iter_mut().enumerate().take(self.count) {
                        let byte_pos = base_offset + i * stride;
                        if byte_pos + 4 <= buffer.len() {
                            *item = u32::from_le_bytes([
                                buffer[byte_pos],
                                buffer[byte_pos + 1],
                                buffer[byte_pos + 2],
                                buffer[byte_pos + 3],
                            ]);
                        }
                    }
                }
            }
        }

        data
    }

    /// Applies sparse overrides to f32 data.
    fn apply_sparse_f32(
        &self,
        data: &mut [f32],
        sparse: &AccessorSparse,
        buffers: &[Vec<u8>],
        buffer_views: &[BufferView],
    ) {
        // Read sparse indices
        let indices = self.read_sparse_indices(sparse, buffers, buffer_views);

        // Read sparse values
        if let Some(values_bv) = buffer_views.get(sparse.values.buffer_view) {
            if let Some(buffer) = buffers.get(values_bv.buffer) {
                let components = self.components_per_element();
                let base_offset = values_bv.byte_offset + sparse.values.byte_offset;

                for (sparse_idx, &target_idx) in indices.iter().enumerate() {
                    if target_idx >= self.count {
                        continue;
                    }
                    for c in 0..components {
                        let byte_pos =
                            base_offset + sparse_idx * components * 4 + c * 4;
                        if byte_pos + 4 <= buffer.len() {
                            let bytes = [
                                buffer[byte_pos],
                                buffer[byte_pos + 1],
                                buffer[byte_pos + 2],
                                buffer[byte_pos + 3],
                            ];
                            let target = target_idx * components + c;
                            if target < data.len() {
                                data[target] = f32::from_le_bytes(bytes);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Reads sparse indices as usize values.
    fn read_sparse_indices(
        &self,
        sparse: &AccessorSparse,
        buffers: &[Vec<u8>],
        buffer_views: &[BufferView],
    ) -> Vec<usize> {
        let mut indices = Vec::with_capacity(sparse.count);

        if let Some(bv) = buffer_views.get(sparse.indices.buffer_view) {
            if let Some(buffer) = buffers.get(bv.buffer) {
                let base_offset = bv.byte_offset + sparse.indices.byte_offset;

                for i in 0..sparse.count {
                    let idx = match sparse.indices.component_type {
                        ComponentType::U8 => {
                            let pos = base_offset + i;
                            if pos < buffer.len() {
                                buffer[pos] as usize
                            } else {
                                0
                            }
                        }
                        ComponentType::U16 => {
                            let pos = base_offset + i * 2;
                            if pos + 2 <= buffer.len() {
                                u16::from_le_bytes([buffer[pos], buffer[pos + 1]])
                                    as usize
                            } else {
                                0
                            }
                        }
                        ComponentType::U32 => {
                            let pos = base_offset + i * 4;
                            if pos + 4 <= buffer.len() {
                                u32::from_le_bytes([
                                    buffer[pos],
                                    buffer[pos + 1],
                                    buffer[pos + 2],
                                    buffer[pos + 3],
                                ]) as usize
                            } else {
                                0
                            }
                        }
                        _ => 0,
                    };
                    indices.push(idx);
                }
            }
        }

        indices
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
            sparse: None,
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

    #[test]
    fn test_accessor_read_f32_data() {
        // Create a buffer with 3 f32 values: 1.0, 2.0, 3.0
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&1.0f32.to_le_bytes());
        buffer.extend_from_slice(&2.0f32.to_le_bytes());
        buffer.extend_from_slice(&3.0f32.to_le_bytes());

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 12,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 3,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert_eq!(data.len(), 3);
        assert!((data[0] - 1.0).abs() < 1e-6);
        assert!((data[1] - 2.0).abs() < 1e-6);
        assert!((data[2] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_accessor_read_f32_vec3() {
        // 2 VEC3 elements: (1,2,3) and (4,5,6)
        let mut buffer = Vec::new();
        for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 24,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 2,
            accessor_type: AccessorType::Vec3,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert_eq!(data.len(), 6);
        assert!((data[0] - 1.0).abs() < 1e-6);
        assert!((data[3] - 4.0).abs() < 1e-6);
        assert!((data[5] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_accessor_read_u16_data() {
        let mut buffer = Vec::new();
        for v in [10u16, 20, 30, 40] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 8,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::U16,
            normalized: false,
            count: 4,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_u16_data(&buffers, &buffer_views);
        assert_eq!(data, vec![10, 20, 30, 40]);
    }

    #[test]
    fn test_accessor_read_u32_data() {
        let mut buffer = Vec::new();
        for v in [100u32, 200, 300] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 12,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::U32,
            normalized: false,
            count: 3,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_u32_data(&buffers, &buffer_views);
        assert_eq!(data, vec![100, 200, 300]);
    }

    #[test]
    fn test_accessor_sparse() {
        // Base data: [0.0, 0.0, 0.0] (3 scalars)
        // Sparse: override index 1 with value 5.0
        let mut base_buffer = Vec::new();
        base_buffer.extend_from_slice(&0.0f32.to_le_bytes());
        base_buffer.extend_from_slice(&0.0f32.to_le_bytes());
        base_buffer.extend_from_slice(&0.0f32.to_le_bytes());

        // Sparse indices buffer: [1u16]
        let mut idx_buffer = Vec::new();
        idx_buffer.extend_from_slice(&1u16.to_le_bytes());

        // Sparse values buffer: [5.0f32]
        let mut val_buffer = Vec::new();
        val_buffer.extend_from_slice(&5.0f32.to_le_bytes());

        let buffers = vec![base_buffer, idx_buffer, val_buffer];
        let buffer_views = vec![
            BufferView {
                name: None,
                buffer: 0,
                byte_offset: 0,
                byte_length: 12,
                byte_stride: None,
                target: None,
            },
            BufferView {
                name: None,
                buffer: 1,
                byte_offset: 0,
                byte_length: 2,
                byte_stride: None,
                target: None,
            },
            BufferView {
                name: None,
                buffer: 2,
                byte_offset: 0,
                byte_length: 4,
                byte_stride: None,
                target: None,
            },
        ];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 3,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: Some(AccessorSparse {
                count: 1,
                indices: AccessorSparseIndices {
                    buffer_view: 1,
                    byte_offset: 0,
                    component_type: ComponentType::U16,
                },
                values: AccessorSparseValues {
                    buffer_view: 2,
                    byte_offset: 0,
                },
            }),
        };

        assert!(accessor.is_sparse());
        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert!((data[0] - 0.0).abs() < 1e-6);
        assert!((data[1] - 5.0).abs() < 1e-6); // overridden
        assert!((data[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_accessor_with_byte_stride() {
        // Interleaved: position(3f) + normal(3f) = 24 bytes stride
        // Only read positions (first 3 floats of each 6-float stride)
        let mut buffer = Vec::new();
        // Element 0: pos(1,2,3) + normal(0,0,1)
        for v in [1.0f32, 2.0, 3.0, 0.0, 0.0, 1.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        // Element 1: pos(4,5,6) + normal(0,1,0)
        for v in [4.0f32, 5.0, 6.0, 0.0, 1.0, 0.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 48,
            byte_stride: Some(24), // 6 floats * 4 bytes
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 2,
            accessor_type: AccessorType::Vec3,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert_eq!(data.len(), 6);
        assert!((data[0] - 1.0).abs() < 1e-6);
        assert!((data[1] - 2.0).abs() < 1e-6);
        assert!((data[2] - 3.0).abs() < 1e-6);
        assert!((data[3] - 4.0).abs() < 1e-6);
        assert!((data[4] - 5.0).abs() < 1e-6);
        assert!((data[5] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_sparse_accessor_json_parsing() {
        let json = r#"{
            "asset": { "version": "2.0" },
            "accessors": [{
                "componentType": 5126,
                "count": 3,
                "type": "SCALAR",
                "sparse": {
                    "count": 1,
                    "indices": {
                        "bufferView": 1,
                        "componentType": 5123
                    },
                    "values": {
                        "bufferView": 2
                    }
                }
            }],
            "bufferViews": [
                { "buffer": 0, "byteLength": 12 },
                { "buffer": 0, "byteLength": 2, "byteOffset": 12 },
                { "buffer": 0, "byteLength": 4, "byteOffset": 14 }
            ],
            "buffers": [{ "byteLength": 18 }]
        }"#;

        let model = GltfModel::from_json(json).unwrap();
        let accessor = &model.accessors[0];
        assert!(accessor.is_sparse());

        let sparse = accessor.sparse.as_ref().unwrap();
        assert_eq!(sparse.count, 1);
        assert_eq!(sparse.indices.buffer_view, 1);
        assert_eq!(sparse.indices.component_type, ComponentType::U16);
        assert_eq!(sparse.values.buffer_view, 2);
    }
}
