//! Ported from `packages/engine/Source/Scene/Model/ModelNode.js`.
//!
//! A node in a model's scene graph.

use cesium_core::matrix4::Matrix4;

/// A node in a [`Model`](super::model::Model) scene graph.
///
/// Wraps a glTF node with its local transform and visibility.
/// Provides the ability to override the node's transform for
/// user-driven animation.
///
/// Mirrors CesiumJS `ModelNode` (126 lines).
pub struct ModelNode {
    /// The name of this node.
    pub name: String,
    /// The ID of this node.
    pub id: String,
    /// The node's current 4x4 matrix transform from local to parent.
    pub matrix: Matrix4,
    /// The node's original 4x4 matrix transform without any
    /// transformations or articulations applied.
    pub original_matrix: Matrix4,
    /// Whether this node is shown.
    pub show: bool,
    /// The index of this node in the scene graph.
    pub node_index: usize,
    /// The indices of this node's children (glTF `node.children`).
    pub children: Vec<usize>,
    /// Whether this node has been animated by user code (external animation).
    pub user_animated: bool,
}

impl ModelNode {
    /// Creates a new ModelNode.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            id: String::new(),
            matrix: Matrix4::IDENTITY,
            original_matrix: Matrix4::IDENTITY,
            show: true,
            node_index: 0,
            children: Vec::new(),
            user_animated: false,
        }
    }

    /// Sets the node's transform matrix.
    ///
    /// Setting to `None` restores the original transform and clears
    /// the `user_animated` flag.
    pub fn set_matrix(&mut self, value: Option<Matrix4>) {
        if let Some(m) = value {
            self.matrix = m;
            self.user_animated = true;
        } else {
            self.matrix = self.original_matrix;
            self.user_animated = false;
        }
    }
}
