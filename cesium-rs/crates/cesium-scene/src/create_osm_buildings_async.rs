//! Ported from `packages/engine/Source/Scene/CreateOSMBuildingsAsync.js`.

/// Creates OSM Buildings asynchronously.
///
/// Factory for OpenStreetMap 3D buildings tileset.
pub struct CreateOsmBuildingsAsync {
    /// Whether creation is complete.
    pub complete: bool,
}

impl CreateOsmBuildingsAsync {
    /// Creates a new CreateOsmBuildingsAsync.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for CreateOsmBuildingsAsync {
    fn default() -> Self { Self::new() }
}
