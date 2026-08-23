//! Ported from `packages/engine/Source/Scene/GltfLoaderUtil.js`.

/// glTF loader utilities.
pub struct GltfLoaderUtil {
    _private: (),
}

impl GltfLoaderUtil {
    /// Creates a new GltfLoaderUtil.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfLoaderUtil {
    fn default() -> Self { Self::new() }
}
