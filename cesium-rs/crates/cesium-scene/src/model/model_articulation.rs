//! Ported from `packages/engine/Source/Scene/ModelArticulation.js`.

/// An articulation (hierarchical transform) within a model.
pub struct ModelArticulation {
    pub name: String,
}

impl ModelArticulation {
    pub fn new(name: &str) -> Self { Self { name: name.to_string() } }
}
