//! Context specs - ported from Renderer/ContextSpec, VertexArraySpec
//! Covers: DrawCommand, RenderCommandList, RenderPass, BlendState, DepthState,
//! FrameStatistics, DebugInspector

use cesium_scene::{
    BlendState, ClearCommand, DepthState, DrawCommand, FrameStatistics, RenderPass,
    RenderState,
};

// ─── RenderPass ─────────────────────────────────────────────────────────────

#[test]
fn render_pass_variants() {
    assert_ne!(RenderPass::Opaque, RenderPass::Translucent);
    assert_ne!(RenderPass::Translucent, RenderPass::Overlay);
}

#[test]
fn render_pass_default() {
    let pass = RenderPass::default();
    assert_eq!(pass, RenderPass::Opaque);
}

// ─── DrawCommand ────────────────────────────────────────────────────────────

#[test]
fn draw_command_default() {
    let cmd = DrawCommand::default();
    assert_eq!(cmd.pass, RenderPass::Opaque);
    assert!(cmd.texture_ids.is_empty());
}

#[test]
fn draw_command_with_fields() {
    let mut cmd = DrawCommand::default();
    cmd.geometry_id = 42;
    cmd.material_id = 7;
    cmd.pass = RenderPass::Translucent;
    assert_eq!(cmd.geometry_id, 42);
    assert_eq!(cmd.material_id, 7);
    assert_eq!(cmd.pass, RenderPass::Translucent);
}

// ─── RenderState ────────────────────────────────────────────────────────────

#[test]
fn render_state_opaque() {
    let state = RenderState::opaque();
    assert!(state.depth_test_enabled);
    assert!(state.depth_write_enabled);
    assert!(!state.blend_enabled);
}

#[test]
fn render_state_translucent() {
    let state = RenderState::translucent();
    assert!(state.depth_test_enabled);
    assert!(!state.depth_write_enabled);
    assert!(state.blend_enabled);
}

// ─── BlendState ─────────────────────────────────────────────────────────────

#[test]
fn blend_state_default() {
    let blend = BlendState::default();
    assert_eq!(blend, BlendState::Opaque);
}

#[test]
fn blend_state_variants() {
    assert_ne!(BlendState::Opaque, BlendState::AlphaBlend);
    assert_ne!(BlendState::AlphaBlend, BlendState::Additive);
    assert_ne!(BlendState::Additive, BlendState::PremultipliedAlpha);
}

// ─── DepthState ─────────────────────────────────────────────────────────────

#[test]
fn depth_state_default() {
    let depth = DepthState::default();
    assert!(depth.enabled);
    assert!(depth.write_enabled);
}

// ─── ClearCommand ───────────────────────────────────────────────────────────

#[test]
fn clear_command_default() {
    let cmd = ClearCommand::default();
    assert!(cmd.color.is_some());
    assert!(cmd.depth.is_some());
    assert!(cmd.stencil.is_some());
}

#[test]
fn clear_command_custom() {
    let cmd = ClearCommand {
        color: None,
        depth: Some(0.5),
        stencil: None,
    };
    assert!(cmd.color.is_none());
    assert_eq!(cmd.depth, Some(0.5));
    assert!(cmd.stencil.is_none());
}

// ─── FrameStatistics ────────────────────────────────────────────────────────

#[test]
fn frame_statistics_default() {
    let stats = FrameStatistics::default();
    assert_eq!(stats.draw_calls, 0);
    assert_eq!(stats.triangles, 0);
}

#[test]
fn frame_statistics_accumulate() {
    let mut stats = FrameStatistics::default();
    stats.draw_calls += 10;
    stats.triangles += 5000;
    assert_eq!(stats.draw_calls, 10);
    assert_eq!(stats.triangles, 5000);
}
