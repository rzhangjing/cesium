//! Ported from `packages/engine/Source/Renderer/ContextLimits.js`.
//!
//! GPU capability limits queried from the context.

use std::sync::atomic::{AtomicU32, Ordering};

static MAX_TEXTURE_SIZE: AtomicU32 = AtomicU32::new(0);
static MAX_CUBE_MAP_TEXTURE_SIZE: AtomicU32 = AtomicU32::new(0);
static MAX_RENDERBUFFER_SIZE: AtomicU32 = AtomicU32::new(0);
static MAX_VERTEX_ATTRIBS: AtomicU32 = AtomicU32::new(0);
static MAX_VERTEX_UNIFORM_VECTORS: AtomicU32 = AtomicU32::new(0);
static MAX_FRAGMENT_UNIFORM_VECTORS: AtomicU32 = AtomicU32::new(0);
static MAX_VARYING_VECTORS: AtomicU32 = AtomicU32::new(0);
static MAX_TEXTURE_IMAGE_UNITS: AtomicU32 = AtomicU32::new(0);
static MAX_COMBINED_TEXTURE_IMAGE_UNITS: AtomicU32 = AtomicU32::new(0);
static MAX_VERTEX_TEXTURE_IMAGE_UNITS: AtomicU32 = AtomicU32::new(0);
static MAX_DRAW_BUFFERS: AtomicU32 = AtomicU32::new(0);
static MAX_SAMPLES: AtomicU32 = AtomicU32::new(0);

/// GPU capability limits queried from the rendering context.
/// All values are set once during context initialization.
pub struct ContextLimits;

impl ContextLimits {
    /// Maximum texture size in texels.
    pub fn max_texture_size() -> u32 { MAX_TEXTURE_SIZE.load(Ordering::Relaxed) }
    /// Maximum cube map texture size.
    pub fn max_cube_map_texture_size() -> u32 { MAX_CUBE_MAP_TEXTURE_SIZE.load(Ordering::Relaxed) }
    /// Maximum renderbuffer size.
    pub fn max_renderbuffer_size() -> u32 { MAX_RENDERBUFFER_SIZE.load(Ordering::Relaxed) }
    /// Maximum vertex attributes.
    pub fn max_vertex_attribs() -> u32 { MAX_VERTEX_ATTRIBS.load(Ordering::Relaxed) }
    /// Maximum draw buffers.
    pub fn max_draw_buffers() -> u32 { MAX_DRAW_BUFFERS.load(Ordering::Relaxed) }
    /// Maximum samples for multisampling.
    pub fn max_samples() -> u32 { MAX_SAMPLES.load(Ordering::Relaxed) }

    /// Sets the maximum texture size (called during context init).
    pub fn set_max_texture_size(v: u32) { MAX_TEXTURE_SIZE.store(v, Ordering::Relaxed); }
    /// Sets the maximum cube map texture size.
    pub fn set_max_cube_map_texture_size(v: u32) { MAX_CUBE_MAP_TEXTURE_SIZE.store(v, Ordering::Relaxed); }
    /// Sets the maximum renderbuffer size.
    pub fn set_max_renderbuffer_size(v: u32) { MAX_RENDERBUFFER_SIZE.store(v, Ordering::Relaxed); }
    /// Sets the maximum vertex attribs.
    pub fn set_max_vertex_attribs(v: u32) { MAX_VERTEX_ATTRIBS.store(v, Ordering::Relaxed); }
    /// Sets the maximum draw buffers.
    pub fn set_max_draw_buffers(v: u32) { MAX_DRAW_BUFFERS.store(v, Ordering::Relaxed); }
    /// Sets the maximum samples.
    pub fn set_max_samples(v: u32) { MAX_SAMPLES.store(v, Ordering::Relaxed); }
}
