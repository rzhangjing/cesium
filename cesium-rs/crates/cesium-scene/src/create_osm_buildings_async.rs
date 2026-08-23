//! Ported from `packages/engine/Source/Scene/createOsmBuildingsAsync.js`.

/// Creates OSM buildings asynchronously.
pub struct CreateOsmBuildingsAsync {
    _private: (),
}

impl CreateOsmBuildingsAsync {
    /// Creates a new CreateOsmBuildingsAsync.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateOsmBuildingsAsync {
    fn default() -> Self { Self::new() }
}
