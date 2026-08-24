//! Ported from `packages/engine/Source/Scene/Model/ModelSceneGraph.js`.
//!
//! The scene graph of a model: node hierarchy, local transforms, and
//! per-frame world-matrix propagation for the runtime nodes.

use cesium_core::matrix4::Matrix4;

use crate::frame_state::FrameState;
use crate::model::model_node::ModelNode;

/// The scene graph of a [`Model`](super::model::Model).
///
/// Manages the hierarchy of nodes, updates transforms, and provides the
/// node world matrices consumed by the draw-command generation path.
/// Mirrors CesiumJS `ModelSceneGraph` (1044 lines).
///
/// DEVIATION: the CesiumJS scene graph drives `ModelRuntimeNode`s through
/// the full `updateNodeMatrices` + skinning matrix chain; the wgpu port
/// propagates plain world matrices (node local transform composed under
/// the parent's world transform) which the Model folds into each draw
/// command's model matrix.
pub struct ModelSceneGraph {
    /// The root nodes in the scene graph.
    root_nodes: Vec<usize>,
    /// All nodes in the scene graph (indexed).
    nodes: Vec<ModelNode>,
    /// The cached world transform of each node (same indices as `nodes`).
    world_matrices: Vec<Matrix4>,
    /// Whether the scene graph needs a transform update.
    transforms_dirty: bool,
}

impl ModelSceneGraph {
    /// Creates a new ModelSceneGraph.
    pub fn new() -> Self {
        Self {
            root_nodes: Vec::new(),
            nodes: Vec::new(),
            world_matrices: Vec::new(),
            transforms_dirty: true,
        }
    }

    /// Adds a node and returns its index (mirrors the JS runtime-node
    /// construction order: nodes are added in glTF index order).
    pub fn add_node(&mut self, node: ModelNode) -> usize {
        self.nodes.push(node);
        self.world_matrices.push(Matrix4::IDENTITY);
        self.transforms_dirty = true;
        self.nodes.len() - 1
    }

    /// Sets the root node indices (mirrors the JS default-scene roots).
    pub fn set_root_nodes(&mut self, roots: Vec<usize>) {
        self.root_nodes = roots;
        self.transforms_dirty = true;
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

    /// Gets a mutable reference to a node by index (marks transforms
    /// dirty, mirroring the JS `dirty` bookkeeping on node mutation).
    pub fn get_node_mut(&mut self, index: usize) -> Option<&mut ModelNode> {
        if index < self.nodes.len() {
            self.transforms_dirty = true;
            self.nodes.get_mut(index)
        } else {
            None
        }
    }

    /// The world transform of the node at `index` (valid after
    /// [`Self::update`] propagated the hierarchy).
    pub fn world_matrix(&self, index: usize) -> Option<Matrix4> {
        self.world_matrices.get(index).copied()
    }

    /// Marks the hierarchy transforms dirty (e.g. after a node matrix
    /// update), mirroring the JS `_dirty` propagation entry points.
    pub fn dirty_transforms(&mut self) {
        self.transforms_dirty = true;
    }

    /// Updates the scene graph transforms for the current frame.
    ///
    /// Mirrors CesiumJS `ModelSceneGraph#update` → `updateNodeMatrices`:
    /// compose each node's local transform under its parent's world
    /// transform (roots compose under the identity). Hidden nodes still
    /// propagate — visibility is applied at draw-command assembly, matching
    /// the JS runtime-node traversal.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.transforms_dirty {
            return;
        }
        // Stack-based depth-first propagation (no recursion: glTF node
        // graphs are trees, and the JS traverses iteratively as well).
        let mut stack: Vec<(usize, Matrix4)> = self
            .root_nodes
            .iter()
            .map(|root| (*root, Matrix4::IDENTITY))
            .collect();
        while let Some((index, parent_world)) = stack.pop() {
            let (local, children) = match self.nodes.get(index) {
                Some(node) => (node.matrix, node.children.clone()),
                None => continue,
            };
            let world = Matrix4::multiply_new(&parent_world, &local);
            if index < self.world_matrices.len() {
                self.world_matrices[index] = world;
            }
            for child in children {
                stack.push((child as usize, world));
            }
        }
        self.transforms_dirty = false;
    }

    /// Returns whether transforms need updating.
    pub fn are_transforms_dirty(&self) -> bool {
        self.transforms_dirty
    }
}

impl Default for ModelSceneGraph {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_core::cartesian3::Cartesian3;

    fn translated_node(index: usize, x: f64, children: Vec<usize>) -> ModelNode {
        let mut node = ModelNode::new(&format!("node{index}"));
        node.node_index = index;
        node.matrix = Matrix4::from_translation_new(&Cartesian3::new(x, 0.0, 0.0));
        node.children = children;
        node
    }

    /// Mirrors the JS scene-graph transform propagation contract: a child's
    /// world matrix composes under its parent's world matrix.
    #[test]
    fn update_propagates_parent_transforms() {
        let mut graph = ModelSceneGraph::new();
        graph.add_node(translated_node(0, 1.0, vec![1]));
        graph.add_node(translated_node(1, 2.0, vec![2]));
        graph.add_node(translated_node(2, 4.0, Vec::new()));
        graph.set_root_nodes(vec![0]);

        graph.update(&FrameState::new());
        assert!(!graph.are_transforms_dirty());
        assert_eq!(graph.world_matrix(0).unwrap().elements[12], 1.0);
        assert_eq!(graph.world_matrix(1).unwrap().elements[12], 3.0);
        assert_eq!(graph.world_matrix(2).unwrap().elements[12], 7.0);
    }

    /// Mutating a node marks the hierarchy dirty again (JS `dirty`
    /// bookkeeping on `node.matrix` writes).
    #[test]
    fn node_mutation_marks_transforms_dirty() {
        let mut graph = ModelSceneGraph::new();
        graph.add_node(translated_node(0, 0.0, Vec::new()));
        graph.set_root_nodes(vec![0]);
        graph.update(&FrameState::new());
        assert!(!graph.are_transforms_dirty());

        graph.get_node_mut(0).unwrap().matrix =
            Matrix4::from_translation_new(&Cartesian3::new(5.0, 0.0, 0.0));
        assert!(graph.are_transforms_dirty());
        graph.update(&FrameState::new());
        assert_eq!(graph.world_matrix(0).unwrap().elements[12], 5.0);
    }
}
