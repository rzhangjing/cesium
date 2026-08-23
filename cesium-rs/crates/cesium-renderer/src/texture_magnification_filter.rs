//! Ported from `packages/engine/Source/Renderer/TextureMagnificationFilter.js`.
//!
//! Texture magnification filter modes.

use cesium_core::webgl_constants::WebGLConstants;

/// Texture magnification filter modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureMagnificationFilter {
    /// Nearest neighbor.
    Nearest = WebGLConstants::NEAREST,
    /// Linear interpolation.
    Linear = WebGLConstants::LINEAR,
}
