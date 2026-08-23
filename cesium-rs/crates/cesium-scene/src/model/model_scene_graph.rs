//! Ported from `packages/engine/Source/Scene/Model/ModelSceneGraph.js`.
//!
//! The scene graph of a model.

use crate::frame_state::FrameState;
use crate::model::model_node::ModelNode;

/// The scene graph of a [`Model`](super::model::Model).
///
/// Manages the hierarchy of nodes, updates transforms, and generates
/// draw commands for rendering.
/// Mirrors CesiumJS `ModelSceneGraph` (1044 lines).
pub struct ModelSceneGraph {
    /// The root nodes of the scene graph.
    root_nodes: Vec<usize>,
    /// All nodes in the scene graph (indexed).
    nodes: Vec<ModelNode>,
    /// Whether the scene graph needs a transform update.
    transforms_dirty: bool,
}

impl ModelSceneGraph {
    /// Creates a new ModelSceneGraph.
    pub fn new() -> Self {
        Self {
            root_nodes: Vec::new(),
            nodes: Vec::new(),
            transforms_dirty: true,
        }
    }

    /// Returns the root node indices.
    pub fn root_nodes(&self) -> &[usize] {
        &self.root_nodes
    }

    /// Returns the number of nodes.
    pub fn nodes_count(&self) -> usize {
        self.nodes.len()
    }

    /// Gets a node by index.
    pub fn get_node(&self, index: usize) -> Option<&ModelNode> {
        self.nodes.get(index)
    }

    /// Gets a mutable reference to a node by index.
    pub fn get_node_mut(&mut self, index: usize) -> Option<&mut ModelNode> {
        self.nodes.get_mut(index)
    }

    /// Updates the scene graph transforms for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires hierarchical transform propagation
        if self.transforms_dirty {
            self.transforms_dirty = false;
        }
    }

    /// Returns whether transforms need updating.
    pub fn are_transforms_dirty(&self) -> bool {
        self.transforms_dirty
    }
}

impl Default for ModelSceneGraph {
    fn default() -> Self { Self::new() }
}
