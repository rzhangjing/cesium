//! Ported from `packages/engine/Source/Scene/findGroupMetadata.js`.

/// Finds group metadata.
pub struct FindGroupMetadata {
    _private: (),
}

impl FindGroupMetadata {
    /// Creates a new FindGroupMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for FindGroupMetadata {
    fn default() -> Self { Self::new() }
}
