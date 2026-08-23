//! Ported from `packages/engine/Source/Renderer/SharedContext.js`.

/// Shared rendering context state.
pub struct SharedContext {
    _private: (),
}

impl SharedContext {
    /// Creates a new SharedContext.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SharedContext {
    fn default() -> Self { Self::new() }
}
