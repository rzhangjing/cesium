//! Ported from `packages/engine/Source/Scene/BuildVectorGltfFromMvt.js`.

/// Builds vector glTF from MVT data.
///
/// Converts Mapbox Vector Tile data to glTF primitives.
pub struct BuildVectorGltfFromMvt {
    /// Whether the build is complete.
    pub complete: bool,
}

impl BuildVectorGltfFromMvt {
    /// Creates a new BuildVectorGltfFromMvt.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for BuildVectorGltfFromMvt {
    fn default() -> Self { Self::new() }
}
