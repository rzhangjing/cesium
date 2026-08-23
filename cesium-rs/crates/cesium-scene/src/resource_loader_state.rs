//! Ported from `packages/engine/Source/Scene/ResourceLoaderState.js`.

/// The state of a resource loader.
pub struct ResourceLoaderState {
    _private: (),
}

impl ResourceLoaderState {
    /// Creates a new ResourceLoaderState.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ResourceLoaderState {
    fn default() -> Self { Self::new() }
}
