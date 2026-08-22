//! Ported from `packages/engine/Source/Core/VideoSynchronizer.js`.
//!
//! Synchronizes video playback with the scene clock.

/// Synchronizes video playback with the Cesium scene clock.
/// Skeleton: requires HTML5 video element.
pub struct VideoSynchronizer;

impl VideoSynchronizer {
    /// Creates a new video synchronizer.
    pub fn new() -> Self {
        Self
    }

    /// Destroys the synchronizer.
    pub fn destroy(&mut self) {}
}
