//! Ported from `packages/engine/Source/Scene/SpecularEnvironmentCubeMap.js`.

/// A cube map for specular environment lighting.
pub struct SpecularEnvironmentCubeMap {
    _private: (),
}

impl SpecularEnvironmentCubeMap {
    /// Creates a new SpecularEnvironmentCubeMap.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SpecularEnvironmentCubeMap {
    fn default() -> Self { Self::new() }
}
