//! Ported from `packages/engine/Source/Scene/findContentMetadata.js`.

/// Finds content metadata.
pub struct FindContentMetadata {
    _private: (),
}

impl FindContentMetadata {
    /// Creates a new FindContentMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for FindContentMetadata {
    fn default() -> Self { Self::new() }
}
