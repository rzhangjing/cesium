//! Ported from `packages/engine/Source/Scene/ContentMetadata.js`.

/// Content metadata.
pub struct ContentMetadata {
    _private: (),
}

impl ContentMetadata {
    /// Creates a new ContentMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ContentMetadata {
    fn default() -> Self { Self::new() }
}
