//! Ported from `packages/engine/Source/Scene/buildVectorGltfFromMvt.js`.

/// Builds a vector glTF from MVT data.
pub struct BuildVectorGltfFromMvt {
    _private: (),
}

impl BuildVectorGltfFromMvt {
    /// Creates a new BuildVectorGltfFromMvt.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BuildVectorGltfFromMvt {
    fn default() -> Self { Self::new() }
}
