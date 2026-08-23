//! Ported from `packages/engine/Source/Scene/GltfLoader.js`.
//!
//! Loads glTF 2.0 assets and converts them to internal render resources.

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

/// Parsed glTF JSON structure (simplified).
pub struct GltfJson {
    /// The glTF version string.
    pub version: String,
    /// The asset metadata.
    pub asset: GltfAsset,
    /// Scene indices.
    pub scene: Option<u32>,
    /// The scenes.
    pub scenes: Vec<GltfScene>,
    /// The nodes.
    pub nodes: Vec<GltfNode>,
    /// The meshes.
    pub meshes: Vec<GltfMesh>,
    /// The materials.
    pub materials: Vec<GltfMaterial>,
    /// The accessors.
    pub accessors: Vec<GltfAccessor>,
    /// The buffer views.
    pub buffer_views: Vec<GltfBufferView>,
    /// The buffers.
    pub buffers: Vec<GltfBuffer>,
    /// The textures.
    pub textures: Vec<GltfTexture>,
    /// The images.
    pub images: Vec<GltfImage>,
    /// The animations.
    pub animations: Vec<GltfAnimation>,
    /// The skins.
    pub skins: Vec<GltfSkin>,
}

/// glTF asset metadata.
pub struct GltfAsset {
    pub version: String,
    pub generator: Option<String>,
    pub copyright: Option<String>,
}

/// A glTF scene (a collection of root nodes).
pub struct GltfScene {
    pub nodes: Vec<u32>,
    pub name: Option<String>,
}

/// A glTF node (transform + mesh reference).
pub struct GltfNode {
    pub name: Option<String>,
    pub mesh: Option<u32>,
    pub children: Vec<u32>,
    pub translation: Option<[f64; 3]>,
    pub rotation: Option<[f64; 4]>,
    pub scale: Option<[f64; 3]>,
    pub matrix: Option<[f64; 16]>,
    pub skin: Option<u32>,
}

/// A glTF mesh (a collection of primitives).
pub struct GltfMesh {
    pub name: Option<String>,
    pub primitives: Vec<GltfPrimitive>,
}

/// A glTF primitive (geometry + material reference).
pub struct GltfPrimitive {
    pub attributes: std::collections::HashMap<String, u32>,
    pub indices: Option<u32>,
    pub material: Option<u32>,
    pub mode: u32, // POINTS=0, LINES=1, LINE_LOOP=2, LINE_STRIP=3, TRIANGLES=4, etc.
}

/// A glTF material.
pub struct GltfMaterial {
    pub name: Option<String>,
    pub pbr_metallic_roughness: Option<GltfPbrMetallicRoughness>,
    pub normal_texture: Option<GltfTextureInfo>,
    pub emissive_factor: [f64; 3],
    pub alpha_mode: String,
    pub alpha_cutoff: f64,
    pub double_sided: bool,
}

/// PBR metallic-roughness material model.
pub struct GltfPbrMetallicRoughness {
    pub base_color_factor: [f64; 4],
    pub base_color_texture: Option<GltfTextureInfo>,
    pub metallic_factor: f64,
    pub roughness_factor: f64,
    pub metallic_roughness_texture: Option<GltfTextureInfo>,
}

/// A reference to a texture.
pub struct GltfTextureInfo {
    pub index: u32,
    pub tex_coord: u32,
}

/// A glTF accessor (typed view into a buffer view).
pub struct GltfAccessor {
    pub buffer_view: Option<u32>,
    pub byte_offset: u32,
    pub component_type: u32,
    pub count: u32,
    pub gl_type: String,
    pub min: Option<Vec<f64>>,
    pub max: Option<Vec<f64>>,
    pub normalized: bool,
}

/// A glTF buffer view (a slice of a buffer).
pub struct GltfBufferView {
    pub buffer: u32,
    pub byte_offset: u32,
    pub byte_length: u32,
    pub byte_stride: Option<u32>,
    pub target: Option<u32>,
}

/// A glTF buffer (raw binary data).
pub struct GltfBuffer {
    pub byte_length: u32,
    pub uri: Option<String>,
    pub data: Option<Vec<u8>>,
}

/// A glTF texture.
pub struct GltfTexture {
    pub sampler: Option<u32>,
    pub source: Option<u32>,
}

/// A glTF image.
pub struct GltfImage {
    pub uri: Option<String>,
    pub mime_type: Option<String>,
    pub buffer_view: Option<u32>,
}

/// A glTF animation.
pub struct GltfAnimation {
    pub name: Option<String>,
    pub channels: Vec<()>,
    pub samplers: Vec<()>,
}

/// A glTF skin (joint hierarchy + inverse bind matrices).
pub struct GltfSkin {
    pub name: Option<String>,
    pub joints: Vec<u32>,
    pub skeleton: Option<u32>,
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
}

impl Default for GltfLoader {
    fn default() -> Self { Self::new() }
}
