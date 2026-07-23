//! cesium-scene: Scene graph and rendering pipeline domain models
//!
//! Maps to CesiumJS:
//! - `Scene/Scene.js`
//! - `Scene/Primitive.js`
//! - `Renderer/DrawCommand.js`
//! - `Scene/Pass.js`
//!
//! # Features
//! - Scene graph node hierarchy with transforms
//! - Frustum culling and visibility determination
//! - Draw command generation and render pass management
//! - Frame statistics tracking

pub mod scene_graph;
pub mod culling;
pub mod draw_command;

pub use scene_graph::{SceneGraph, SceneNode, NodeId, RenderableContent};
pub use culling::{
    CullingContext, CullResult, VisibilityResult,
    cull_scene, sort_front_to_back, sort_back_to_front, filter_visible,
};
pub use draw_command::{
    DrawCommand, RenderCommandList, RenderPass, BlendState, DepthState, FrameStatistics,
};
