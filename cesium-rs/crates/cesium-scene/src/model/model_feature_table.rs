//! Ported from `packages/engine/Source/Scene/ModelFeatureTable.js`.

/// A feature table within a 3D Tiles batch.
pub struct ModelFeatureTable {
    pub features_length: u32,
}

impl ModelFeatureTable {
    pub fn new() -> Self { Self { features_length: 0 } }
}

impl Default for ModelFeatureTable {
    fn default() -> Self { Self::new() }
}
