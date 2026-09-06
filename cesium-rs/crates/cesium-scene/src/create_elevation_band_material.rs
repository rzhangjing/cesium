//! Ported from `packages/engine/Source/Scene/CreateElevationBandMaterial.js`.

/// Creates elevation band material.
///
/// Generates a material for visualizing elevation bands on terrain.
pub struct CreateElevationBandMaterial {
    /// Whether creation is complete.
    pub complete: bool,
}

impl CreateElevationBandMaterial {
    /// Creates a new CreateElevationBandMaterial.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for CreateElevationBandMaterial {
    fn default() -> Self { Self::new() }
}
