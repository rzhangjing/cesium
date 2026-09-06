//! Ported from `packages/engine/Source/Scene/GltfSpzLoader.js`.

/// Loads glTF SPZ data.
///
/// Parses and loads SPZ (compressed point cloud) data from glTF models.
pub struct GltfSpzLoader {
    /// Whether loading is complete.
    pub complete: bool,
}

impl GltfSpzLoader {
    /// Creates a new GltfSpzLoader.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for GltfSpzLoader {
    fn default() -> Self { Self::new() }
}
