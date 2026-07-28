//! DrawCommand / RenderCommandList / FrameStatistics specs
//! Ported from CesiumJS Renderer/DrawCommand.js + Scene/Pass.js
//!
//! A-class tests: DrawCommand construction/builder, RenderPass ordering,
//! RenderCommandList push/sort/clear, BlendState/DepthState, FrameStatistics merge/reset

use cesium_scene::{
    BlendState, DepthState, DrawCommand, FrameStatistics, RenderCommandList, RenderPass,
};
use glam::DMat4;

// ─── DrawCommand ───────────────────────────────────────────────────────────────

#[test]
fn draw_command_defaults() {
    let cmd = DrawCommand::default();
    assert_eq!(cmd.pass, RenderPass::Opaque);
    assert_eq!(cmd.model_matrix, DMat4::IDENTITY);
    assert_eq!(cmd.geometry_id, 0);
    assert_eq!(cmd.material_id, 0);
    assert!(cmd.texture_ids.is_empty());
    assert_eq!(cmd.blend_state, BlendState::Opaque);
    assert!(cmd.depth_state.enabled);
    assert!(cmd.depth_state.write_enabled);
    assert!(cmd.cull_face);
    assert_eq!(cmd.sort_key, 0.0);
    assert_eq!(cmd.instance_count, 1);
    assert!(cmd.casts_shadows);
    assert!(cmd.receives_shadows);
    assert_eq!(cmd.pick_id, None);
    assert!(!cmd.is_transparent());
}

#[test]
fn draw_command_new_with_ids() {
    let cmd = DrawCommand::new(7, 13);
    assert_eq!(cmd.geometry_id, 7);
    assert_eq!(cmd.material_id, 13);
    assert_eq!(cmd.pass, RenderPass::Opaque);
}

#[test]
fn draw_command_builder_chain() {
    let cmd = DrawCommand::new(1, 2)
        .with_pass(RenderPass::Translucent)
        .with_blend_state(BlendState::AlphaBlend)
        .with_sort_key(42.5)
        .with_pick_id(99)
        .with_model_matrix(DMat4::from_translation(glam::DVec3::new(1.0, 2.0, 3.0)));

    assert_eq!(cmd.pass, RenderPass::Translucent);
    assert_eq!(cmd.blend_state, BlendState::AlphaBlend);
    assert_eq!(cmd.sort_key, 42.5);
    assert_eq!(cmd.pick_id, Some(99));
    assert!(cmd.is_transparent());
    assert_eq!(cmd.model_matrix.w_axis.x, 1.0);
}

#[test]
fn draw_command_is_transparent_variants() {
    assert!(!DrawCommand::default().is_transparent());
    assert!(DrawCommand::new(0, 0)
        .with_blend_state(BlendState::AlphaBlend)
        .is_transparent());
    assert!(DrawCommand::new(0, 0)
        .with_blend_state(BlendState::Additive)
        .is_transparent());
    assert!(DrawCommand::new(0, 0)
        .with_blend_state(BlendState::PremultipliedAlpha)
        .is_transparent());
}

// ─── RenderPass ordering ───────────────────────────────────────────────────────

#[test]
fn render_pass_ordering() {
    assert!(RenderPass::Environment < RenderPass::Cesium3DTile);
    assert!(RenderPass::Cesium3DTile < RenderPass::Opaque);
    assert!(RenderPass::Opaque < RenderPass::Translucent);
    assert!(RenderPass::Translucent < RenderPass::Overlay);
}

#[test]
fn render_pass_default_is_opaque() {
    assert_eq!(RenderPass::default(), RenderPass::Opaque);
}

// ─── RenderCommandList ─────────────────────────────────────────────────────────

#[test]
fn command_list_push_and_query() {
    let mut list = RenderCommandList::new();
    assert!(list.is_empty());
    assert_eq!(list.len(), 0);

    list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Opaque));
    list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Translucent));
    list.push(DrawCommand::new(3, 3).with_pass(RenderPass::Opaque));
    list.push(DrawCommand::new(4, 4).with_pass(RenderPass::Environment));

    assert!(!list.is_empty());
    assert_eq!(list.len(), 4);
    assert_eq!(list.commands_for_pass(RenderPass::Opaque).len(), 2);
    assert_eq!(list.commands_for_pass(RenderPass::Translucent).len(), 1);
    assert_eq!(list.commands_for_pass(RenderPass::Environment).len(), 1);
    assert_eq!(list.commands_for_pass(RenderPass::Overlay).len(), 0);
}

#[test]
fn command_list_sort_opaque_front_to_back() {
    let mut list = RenderCommandList::new();
    list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Opaque).with_sort_key(300.0));
    list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Opaque).with_sort_key(100.0));
    list.push(DrawCommand::new(3, 3).with_pass(RenderPass::Opaque).with_sort_key(200.0));

    list.sort();

    let opaque = list.commands_for_pass(RenderPass::Opaque);
    assert_eq!(opaque[0].geometry_id, 2); // 100
    assert_eq!(opaque[1].geometry_id, 3); // 200
    assert_eq!(opaque[2].geometry_id, 1); // 300
}

#[test]
fn command_list_sort_translucent_back_to_front() {
    let mut list = RenderCommandList::new();
    list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Translucent).with_sort_key(100.0));
    list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Translucent).with_sort_key(300.0));
    list.push(DrawCommand::new(3, 3).with_pass(RenderPass::Translucent).with_sort_key(200.0));

    list.sort();

    let translucent = list.commands_for_pass(RenderPass::Translucent);
    assert_eq!(translucent[0].geometry_id, 2); // 300 (farthest)
    assert_eq!(translucent[1].geometry_id, 3); // 200
    assert_eq!(translucent[2].geometry_id, 1); // 100 (nearest)
}

#[test]
fn command_list_clear() {
    let mut list = RenderCommandList::new();
    list.push(DrawCommand::new(1, 1));
    list.push(DrawCommand::new(2, 2));
    assert_eq!(list.len(), 2);

    list.clear();
    assert!(list.is_empty());
    assert_eq!(list.len(), 0);
}

// ─── DepthState ────────────────────────────────────────────────────────────────

#[test]
fn depth_state_default() {
    let ds = DepthState::default();
    assert!(ds.enabled);
    assert!(ds.write_enabled);
}

// ─── FrameStatistics ───────────────────────────────────────────────────────────

#[test]
fn frame_statistics_merge() {
    let mut stats = FrameStatistics {
        draw_calls: 100,
        triangles: 50000,
        vertices: 25000,
        texture_binds: 10,
        shader_switches: 5,
        culled_objects: 20,
        frame_time_ms: 16.0,
    };

    let other = FrameStatistics {
        draw_calls: 50,
        triangles: 25000,
        vertices: 12000,
        texture_binds: 5,
        shader_switches: 3,
        culled_objects: 10,
        frame_time_ms: 8.0,
    };

    stats.merge(&other);

    assert_eq!(stats.draw_calls, 150);
    assert_eq!(stats.triangles, 75000);
    assert_eq!(stats.vertices, 37000);
    assert_eq!(stats.texture_binds, 15);
    assert_eq!(stats.shader_switches, 8);
    assert_eq!(stats.culled_objects, 30);
    // frame_time_ms is NOT merged (it's per-frame)
    assert_eq!(stats.frame_time_ms, 16.0);
}

#[test]
fn frame_statistics_reset() {
    let mut stats = FrameStatistics {
        draw_calls: 100,
        triangles: 50000,
        frame_time_ms: 16.0,
        ..Default::default()
    };

    stats.reset();

    assert_eq!(stats.draw_calls, 0);
    assert_eq!(stats.triangles, 0);
    assert_eq!(stats.frame_time_ms, 0.0);
}
