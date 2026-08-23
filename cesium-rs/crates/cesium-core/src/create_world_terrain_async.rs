//! Ported from `packages/engine/Source/Core/createWorldTerrainAsync.js`.

/// Creates world terrain data asynchronously.
pub struct CreateWorldTerrainAsync {
    _private: (),
}

impl CreateWorldTerrainAsync {
    /// Creates a new CreateWorldTerrainAsync.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateWorldTerrainAsync {
    fn default() -> Self { Self::new() }
}
