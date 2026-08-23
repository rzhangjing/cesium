//! Ported from `packages/engine/Source/Scene/ImplicitSubtreeCache.js`.

/// Implicit subtree cache.
pub struct ImplicitSubtreeCache {
    _private: (),
}

impl ImplicitSubtreeCache {
    /// Creates a new ImplicitSubtreeCache.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitSubtreeCache {
    fn default() -> Self { Self::new() }
}
