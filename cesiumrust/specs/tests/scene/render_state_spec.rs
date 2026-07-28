//! RenderState / ClearCommand / ComputeCommand / Texture / Framebuffer / TextureAtlas specs
//! Ported from CesiumJS Renderer/RenderState.js + ClearCommand.js + Texture.js + TextureAtlas.js
//!
//! A-class tests: RenderState presets, ClearCommand variants, ComputeCommand uniforms,
//! PassState defaults, Texture construction/mipmaps, Framebuffer attachments,
//! TextureAtlas packing, GpuBuffer

use cesium_scene::{
    BufferUsage, ClearCommand, ComputeCommand, ComputeUniformValue, CullFace, DepthFunc,
    Framebuffer, GpuBuffer, PassState, PixelDatatype, PixelFormat, PolygonOffsetState,
    RenderState, ScissorState, StencilOp, StencilState, Texture, TextureAtlas, TextureFilter,
    TextureWrap,
};
use glam::DVec4;

// ─── RenderState presets ───────────────────────────────────────────────────────

#[test]
fn render_state_opaque_preset() {
    let rs = RenderState::opaque();
    assert!(rs.cull_enabled);
    assert_eq!(rs.cull_face, CullFace::Back);
    assert!(rs.depth_test_enabled);
    assert!(rs.depth_write_enabled);
    assert_eq!(rs.depth_func, DepthFunc::Less);
    assert!(!rs.blend_enabled);
}

#[test]
fn render_state_translucent_preset() {
    let rs = RenderState::translucent();
    assert!(rs.cull_enabled);
    assert!(rs.depth_test_enabled);
    assert!(!rs.depth_write_enabled); // No depth write for transparency
    assert!(rs.blend_enabled);
}

#[test]
fn render_state_2d_preset() {
    let rs = RenderState::state_2d();
    assert!(!rs.cull_enabled);
    assert!(!rs.depth_test_enabled);
    assert!(!rs.depth_write_enabled);
    assert!(rs.blend_enabled);
}

#[test]
fn render_state_default() {
    let rs = RenderState::default();
    assert!(!rs.cull_enabled);
    assert_eq!(rs.cull_face, CullFace::Back);
    assert!(!rs.depth_test_enabled);
    assert!(!rs.depth_write_enabled);
    assert_eq!(rs.depth_func, DepthFunc::Greater);
    assert!(!rs.blend_enabled);
    assert_eq!(rs.line_width, 0.0);
}

// ─── StencilState / PolygonOffset / Scissor ────────────────────────────────────

#[test]
fn stencil_state_default() {
    let s = StencilState::default();
    assert!(!s.enabled);
    assert_eq!(s.front_op, StencilOp::Keep);
    assert_eq!(s.back_op, StencilOp::Keep);
    assert_eq!(s.ref_value, 0);
    assert_eq!(s.mask, 0xFFFFFFFF);
}

#[test]
fn polygon_offset_default() {
    let po = PolygonOffsetState::default();
    assert!(!po.enabled);
    assert_eq!(po.factor, 0.0);
    assert_eq!(po.units, 0.0);
}

#[test]
fn scissor_state_default() {
    let sc = ScissorState::default();
    assert!(!sc.enabled);
    assert_eq!(sc.x, 0);
    assert_eq!(sc.y, 0);
    assert_eq!(sc.width, 0);
    assert_eq!(sc.height, 0);
}

// ─── ClearCommand ──────────────────────────────────────────────────────────────

#[test]
fn clear_command_default() {
    let cc = ClearCommand::default();
    assert_eq!(cc.color, Some(DVec4::new(0.0, 0.0, 0.0, 1.0)));
    assert_eq!(cc.depth, Some(1.0));
    assert_eq!(cc.stencil, Some(0));
}

#[test]
fn clear_command_color_only() {
    let cc = ClearCommand::color_only(DVec4::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(cc.color, Some(DVec4::new(1.0, 0.0, 0.0, 1.0)));
    assert!(cc.depth.is_none());
    assert!(cc.stencil.is_none());
}

#[test]
fn clear_command_depth_only() {
    let cc = ClearCommand::depth_only(0.5);
    assert!(cc.color.is_none());
    assert_eq!(cc.depth, Some(0.5));
    assert!(cc.stencil.is_none());
}

#[test]
fn clear_command_all() {
    let cc = ClearCommand::all(DVec4::new(0.0, 0.0, 1.0, 1.0), 1.0, 255);
    assert!(cc.color.is_some());
    assert_eq!(cc.depth, Some(1.0));
    assert_eq!(cc.stencil, Some(255));
}

// ─── ComputeCommand ────────────────────────────────────────────────────────────

#[test]
fn compute_command_new_and_uniforms() {
    let mut cmd = ComputeCommand::new(5, [64, 64, 1]);
    assert_eq!(cmd.shader_id, 5);
    assert_eq!(cmd.work_groups, [64, 64, 1]);
    assert!(cmd.uniform_map.is_empty());

    cmd.set_uniform("u_scale", ComputeUniformValue::Float(2.0));
    cmd.set_uniform("u_offset", ComputeUniformValue::Vec3([1.0, 2.0, 3.0]));
    assert_eq!(cmd.uniform_map.len(), 2);
    assert_eq!(cmd.uniform_map[0].0, "u_scale");
    assert_eq!(cmd.uniform_map[1].0, "u_offset");
}

// ─── PassState ─────────────────────────────────────────────────────────────────

#[test]
fn pass_state_default() {
    let ps = PassState::default();
    assert_eq!(ps.viewport, [0, 0, 1920, 1080]);
    assert!(ps.framebuffer_id.is_none());
}

// ─── Texture ───────────────────────────────────────────────────────────────────

#[test]
fn texture_new_defaults() {
    let tex = Texture::new(1, 512, 256);
    assert_eq!(tex.id, 1);
    assert_eq!(tex.width, 512);
    assert_eq!(tex.height, 256);
    assert_eq!(tex.format, PixelFormat::Rgba);
    assert_eq!(tex.datatype, PixelDatatype::UnsignedByte);
    assert_eq!(tex.min_filter, TextureFilter::Linear);
    assert_eq!(tex.mag_filter, TextureFilter::Linear);
    assert_eq!(tex.wrap_s, TextureWrap::ClampToEdge);
    assert_eq!(tex.wrap_t, TextureWrap::ClampToEdge);
    assert!(!tex.generate_mipmaps);
}

#[test]
fn texture_with_mipmaps() {
    let tex = Texture::new(0, 256, 256).with_mipmaps();
    assert!(tex.generate_mipmaps);
    assert_eq!(tex.min_filter, TextureFilter::LinearMipmapLinear);
}

// ─── Framebuffer ───────────────────────────────────────────────────────────────

#[test]
fn framebuffer_attach() {
    let mut fb = Framebuffer::new(0, 1024, 768);
    assert_eq!(fb.width, 1024);
    assert_eq!(fb.height, 768);
    assert!(fb.color_textures.is_empty());
    assert!(fb.depth_texture.is_none());

    fb.attach_color(10);
    fb.attach_color(11);
    fb.attach_depth(20);

    assert_eq!(fb.color_textures.len(), 2);
    assert_eq!(fb.color_textures[0], 10);
    assert_eq!(fb.color_textures[1], 11);
    assert_eq!(fb.depth_texture, Some(20));
}

// ─── TextureAtlas ──────────────────────────────────────────────────────────────

#[test]
fn texture_atlas_add_entries() {
    let mut atlas = TextureAtlas::new(0, 512, 512, 2);
    assert_eq!(atlas.entry_count(), 0);

    let e1 = atlas.add_entry(64, 64).unwrap();
    assert_eq!(e1.x, 2); // padding
    assert_eq!(e1.y, 2);
    assert_eq!(e1.width, 64);
    assert_eq!(e1.height, 64);

    let e2 = atlas.add_entry(64, 64).unwrap();
    assert_eq!(atlas.entry_count(), 2);
    // Entries should not overlap
    assert!(e2.x >= e1.x + e1.width + 2 || e2.y >= e1.y + e1.height + 2);
}

#[test]
fn texture_atlas_full_returns_none() {
    let mut atlas = TextureAtlas::new(0, 64, 64, 2);
    // Try to add an entry larger than atlas
    let result = atlas.add_entry(100, 100);
    assert!(result.is_none());
}

// ─── GpuBuffer ─────────────────────────────────────────────────────────────────

#[test]
fn gpu_buffer_new() {
    let buf = GpuBuffer::new(0, 4096, BufferUsage::DynamicDraw);
    assert_eq!(buf.id, 0);
    assert_eq!(buf.size_in_bytes, 4096);
    assert_eq!(buf.usage, BufferUsage::DynamicDraw);
}
