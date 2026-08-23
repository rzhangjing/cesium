//! Ported from `packages/engine/Source/Scene/Model/ImageryInput.js`.

/// Input data for model imagery.
pub struct ImageryInput {
    _private: (),
}

impl ImageryInput {
    /// Creates a new ImageryInput.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageryInput {
    fn default() -> Self { Self::new() }
}
