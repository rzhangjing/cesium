//! Ported from `packages/engine/Source/Renderer/Sampler.js`.
//!
//! GPU texture sampler state.

use crate::texture_wrap::TextureWrap;
use crate::texture_magnification_filter::TextureMagnificationFilter;
use crate::texture_minification_filter::TextureMinificationFilter;

/// GPU texture sampler parameters.
#[derive(Debug, Clone)]
pub struct Sampler {
    /// Texture wrapping mode in the S (U) direction.
    pub wrap_s: TextureWrap,
    /// Texture wrapping mode in the T (V) direction.
    pub wrap_t: TextureWrap,
    /// Texture wrapping mode in the R (W) direction (3D textures).
    pub wrap_r: TextureWrap,
    /// Magnification filter.
    pub mag_filter: TextureMagnificationFilter,
    /// Minification filter.
    pub min_filter: TextureMinificationFilter,
    /// Anisotropy level.
    pub anisotropy: u32,
}

impl Sampler {
    /// Creates a new sampler with default parameters.
    pub fn new() -> Self {
        use cesium_core::webgl_constants::WebGLConstants;
        Self {
            wrap_s: TextureWrap::ClampToEdge,
            wrap_t: TextureWrap::ClampToEdge,
            wrap_r: TextureWrap::ClampToEdge,
            mag_filter: TextureMagnificationFilter::Linear,
            min_filter: TextureMinificationFilter::Linear,
            anisotropy: 1,
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}
