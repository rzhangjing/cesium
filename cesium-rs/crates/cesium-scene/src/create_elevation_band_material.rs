//! Ported from `packages/engine/Source/Scene/createElevationBandMaterial.js`.

/// Creates an elevation band material.
pub struct CreateElevationBandMaterial {
    _private: (),
}

impl CreateElevationBandMaterial {
    /// Creates a new CreateElevationBandMaterial.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateElevationBandMaterial {
    fn default() -> Self { Self::new() }
}
