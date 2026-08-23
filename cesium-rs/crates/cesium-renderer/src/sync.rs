//! Ported from `packages/engine/Source/Renderer/sync.js`.

/// Synchronizes GPU operations.
pub struct Sync {
    _private: (),
}

impl Sync {
    /// Creates a new Sync.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Sync {
    fn default() -> Self { Self::new() }
}
