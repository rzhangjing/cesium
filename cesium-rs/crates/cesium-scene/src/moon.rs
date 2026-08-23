//! Ported from `packages/engine/Source/Scene/Moon.js`.
//!
//! Draws the moon as a textured sphere in the scene.

use cesium_core::cartesian3::Cartesian3;
use crate::frame_state::FrameState;

/// Draws the moon as a textured sphere in the scene.
///
/// The moon's position and phase are computed from the scene's time
/// using an astronomical ephemeris.
pub struct Moon {
    /// Whether the moon is shown.
    pub show: bool,
    /// Whether to show the moon only at night.
    pub only_sun_at_night: bool,
    /// The moon's texture URL.
    pub texture_url: Option<String>,
    /// The moon's current position in world coordinates (computed each frame).
    pub position: Cartesian3,
    /// The moon's current phase (0.0 = new moon, 0.5 = full moon, 1.0 = new moon).
    pub phase: f64,
    /// Whether this has been destroyed.
    is_destroyed: bool,
}

impl Moon {
    /// Creates a new Moon.
    pub fn new() -> Self {
        Self {
            show: true,
            only_sun_at_night: false,
            texture_url: None,
            position: Cartesian3::default(),
            phase: 0.0,
            is_destroyed: false,
        }
    }

    /// Updates the moon position and phase for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.show { return; }
        // In full port: compute moon position from JulianDate using Meeus ephemeris
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Moon {
    fn default() -> Self { Self::new() }
}
