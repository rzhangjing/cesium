//! Ported from `packages/engine/Source/Scene/SupportedImageFormats.js`.

/// Image formats supported by the browser/renderer.
#[derive(Debug, Clone)]
pub struct SupportedImageFormats {
    /// Whether the browser supports WebP images.
    pub webp: bool,
    /// Whether the browser supports compressed textures required to view
    /// KTX2 + Basis Universal images.
    pub basis: bool,
}

impl SupportedImageFormats {
    /// Creates a new `SupportedImageFormats` with the given options.
    pub fn new(webp: bool, basis: bool) -> Self {
        Self { webp, basis }
    }
}

impl Default for SupportedImageFormats {
    fn default() -> Self {
        Self {
            webp: false,
            basis: false,
        }
    }
}
