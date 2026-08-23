//! Ported from `packages/engine/Source/Scene/SupportedImageFormats.js`.

/// The image formats supported by the renderer.
pub struct SupportedImageFormats {
    /// Whether JPEG is supported.
    pub jpeg: bool,
    /// Whether PNG is supported.
    pub png: bool,
    /// Whether WebP is supported.
    pub webp: bool,
    /// Whether Basis Universal is supported.
    pub basis: bool,
}

impl SupportedImageFormats {
    /// Creates a new supported image formats.
    pub fn new() -> Self {
        Self { jpeg: true, png: true, webp: false, basis: false }
    }
}

impl Default for SupportedImageFormats {
    fn default() -> Self { Self::new() }
}
