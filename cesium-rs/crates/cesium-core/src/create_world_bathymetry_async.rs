//! Ported from `packages/engine/Source/Core/createWorldBathymetryAsync.js`.

/// Creates world bathymetry data asynchronously.
pub struct CreateWorldBathymetryAsync {
    _private: (),
}

impl CreateWorldBathymetryAsync {
    /// Creates a new CreateWorldBathymetryAsync.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateWorldBathymetryAsync {
    fn default() -> Self { Self::new() }
}
