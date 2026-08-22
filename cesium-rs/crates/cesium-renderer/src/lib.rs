//! One-to-one port of `packages/engine/Source/Renderer`.
//!
//! Thin GPU abstraction layer of CesiumJS (Context, Buffer, Texture,
//! Framebuffer, ShaderProgram, DrawCommand, ...) reimplemented on top of
//! `wgpu` (NOT Bevy). Domain math stays `f64` in `cesium-core`; narrowing
//! to `f32` for GPU buffers happens only at this boundary.

#![forbid(unsafe_code)]
