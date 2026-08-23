//! Ported from `packages/engine/Source/Scene/ResourceCache.js`.

/// A cache for loaded resources.
pub struct ResourceCache {
    _private: (),
}

impl ResourceCache {
    /// Creates a new ResourceCache.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ResourceCache {
    fn default() -> Self { Self::new() }
}
