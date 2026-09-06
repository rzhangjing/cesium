//! Ported from `packages/engine/Source/Scene/Model/NodeRenderResources.js`.
//!
//! Per-node rendering resources, extending [`ModelRenderResources`] with
//! node-specific vertex attributes and instance data.

use crate::model::model_render_resources::ModelRenderResources;

/// Per-node rendering resources.
///
/// Extends the model-level render resources with node-specific vertex
/// attributes, instance count, and per-primitive render resources.
/// Mirrors CesiumJS `NodeRenderResources` (~150 lines).
pub struct NodeRenderResources {
    /// The parent model render resources (cloned state).
    pub model_render_resources: ModelRenderResources,
    /// The runtime node index this resources belongs to.
    pub runtime_node_index: usize,
    /// Vertex attribute descriptors for this node.
    pub attributes: Vec<NodeVertexAttribute>,
    /// The next available vertex attribute index (POSITION occupies 0).
    pub attribute_index: usize,
    /// The feature ID vertex attribute set index.
    pub feature_id_vertex_attribute_set_index: usize,
    /// The number of instances for GPU instancing.
    pub instance_count: usize,
    /// Per-primitive render resources indices.
    pub primitive_render_resources_indices: Vec<usize>,
    /// Whether this node has a silhouette.
    pub has_silhouette: bool,
    /// Whether this node uses skip-level-of-detail rendering.
    pub has_skip_level_of_detail: bool,
}

/// A vertex attribute descriptor for a node.
#[derive(Debug, Clone)]
pub struct NodeVertexAttribute {
    /// The attribute name (e.g. "POSITION", "NORMAL", "TEXCOORD_0").
    pub name: String,
    /// The attribute index in the vertex layout.
    pub index: usize,
    /// The number of components per vertex (1–4).
    pub components_per_attribute: u32,
    /// The component datatype (e.g. 5126 = FLOAT).
    pub component_datatype: u32,
    /// Whether the attribute should be normalized.
    pub normalized: bool,
}

impl NodeRenderResources {
    /// Creates a new `NodeRenderResources`.
    pub fn new(runtime_node_index: usize) -> Self {
        Self {
            model_render_resources: ModelRenderResources::new(),
            runtime_node_index,
            attributes: Vec::new(),
            attribute_index: 1, // POSITION occupies index 0
            feature_id_vertex_attribute_set_index: 0,
            instance_count: 0,
            primitive_render_resources_indices: Vec::new(),
            has_silhouette: false,
            has_skip_level_of_detail: false,
        }
    }

    /// Adds a vertex attribute and returns its assigned index.
    pub fn add_attribute(&mut self, attr: NodeVertexAttribute) -> usize {
        let idx = self.attribute_index;
        self.attributes.push(attr);
        self.attribute_index += 1;
        idx
    }

    /// Returns the total number of vertex attributes (excluding POSITION).
    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
    }
}

impl Default for NodeRenderResources {
    fn default() -> Self { Self::new(0) }
}
