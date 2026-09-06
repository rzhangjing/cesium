//! Ported from `packages/engine/Source/Scene/ModelComponents.js`.
//!
//! Components of a loaded model: nodes, meshes, materials, textures,
//! animations, skins, and scene graph root indices.

use serde_json::Value;

/// Components of a loaded model.
///
/// Mirrors CesiumJS `ModelComponents` (1808 lines) — the top-level container
/// for all parsed glTF / 3D Tiles model data. The JS version defines many
/// inner types (Node, Mesh, Primitive, Material, Texture, Animation, Skin, etc.);
/// the Rust port captures the core structural fields.
///
/// DEVIATION: the full JS type hierarchy is split across multiple Rust modules
/// (ModelNode, ModelSkin, ModelAnimation, etc.); this struct aggregates them.
#[derive(Debug, Clone)]
pub struct ModelComponents {
    /// The model's nodes (flattened from the scene graph).
    pub nodes_length: usize,
    /// The model's meshes.
    pub meshes_length: usize,
    /// The model's materials.
    pub materials_length: usize,
    /// The model's textures.
    pub textures_length: usize,
    /// The model's images.
    pub images_length: usize,
    /// The model's animations.
    pub animations_length: usize,
    /// The model's skins.
    pub skins_length: usize,
    /// The default scene's root node indices.
    pub root_nodes: Vec<usize>,
    /// The model's name (from the asset or extras).
    pub name: String,
    /// Up-axis conversion (e.g., Y-up to Z-up).
    pub up_axis: i32,
    /// Extras data from the glTF asset.
    pub extras: Option<Value>,
}

impl ModelComponents {
    /// Creates a new empty `ModelComponents`.
    pub fn new() -> Self {
        Self {
            nodes_length: 0,
            meshes_length: 0,
            materials_length: 0,
            textures_length: 0,
            images_length: 0,
            animations_length: 0,
            skins_length: 0,
            root_nodes: Vec::new(),
            name: String::new(),
            up_axis: 2, // Z_UP (matches CesiumJS Axis.Z)
            extras: None,
        }
    }

    /// Returns the total number of resource entries (sum of all lengths).
    pub fn total_resources(&self) -> usize {
        self.nodes_length
            + self.meshes_length
            + self.materials_length
            + self.textures_length
            + self.images_length
            + self.animations_length
            + self.skins_length
    }
}

impl Default for ModelComponents {
    fn default() -> Self {
        Self::new()
    }
}
