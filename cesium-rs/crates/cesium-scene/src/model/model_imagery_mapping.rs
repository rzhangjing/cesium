//! Ported from `packages/engine/Source/Scene/Model/ModelImageryMapping.js`.

/// Mapping between model and imagery.
pub struct ModelImageryMapping {
    _private: (),
}

impl ModelImageryMapping {
    /// Creates a new ModelImageryMapping.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ModelImageryMapping {
    fn default() -> Self { Self::new() }
}
