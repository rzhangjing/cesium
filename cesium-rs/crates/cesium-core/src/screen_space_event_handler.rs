//! Ported from `packages/engine/Source/Core/ScreenSpaceEventHandler.js`.

/// Handles screen-space input events.
pub struct ScreenSpaceEventHandler {
    _private: (),
}

impl ScreenSpaceEventHandler {
    /// Creates a new ScreenSpaceEventHandler.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ScreenSpaceEventHandler {
    fn default() -> Self { Self::new() }
}
