//! Ported from `packages/engine/Source/Scene/GroupMetadata.js`.

/// Group metadata.
pub struct GroupMetadata {
    _private: (),
}

impl GroupMetadata {
    /// Creates a new GroupMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GroupMetadata {
    fn default() -> Self { Self::new() }
}
