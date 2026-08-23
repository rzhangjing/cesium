//! Ported from `packages/engine/Source/Scene/Model/ModelNode.js`.
//!
//! A node in a model's scene graph.

use cesium_core::matrix4::Matrix4;

/// A node in a [`Model`](super::model::Model) scene graph.
///
/// Wraps a glTF node with its local transform and visibility.
/// Mirrors CesiumJS `ModelNode` (200 lines).
pub struct ModelNode {
    /// The name of this node.
    pub name: String,
    /// The ID of this node.
    pub id: String,
    /// The local transform matrix.
    pub matrix: Matrix4,
    /// Whether this node is shown.
    pub show: bool,
    /// The index of this node in the scene graph.
    pub node_index: usize,
}

impl ModelNode {
    /// Creates a new ModelNode.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            id: String::new(),
            matrix: Matrix4::IDENTITY,
            show: true,
            node_index: 0,
        }
    }
}
