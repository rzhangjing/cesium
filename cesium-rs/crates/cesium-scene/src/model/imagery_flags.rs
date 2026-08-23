//! Ported from `packages/engine/Source/Scene/Model/ImageryFlags.js`.

/// Flags for model imagery processing.
pub struct ImageryFlags {
    _private: (),
}

impl ImageryFlags {
    /// Creates a new ImageryFlags.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageryFlags {
    fn default() -> Self { Self::new() }
}
