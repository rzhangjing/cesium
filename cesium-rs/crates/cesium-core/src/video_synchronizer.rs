//! Ported from `packages/engine/Source/Core/VideoSynchronizer.js`.

/// Synchronizes video playback with the scene clock.
pub struct VideoSynchronizer {
    _private: (),
}

impl VideoSynchronizer {
    /// Creates a new VideoSynchronizer.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VideoSynchronizer {
    fn default() -> Self { Self::new() }
}
