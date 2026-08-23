//! Ported from `packages/engine/Source/Renderer/TextureMinificationFilter.js`.
//!
//! Texture minification filter modes.

use cesium_core::webgl_constants::WebGLConstants;

/// Texture minification filter modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureMinificationFilter {
    /// Nearest neighbor.
    Nearest = WebGLConstants::NEAREST,
    /// Linear interpolation.
    Linear = WebGLConstants::LINEAR,
    /// Nearest mipmap nearest.
    NearestMipmapNearest = WebGLConstants::NEAREST_MIPMAP_NEAREST,
    /// Linear mipmap nearest.
    LinearMipmapNearest = WebGLConstants::LINEAR_MIPMAP_NEAREST,
    /// Nearest mipmap linear.
    NearestMipmapLinear = WebGLConstants::NEAREST_MIPMAP_LINEAR,
    /// Linear mipmap linear.
    LinearMipmapLinear = WebGLConstants::LINEAR_MIPMAP_LINEAR,
}
