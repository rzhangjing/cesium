//! Ported from `packages/engine/Source/Scene/Model/NodeRenderResources.js`.

/// Rendering resources for a model node.
pub struct NodeRenderResources {
    _private: (),
}

impl NodeRenderResources {
    /// Creates a new NodeRenderResources.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for NodeRenderResources {
    fn default() -> Self { Self::new() }
}
