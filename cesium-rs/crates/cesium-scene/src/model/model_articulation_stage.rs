//! Ported from `packages/engine/Source/Scene/ModelArticulationStage.js`.

/// A stage within a model articulation.
pub struct ModelArticulationStage {
    pub name: String,
}

impl ModelArticulationStage {
    pub fn new(name: &str) -> Self { Self { name: name.to_string() } }
}
