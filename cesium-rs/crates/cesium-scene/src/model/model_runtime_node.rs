//! Ported from `packages/engine/Source/Scene/Model/ModelRuntimeNode.js`.
//!
//! A runtime node in a model, wrapping the static `ModelNode` with
//! computed runtime data: world transform, skinning index, visibility.

use cesium_core::matrix4::Matrix4;

/// A runtime node in a model.
///
/// Mirrors CesiumJS `ModelRuntimeNode`:
/// - `node_index`: index into the model's node array
/// - `local_matrix`: the node's local transform
/// - `world_matrix`: computed world transform (after hierarchy propagation)
/// - `skin_index`: index into the skin's joint array (-1 = not skinned)
/// - `show`: whether this node is visible
/// - `computed`: whether the world matrix has been computed this frame
#[derive(Debug, Clone)]
pub struct ModelRuntimeNode {
    /// Index of this node in the model's node array.
    pub node_index: usize,
    /// The node's local transform.
    pub local_matrix: Matrix4,
    /// Computed world transform (valid after hierarchy update).
    pub world_matrix: Matrix4,
    /// Index into the skin's joint array (-1 = not a skinned joint).
    pub skin_index: i32,
    /// Whether this node is visible.
    pub show: bool,
    /// Whether the world matrix has been computed for the current frame.
    pub computed: bool,
}

impl ModelRuntimeNode {
    /// Creates a new `ModelRuntimeNode` with the given index.
    pub fn new(node_index: usize) -> Self {
        Self {
            node_index,
            local_matrix: Matrix4::IDENTITY,
            world_matrix: Matrix4::IDENTITY,
            skin_index: -1,
            show: true,
            computed: false,
        }
    }

    /// Whether this node is a skinned joint.
    pub fn is_skinned(&self) -> bool {
        self.skin_index >= 0
    }

    /// Marks the node as needing recomputation (e.g., after local matrix change).
    pub fn mark_dirty(&mut self) {
        self.computed = false;
    }
}

impl Default for ModelRuntimeNode {
    fn default() -> Self {
        Self::new(0)
    }
}
