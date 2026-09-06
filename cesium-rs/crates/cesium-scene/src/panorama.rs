//! Ported from `packages/engine/Source/Scene/Panorama.js`.

/// Base panorama type.
///
/// Represents a panoramic image for street-level visualization.
pub struct Panorama {
    /// The panorama URL.
    pub url: String,
    /// Whether the panorama is loaded.
    pub loaded: bool,
}

impl Panorama {
    /// Creates a new Panorama.
    pub fn new() -> Self { Self { url: String::new(), loaded: false } }
}

impl Default for Panorama {
    fn default() -> Self { Self::new() }
}
