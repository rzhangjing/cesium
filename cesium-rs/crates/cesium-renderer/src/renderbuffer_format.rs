//! Ported from `packages/engine/Source/Renderer/RenderbufferFormat.js`.
//!
//! Renderbuffer internal formats.

use cesium_core::webgl_constants::WebGLConstants;

/// Renderbuffer internal formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RenderbufferFormat {
    /// RGBA4.
    Rgba4 = WebGLConstants::RGBA4,
    /// RGB5-A1.
    Rgb5A1 = WebGLConstants::RGB5_A1,
    /// RGB565.
    Rgb565 = WebGLConstants::RGB565,
    /// Depth component 16.
    DepthComponent16 = WebGLConstants::DEPTH_COMPONENT16,
    /// Depth component 24.
    DepthComponent24 = WebGLConstants::DEPTH_COMPONENT24,
    /// Depth component 32F.
    DepthComponent32f = WebGLConstants::DEPTH_COMPONENT32F,
    /// Stencil index 8.
    StencilIndex8 = WebGLConstants::STENCIL_INDEX8,
    /// Depth stencil.
    DepthStencil = WebGLConstants::DEPTH_STENCIL,
    /// RGBA8.
    Rgba8 = WebGLConstants::RGBA8,
    /// SRGB8-ALPHA8.
    Srgb8Alpha8 = WebGLConstants::SRGB8_ALPHA8,
}
