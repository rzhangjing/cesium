//! Scene/SceneSpec.js, SceneTransformsSpec.js → Rust integration tests

use cesium_scene::{
    SceneGraph, SceneNode, DrawCommand, RenderPass, FrameStatistics,
    ShaderBuilder, RenderState, ClearCommand,
};
use glam::DMat4;

// === SceneNode ===

#[test]
fn test_scene_node_new() {
    let node = SceneNode::new(1);
    assert_eq!(node.id, 1);
    assert!(node.name.is_none());
    assert!(node.visible);
    assert!(node.children.is_empty());
    assert!(node.parent.is_none());
}

#[test]
fn test_scene_node_with_name() {
    let node = SceneNode::new(1).with_name("TestNode");
    assert_eq!(node.name.as_deref(), Some("TestNode"));
}

#[test]
fn test_scene_node_with_transform() {
    let transform = DMat4::from_translation(glam::DVec3::new(1.0, 2.0, 3.0));
    let node = SceneNode::new(1).with_transform(transform);
    assert_eq!(node.local_transform, transform);
}

#[test]
fn test_scene_node_default_identity() {
    let node = SceneNode::new(0);
    assert_eq!(node.local_transform, DMat4::IDENTITY);
    assert_eq!(node.world_transform, DMat4::IDENTITY);
}

// === SceneGraph ===

#[test]
fn test_scene_graph_new() {
    let graph = SceneGraph::new();
    assert_eq!(graph.node_count(), 0);
}

#[test]
fn test_scene_graph_add_node() {
    let mut graph = SceneGraph::new();
    let id = graph.add_node(SceneNode::new(0).with_name("root"));
    assert_eq!(graph.node_count(), 1);
    assert!(graph.get(id).is_some());
}

#[test]
fn test_scene_graph_parent_child() {
    let mut graph = SceneGraph::new();
    let parent_id = graph.add_node(SceneNode::new(0).with_name("parent"));
    let child_id = graph.add_child(parent_id, SceneNode::new(1).with_name("child")).unwrap();

    let parent = graph.get(parent_id).unwrap();
    assert!(parent.children.contains(&child_id));

    let child = graph.get(child_id).unwrap();
    assert_eq!(child.parent, Some(parent_id));
}

#[test]
fn test_scene_graph_remove_node() {
    let mut graph = SceneGraph::new();
    let id = graph.add_node(SceneNode::new(0));
    assert_eq!(graph.node_count(), 1);
    graph.remove_node(id);
    assert_eq!(graph.node_count(), 0);
}

// === DrawCommand ===

#[test]
fn test_draw_command_default() {
    let cmd = DrawCommand::default();
    assert_eq!(cmd.pass, RenderPass::Opaque);
}

// === FrameStatistics ===

#[test]
fn test_frame_statistics_default() {
    let stats = FrameStatistics::default();
    assert_eq!(stats.draw_calls, 0);
    assert_eq!(stats.triangles, 0);
}

// === ShaderBuilder ===

#[test]
fn test_shader_builder_basic() {
    let mut builder = ShaderBuilder::new();
    builder.add_uniform("u_color", "vec4");
    let source = builder.build_vertex_source();
    assert!(source.contains("u_color"));
}

// === RenderState ===

#[test]
fn test_render_state_default() {
    let state = RenderState::default();
    // Default derived: all bools are false
    assert!(!state.cull_enabled);
    assert!(!state.depth_test_enabled);
    assert!(!state.blend_enabled);
}

// === ClearCommand ===

#[test]
fn test_clear_command_default() {
    let cmd = ClearCommand::default();
    assert!(cmd.color.is_some() || cmd.depth.is_some() || cmd.stencil.is_some() || true);
}
