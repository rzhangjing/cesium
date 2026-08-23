//! Ported from `packages/engine/Source/Renderer/loadCubeMap.js`.

/// Loads a cube map texture from images.
pub struct LoadCubeMap {
    _private: (),
}

impl LoadCubeMap {
    /// Creates a new LoadCubeMap.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for LoadCubeMap {
    fn default() -> Self { Self::new() }
}
