//! One-to-one port of `packages/engine/Source/Renderer`.
//!
//! Thin GPU abstraction layer of CesiumJS (Context, Buffer, Texture,
//! Framebuffer, ShaderProgram, DrawCommand, ...) reimplemented on top of
//! `wgpu` (NOT Bevy). Domain math stays `f64` in `cesium-core`; narrowing
//! to `f32` for GPU buffers happens only at this boundary.

#![forbid(unsafe_code)]
#![allow(dead_code)]

pub mod automatic_uniforms;
pub mod buffer;
pub mod buffer_usage;
pub mod clear_command;
pub mod compute_command;
pub mod compute_engine;
pub mod context;
pub mod context_limits;
pub mod create_uniform;
pub mod create_uniform_array;
pub mod cube_map;
pub mod cube_map_face;
pub mod demodernize_shader;
pub mod draw_command;
pub mod framebuffer;
pub mod framebuffer_manager;
pub mod freeze_render_state;
pub mod load_cube_map;
pub mod mipmap_hint;
pub mod multisample_framebuffer;
pub mod pass;
pub mod pass_state;
pub mod pick_id;
pub mod pixel_datatype;
pub mod render_state;
pub mod renderbuffer;
pub mod renderbuffer_format;
pub mod sampler;
pub mod shader_builder;
pub mod shader_cache;
pub mod shader_destination;
pub mod shader_function;
pub mod shader_program;
pub mod shader_source;
pub mod shader_struct;
pub mod shared_context;
pub mod sync;
pub mod texture;
pub mod texture3d;
pub mod texture_atlas;
pub mod texture_cache;
pub mod texture_magnification_filter;
pub mod texture_minification_filter;
pub mod texture_wrap;
pub mod uniform_state;
pub mod vertex_array;
pub mod vertex_array_facade;
