//! Ported from `packages/engine/Source/Renderer/TextureWrap.js`.
//!
//! Texture wrapping modes.

use cesium_core::webgl_constants::WebGLConstants;

/// Texture wrapping modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureWrap {
    /// Clamp to edge.
    ClampToEdge = WebGLConstants::CLAMP_TO_EDGE,
    /// Repeat.
    Repeat = WebGLConstants::REPEAT,
    /// Mirrored repeat.
    MirroredRepeat = WebGLConstants::MIRRORED_REPEAT,
}
