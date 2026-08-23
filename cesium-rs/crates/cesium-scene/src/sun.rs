//! Ported from `packages/engine/Source/Scene/Sun.js`.
//!
//! Draws the sun as a billboard in the scene.

use cesium_core::cartesian3::Cartesian3;
use crate::frame_state::FrameState;

/// Draws the sun as a billboard in the scene.
///
/// The sun's position is computed from the scene's time and the
/// astronomical ephemeris.
pub struct Sun {
    /// Whether the sun is shown.
    pub show: bool,
    /// The sun's current position in world coordinates (computed each frame).
    pub position: Cartesian3,
    /// The sun's direction from the camera (computed each frame).
    pub direction: Cartesian3,
    /// Whether this has been destroyed.
    is_destroyed: bool,
}

impl Sun {
    /// Creates a new Sun.
    pub fn new() -> Self {
        Self {
            show: true,
            position: Cartesian3::default(),
            direction: Cartesian3::default(),
            is_destroyed: false,
        }
    }

    /// Updates the sun position for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.show { return; }
        // In full port: compute sun position from JulianDate using Simon 1994 ephemeris
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Sun {
    fn default() -> Self { Self::new() }
}
