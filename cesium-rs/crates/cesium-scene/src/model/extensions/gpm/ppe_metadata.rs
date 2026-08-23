//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Per-pixel effect metadata.
pub struct PpeMetadata {
    _private: (),
}

impl PpeMetadata {
    /// Creates a new PpeMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PpeMetadata {
    fn default() -> Self { Self::new() }
}
