//! Ported from `packages/engine/Source/Scene/SpecularEnvironmentCubeMap.js`.

/// Specular environment cube map.
///
/// Pre-filtered environment map for specular IBL reflections.
pub struct SpecularEnvironmentCubeMap {
    /// Whether the cube map is ready.
    pub ready: bool,
    /// The number of mip levels.
    pub mip_count: u32,
}

impl SpecularEnvironmentCubeMap {
    /// Creates a new SpecularEnvironmentCubeMap.
    pub fn new() -> Self { Self { ready: false, mip_count: 0 } }
}

impl Default for SpecularEnvironmentCubeMap {
    fn default() -> Self { Self::new() }
}
