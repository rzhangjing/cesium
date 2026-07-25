//! Renderer/RenderStateSpec.js, ClearCommandSpec.js, ComputeCommandSpec.js, PassStateSpec.js
//! → Rust integration tests

use cesium_scene::{
    RenderState, CullFace, DepthFunc, StencilOp, StencilState,
    ClearCommand, ComputeCommand, ComputeUniformValue, PassState,
};
use glam::DVec4;

// === RenderState presets ===

#[test]
fn test_render_state_default() {
    let state = RenderState::default();
    assert!(!state.cull_enabled);
    assert!(!state.depth_test_enabled);
    assert!(!state.blend_enabled);
}

#[test]
fn test_render_state_opaque() {
    let state = RenderState::opaque();
    assert!(state.cull_enabled);
    assert_eq!(state.cull_face, CullFace::Back);
    assert!(state.depth_test_enabled);
    assert!(state.depth_write_enabled);
    assert_eq!(state.depth_func, DepthFunc::Less);
    assert!(!state.blend_enabled);
}

#[test]
fn test_render_state_translucent() {
    let state = RenderState::translucent();
    assert!(state.cull_enabled);
    assert!(state.depth_test_enabled);
    assert!(!state.depth_write_enabled);
    assert!(state.blend_enabled);
}

#[test]
fn test_render_state_2d() {
    let state = RenderState::state_2d();
    assert!(!state.cull_enabled);
    assert!(!state.depth_test_enabled);
    assert!(!state.depth_write_enabled);
    assert!(state.blend_enabled);
}

// === CullFace ===

#[test]
fn test_cull_face_default() {
    assert_eq!(CullFace::default(), CullFace::Back);
}

#[test]
fn test_cull_face_variants() {
    assert_ne!(CullFace::Back, CullFace::Front);
    assert_ne!(CullFace::Front, CullFace::FrontAndBack);
}

// === DepthFunc ===

#[test]
fn test_depth_func_default() {
    assert_eq!(DepthFunc::default(), DepthFunc::Greater);
}

// === StencilState ===

#[test]
fn test_stencil_state_default() {
    let stencil = StencilState::default();
    assert!(!stencil.enabled);
    assert_eq!(stencil.front_op, StencilOp::Keep);
    assert_eq!(stencil.mask, 0xFFFFFFFF);
}

// === ClearCommand ===

#[test]
fn test_clear_command_default() {
    let cmd = ClearCommand::default();
    assert!(cmd.color.is_some());
    assert!(cmd.depth.is_some());
    assert!(cmd.stencil.is_some());
    assert_eq!(cmd.depth.unwrap(), 1.0);
}

#[test]
fn test_clear_command_color_only() {
    let cmd = ClearCommand::color_only(DVec4::new(1.0, 0.0, 0.0, 1.0));
    assert!(cmd.color.is_some());
    assert!(cmd.depth.is_none());
    assert!(cmd.stencil.is_none());
}

#[test]
fn test_clear_command_depth_only() {
    let cmd = ClearCommand::depth_only(0.5);
    assert!(cmd.color.is_none());
    assert_eq!(cmd.depth.unwrap(), 0.5);
}

#[test]
fn test_clear_command_all() {
    let cmd = ClearCommand::all(DVec4::ZERO, 1.0, 0);
    assert!(cmd.color.is_some());
    assert!(cmd.depth.is_some());
    assert!(cmd.stencil.is_some());
}

// === ComputeCommand ===

#[test]
fn test_compute_command_new() {
    let cmd = ComputeCommand::new(0, [64, 1, 1]);
    assert_eq!(cmd.shader_id, 0);
    assert_eq!(cmd.work_groups, [64, 1, 1]);
    assert!(cmd.uniform_map.is_empty());
}

#[test]
fn test_compute_command_set_uniform() {
    let mut cmd = ComputeCommand::new(1, [32, 32, 1]);
    cmd.set_uniform("u_scale", ComputeUniformValue::Float(2.0));
    cmd.set_uniform("u_offset", ComputeUniformValue::Vec3([1.0, 2.0, 3.0]));
    assert_eq!(cmd.uniform_map.len(), 2);
}

// === PassState ===

#[test]
fn test_pass_state_default() {
    let state = PassState::default();
    assert_eq!(state.viewport, [0, 0, 1920, 1080]);
    assert!(state.framebuffer_id.is_none());
}
