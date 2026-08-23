//! Ported from `packages/engine/Source/Scene/ResourceLoader.js`.

/// A loader for resources.
pub struct ResourceLoader {
    _private: (),
}

impl ResourceLoader {
    /// Creates a new ResourceLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ResourceLoader {
    fn default() -> Self { Self::new() }
}
