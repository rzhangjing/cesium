//! Ported from `packages/engine/Source/Scene/Model/ImageryConfiguration.js`.

/// Configuration for model imagery.
pub struct ImageryConfiguration {
    _private: (),
}

impl ImageryConfiguration {
    /// Creates a new ImageryConfiguration.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageryConfiguration {
    fn default() -> Self { Self::new() }
}
