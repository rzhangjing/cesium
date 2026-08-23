//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Storage type for GPM data.
pub struct StorageType {
    _private: (),
}

impl StorageType {
    /// Creates a new StorageType.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for StorageType {
    fn default() -> Self { Self::new() }
}
