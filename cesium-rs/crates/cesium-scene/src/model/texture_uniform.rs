//! Ported from `packages/engine/Source/Scene/Model/TextureUniform.js`.
//!
//! A texture uniform for shaders — holds either inline pixel data or a
//! resource reference, together with sampler state.

/// Sampler wrap mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextureWrap {
    /// Clamp to edge.
    ClampToEdge = 0,
    /// Mirrored repeat.
    MirroredRepeat = 1,
    /// Repeat.
    Repeat = 2,
}

/// Sampler min/mag filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextureFilter {
    /// Nearest neighbor.
    Nearest = 0,
    /// Linear filtering.
    Linear = 1,
    /// Nearest mipmap nearest.
    NearestMipmapNearest = 2,
    /// Linear mipmap nearest.
    LinearMipmapNearest = 3,
    /// Nearest mipmap linear.
    NearestMipmapLinear = 4,
    /// Linear mipmap linear.
    LinearMipmapLinear = 5,
}

/// A texture uniform for shaders.
///
/// Holds either inline pixel data (`typed_array` + `width`/`height`) or a
/// resource URL. Combined with sampler state (wrap/filter) to describe a
/// complete `sampler2D` uniform value.
/// Mirrors CesiumJS `TextureUniform` (~130 lines).
pub struct TextureUniform {
    /// Inline pixel data (row-major, y-up). Mutually exclusive with `resource_url`.
    pub typed_array: Option<Vec<u8>>,
    /// Image width in pixels (required when `typed_array` is set).
    pub width: Option<u32>,
    /// Image height in pixels (required when `typed_array` is set).
    pub height: Option<u32>,
    /// URL/resource pointing to the texture image. Mutually exclusive with `typed_array`.
    pub resource_url: Option<String>,
    /// The horizontal wrap mode.
    pub wrap_s: TextureWrap,
    /// The vertical wrap mode.
    pub wrap_t: TextureWrap,
    /// The minification filter.
    pub min_filter: TextureFilter,
    /// The magnification filter.
    pub mag_filter: TextureFilter,
    /// Whether this texture uniform has been marked dirty and needs re-upload.
    pub dirty: bool,
}

impl TextureUniform {
    /// Creates a new `TextureUniform` with default sampler state.
    pub fn new() -> Self {
        Self {
            typed_array: None,
            width: None,
            height: None,
            resource_url: None,
            wrap_s: TextureWrap::ClampToEdge,
            wrap_t: TextureWrap::ClampToEdge,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            dirty: true,
        }
    }

    /// Creates a `TextureUniform` from inline pixel data.
    pub fn from_typed_array(
        data: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            typed_array: Some(data),
            width: Some(width),
            height: Some(height),
            ..Self::new()
        }
    }

    /// Creates a `TextureUniform` from a resource URL.
    pub fn from_url(url: &str) -> Self {
        Self {
            resource_url: Some(url.to_string()),
            ..Self::new()
        }
    }

    /// Returns `true` if this uniform has inline pixel data.
    pub fn has_typed_array(&self) -> bool {
        self.typed_array.is_some()
    }

    /// Returns `true` if this uniform references an external resource.
    pub fn has_resource(&self) -> bool {
        self.resource_url.is_some()
    }
}

impl Default for TextureUniform {
    fn default() -> Self { Self::new() }
}
