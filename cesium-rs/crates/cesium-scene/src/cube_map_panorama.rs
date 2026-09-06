//! Ported from `packages/engine/Source/Scene/CubeMapPanorama.js`.

/// Cube map panorama.
///
/// Represents a panorama as six cube face images.
pub struct CubeMapPanorama {
    /// The six face URLs (px, nx, py, ny, pz, nz).
    pub face_urls: Vec<String>,
    /// Whether the panorama is loaded.
    pub loaded: bool,
}

impl CubeMapPanorama {
    /// Creates a new CubeMapPanorama.
    pub fn new() -> Self { Self { face_urls: Vec::new(), loaded: false } }
}

impl Default for CubeMapPanorama {
    fn default() -> Self { Self::new() }
}
