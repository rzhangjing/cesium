//! Ported from `packages/engine/Source/Scene/ModelTexture.js`.

/// A texture within a model.
pub struct ModelTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

impl ModelTexture {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), width: 0, height: 0 }
    }
}
