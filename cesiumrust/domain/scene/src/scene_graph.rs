//! Scene graph node structure and traversal.
//!
//! Maps to CesiumJS `Scene/Scene.js` and `Scene/Primitive.js`

use cesium_geospatial::bounding::BoundingSphere;
use std::collections::HashMap;

/// Unique identifier for a scene node.
pub type NodeId = u64;

/// A node in the scene graph.
///
/// Maps to CesiumJS scene primitives and model nodes.
#[derive(Debug, Clone)]
pub struct SceneNode {
    /// Unique identifier.
    pub id: NodeId,

    /// Optional name for debugging.
    pub name: Option<String>,

    /// Local transform relative to parent.
    pub local_transform: glam::DMat4,

    /// World transform (computed during traversal).
    pub world_transform: glam::DMat4,

    /// Bounding volume in local space.
    pub bounding_volume: Option<BoundingSphere>,

    /// Whether this node is visible.
    pub visible: bool,

    /// Whether this node casts shadows.
    pub shadows_enabled: bool,

    /// Child node IDs.
    pub children: Vec<NodeId>,

    /// Parent node ID (None for root).
    pub parent: Option<NodeId>,

    /// Renderable content (if any).
    pub renderable: Option<RenderableContent>,

    /// User-defined metadata.
    pub metadata: HashMap<String, String>,
}

impl SceneNode {
    /// Creates a new scene node with the given ID.
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            name: None,
            local_transform: glam::DMat4::IDENTITY,
            world_transform: glam::DMat4::IDENTITY,
            bounding_volume: None,
            visible: true,
            shadows_enabled: true,
            children: Vec::new(),
            parent: None,
            renderable: None,
            metadata: HashMap::new(),
        }
    }

    /// Creates a node with a name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the local transform.
    pub fn with_transform(mut self, transform: glam::DMat4) -> Self {
        self.local_transform = transform;
        self
    }

    /// Sets the bounding volume.
    pub fn with_bounding_volume(mut self, bv: BoundingSphere) -> Self {
        self.bounding_volume = Some(bv);
        self
    }

    /// Sets the renderable content.
    pub fn with_renderable(mut self, renderable: RenderableContent) -> Self {
        self.renderable = Some(renderable);
        self
    }

    /// Computes the world-space bounding sphere.
    pub fn world_bounding_sphere(&self) -> Option<BoundingSphere> {
        self.bounding_volume.map(|bv| {
            let center = self.world_transform.transform_point3(bv.center);
            // Scale radius by the maximum scale factor
            let scale = self.world_transform.x_axis.truncate().length()
                .max(self.world_transform.y_axis.truncate().length())
                .max(self.world_transform.z_axis.truncate().length());
            BoundingSphere::new(center, bv.radius * scale)
        })
    }
}

/// Renderable content types.
#[derive(Debug, Clone)]
pub enum RenderableContent {
    /// A mesh with material.
    Mesh {
        /// Mesh asset ID.
        mesh_id: u64,
        /// Material ID.
        material_id: u64,
    },

    /// A model (glTF).
    Model {
        /// Model asset ID.
        model_id: u64,
    },

    /// A point cloud.
    PointCloud {
        /// Point cloud asset ID.
        point_cloud_id: u64,
        /// Number of points.
        point_count: usize,
    },

    /// A wireframe bounding volume (for debugging).
    DebugWireframe {
        /// Color [r, g, b, a].
        color: [f32; 4],
    },
}

/// The scene graph containing all nodes.
#[derive(Debug, Default)]
pub struct SceneGraph {
    /// All nodes in the scene.
    nodes: HashMap<NodeId, SceneNode>,

    /// Root node IDs.
    roots: Vec<NodeId>,

    /// Next available node ID.
    next_id: NodeId,
}

impl SceneGraph {
    /// Creates a new empty scene graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
            next_id: 1,
        }
    }

    /// Adds a node to the scene graph.
    ///
    /// Returns the assigned node ID.
    pub fn add_node(&mut self, mut node: SceneNode) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        node.id = id;

        if node.parent.is_none() {
            self.roots.push(id);
        }

        self.nodes.insert(id, node);
        id
    }

    /// Adds a child node to a parent.
    pub fn add_child(&mut self, parent_id: NodeId, mut child: SceneNode) -> Option<NodeId> {
        if !self.nodes.contains_key(&parent_id) {
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;
        child.id = id;
        child.parent = Some(parent_id);

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(id);
        }

        self.nodes.insert(id, child);
        Some(id)
    }

    /// Removes a node and all its descendants.
    pub fn remove_node(&mut self, id: NodeId) -> Option<SceneNode> {
        let node = self.nodes.remove(&id)?;

        // Remove from parent's children
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.children.retain(|&c| c != id);
            }
        }

        // Remove from roots if it's a root
        self.roots.retain(|&r| r != id);

        // Remove all descendants
        for child_id in &node.children {
            self.remove_node_recursive(*child_id);
        }

        Some(node)
    }

    /// Recursively removes a node and its descendants.
    fn remove_node_recursive(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.remove(&id) {
            for child_id in node.children {
                self.remove_node_recursive(child_id);
            }
        }
    }

    /// Gets a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    /// Gets a mutable node by ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    /// Returns the root node IDs.
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    /// Returns the total number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Updates world transforms for all nodes.
    pub fn update_world_transforms(&mut self) {
        let roots: Vec<NodeId> = self.roots.clone();
        for root_id in roots {
            self.update_node_transform(root_id, glam::DMat4::IDENTITY);
        }
    }

    /// Recursively updates a node's world transform.
    fn update_node_transform(&mut self, id: NodeId, parent_world: glam::DMat4) {
        let (world_transform, children) = if let Some(node) = self.nodes.get_mut(&id) {
            node.world_transform = parent_world * node.local_transform;
            (node.world_transform, node.children.clone())
        } else {
            return;
        };

        for child_id in children {
            self.update_node_transform(child_id, world_transform);
        }
    }

    /// Traverses the scene graph, calling the visitor for each visible node.
    pub fn traverse<F>(&self, mut visitor: F)
    where
        F: FnMut(&SceneNode),
    {
        for root_id in &self.roots {
            self.traverse_node(*root_id, &mut visitor);
        }
    }

    /// Recursively traverses a node and its descendants.
    fn traverse_node<F>(&self, id: NodeId, visitor: &mut F)
    where
        F: FnMut(&SceneNode),
    {
        if let Some(node) = self.nodes.get(&id) {
            if !node.visible {
                return;
            }
            visitor(node);
            for child_id in &node.children {
                self.traverse_node(*child_id, visitor);
            }
        }
    }

    /// Collects all renderable node IDs.
    pub fn collect_renderable_ids(&self) -> Vec<NodeId> {
        let mut renderables = Vec::new();
        self.traverse(|node| {
            if node.renderable.is_some() {
                renderables.push(node.id);
            }
        });
        renderables
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn test_add_node() {
        let mut scene = SceneGraph::new();
        let node = SceneNode::new(0).with_name("TestNode");
        let id = scene.add_node(node);

        assert_eq!(scene.node_count(), 1);
        assert!(scene.get(id).is_some());
        assert_eq!(scene.get(id).unwrap().name, Some("TestNode".to_string()));
    }

    #[test]
    fn test_add_child() {
        let mut scene = SceneGraph::new();
        let parent = SceneNode::new(0).with_name("Parent");
        let parent_id = scene.add_node(parent);

        let child = SceneNode::new(0).with_name("Child");
        let child_id = scene.add_child(parent_id, child).unwrap();

        assert_eq!(scene.node_count(), 2);
        assert_eq!(scene.get(child_id).unwrap().parent, Some(parent_id));
        assert!(scene.get(parent_id).unwrap().children.contains(&child_id));
    }

    #[test]
    fn test_remove_node() {
        let mut scene = SceneGraph::new();
        let parent = SceneNode::new(0);
        let parent_id = scene.add_node(parent);

        let child = SceneNode::new(0);
        let child_id = scene.add_child(parent_id, child).unwrap();

        scene.remove_node(child_id);

        assert_eq!(scene.node_count(), 1);
        assert!(scene.get(child_id).is_none());
        assert!(!scene.get(parent_id).unwrap().children.contains(&child_id));
    }

    #[test]
    fn test_remove_node_with_descendants() {
        let mut scene = SceneGraph::new();
        let root = SceneNode::new(0);
        let root_id = scene.add_node(root);

        let child = SceneNode::new(0);
        let child_id = scene.add_child(root_id, child).unwrap();

        let grandchild = SceneNode::new(0);
        let grandchild_id = scene.add_child(child_id, grandchild).unwrap();

        scene.remove_node(child_id);

        assert_eq!(scene.node_count(), 1);
        assert!(scene.get(child_id).is_none());
        assert!(scene.get(grandchild_id).is_none());
    }

    #[test]
    fn test_update_world_transforms() {
        let mut scene = SceneGraph::new();

        let parent = SceneNode::new(0)
            .with_transform(glam::DMat4::from_translation(DVec3::new(10.0, 0.0, 0.0)));
        let parent_id = scene.add_node(parent);

        let child = SceneNode::new(0)
            .with_transform(glam::DMat4::from_translation(DVec3::new(5.0, 0.0, 0.0)));
        scene.add_child(parent_id, child);

        scene.update_world_transforms();

        // Parent world = identity * local = translation(10, 0, 0)
        let parent_node = scene.get(parent_id).unwrap();
        let parent_pos = parent_node.world_transform.w_axis.truncate();
        assert!((parent_pos.x - 10.0).abs() < 1e-10);

        // Child world = parent_world * child_local = translation(15, 0, 0)
        let child_id = parent_node.children[0];
        let child_node = scene.get(child_id).unwrap();
        let child_pos = child_node.world_transform.w_axis.truncate();
        assert!((child_pos.x - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_traverse_visible_only() {
        let mut scene = SceneGraph::new();

        let mut root = SceneNode::new(0).with_name("Root");
        root.visible = true;
        let root_id = scene.add_node(root);

        let mut visible_child = SceneNode::new(0).with_name("Visible");
        visible_child.visible = true;
        scene.add_child(root_id, visible_child);

        let mut hidden_child = SceneNode::new(0).with_name("Hidden");
        hidden_child.visible = false;
        scene.add_child(root_id, hidden_child);

        let mut visited = Vec::new();
        scene.traverse(|node| {
            visited.push(node.name.clone());
        });

        assert_eq!(visited.len(), 2);
        assert!(visited.contains(&Some("Root".to_string())));
        assert!(visited.contains(&Some("Visible".to_string())));
        assert!(!visited.contains(&Some("Hidden".to_string())));
    }

    #[test]
    fn test_collect_renderables() {
        let mut scene = SceneGraph::new();

        let node_with_mesh = SceneNode::new(0).with_renderable(RenderableContent::Mesh {
            mesh_id: 1,
            material_id: 1,
        });
        scene.add_node(node_with_mesh);

        let node_without_mesh = SceneNode::new(0);
        scene.add_node(node_without_mesh);

        let renderables = scene.collect_renderable_ids();
        assert_eq!(renderables.len(), 1);
    }

    #[test]
    fn test_world_bounding_sphere() {
        let node = SceneNode::new(0)
            .with_transform(glam::DMat4::from_translation(DVec3::new(100.0, 0.0, 0.0)))
            .with_bounding_volume(BoundingSphere::new(DVec3::ZERO, 10.0));

        // Set world transform manually for this test
        let mut node = node;
        node.world_transform = node.local_transform;

        let world_bv = node.world_bounding_sphere().unwrap();
        assert!((world_bv.center.x - 100.0).abs() < 1e-10);
        assert!((world_bv.radius - 10.0).abs() < 1e-10);
    }
}
