//! Ported from `packages/engine/Source/Renderer/BufferUsage.js`.
//!
//! GPU buffer usage hints.

use cesium_core::webgl_constants::WebGLConstants;

/// Buffer usage hints for GPU memory allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferUsage {
    /// Data is set once and drawn many times.
    StreamDraw = WebGLConstants::STREAM_DRAW,
    /// Data is set once and drawn many times (optimized).
    StaticDraw = WebGLConstants::STATIC_DRAW,
    /// Data is updated frequently and drawn many times.
    DynamicDraw = WebGLConstants::DYNAMIC_DRAW,
    /// Data is updated frequently and read back.
    DynamicRead = WebGLConstants::DYNAMIC_READ,
}

impl BufferUsage {
    /// Validates a buffer usage value.
    pub fn validate(usage: BufferUsage) -> bool {
        matches!(
            usage,
            BufferUsage::StreamDraw
                | BufferUsage::StaticDraw
                | BufferUsage::DynamicDraw
                | BufferUsage::DynamicRead
        )
    }
}
