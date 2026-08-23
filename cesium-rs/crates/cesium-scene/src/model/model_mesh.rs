//! Ported from `packages/engine/Source/Scene/Model/ModelMesh.js`.
//!
//! A mesh within a model.

/// A mesh within a [`Model`](super::model::Model).
///
/// Contains one or more primitives that share the same transform.
/// Mirrors CesiumJS `ModelMesh` (121 lines).
pub struct ModelMesh {
    /// The name of this mesh.
    pub name: String,
    /// The ID of this mesh.
    pub id: String,
    /// The number of primitives in this mesh.
    pub primitive_count: usize,
    /// The indices of primitives in this mesh.
    pub primitive_indices: Vec<usize>,
}

impl ModelMesh {
    /// Creates a new ModelMesh.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            id: String::new(),
            primitive_count: 0,
            primitive_indices: Vec::new(),
        }
    }
}
