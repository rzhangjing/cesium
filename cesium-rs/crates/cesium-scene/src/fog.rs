//! Ported from `packages/engine/Source/Scene/Fog.js`.
//!
//! Fog effect based on distance from the camera.

use cesium_core::color::Color;

use crate::frame_state::FrameState;

/// Fog effect that fades objects to a specified color based on distance.
///
/// Mirrors CesiumJS `Fog` (252 lines).
pub struct Fog {
    /// Whether fog is enabled.
    pub enabled: bool,
    /// The minimum brightness of the fog (0.0 to 1.0).
    pub minimum_brightness: f64,
    /// The maximum brightness of the fog (0.0 to 1.0).
    pub maximum_brightness: f64,
    /// The density of the fog (0.0 to 1.0).
    pub density: f64,
    /// The screen space error factor for fog.
    pub screen_space_error_factor: f64,
    /// The fog color (usually the sky color).
    pub color: Color,
}

impl Fog {
    /// Creates a new Fog with default settings.
    pub fn new() -> Self {
        Self {
            enabled: true,
            minimum_brightness: 0.03,
            maximum_brightness: 0.5,
            density: 2.0e-4,
            screen_space_error_factor: 2.0,
            color: Color::new(0.7, 0.8, 0.9, 1.0),
        }
    }

    /// Updates fog parameters based on camera altitude.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Adjust density based on camera height above terrain
    }
}

impl Default for Fog {
    fn default() -> Self { Self::new() }
}
