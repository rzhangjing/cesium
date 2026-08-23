//! Ported from `packages/engine/Source/Scene/ModelSkin.js`.

/// A skin for vertex deformation in a model.
pub struct ModelSkin {
    pub name: String,
    pub joints: Vec<String>,
}

impl ModelSkin {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), joints: Vec::new() }
    }
}
