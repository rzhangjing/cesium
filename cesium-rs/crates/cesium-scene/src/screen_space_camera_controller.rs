//! Ported from `packages/engine/Source/Scene/ScreenSpaceCameraController.js`.

/// Controls the camera via screen-space input events.
///
/// Handles mouse drag, wheel, and touch events to move the camera.
pub struct ScreenSpaceCameraController {
    /// Whether the controller is enabled.
    pub enable_rotate: bool,
    /// Whether zoom is enabled.
    pub enable_zoom: bool,
    /// Whether tilt is enabled.
    pub enable_tilt: bool,
    /// The rotation speed multiplier.
    pub rotation_rate: f64,
    /// The zoom speed multiplier.
    pub zoom_rate: f64,
    /// The minimum zoom distance.
    pub minimum_zoom_distance: f64,
    /// The maximum zoom distance.
    pub maximum_zoom_distance: f64,
    is_destroyed: bool,
}

impl ScreenSpaceCameraController {
    /// Creates a new screen space camera controller.
    pub fn new() -> Self {
        Self {
            enable_rotate: true,
            enable_zoom: true,
            enable_tilt: true,
            rotation_rate: 1.0,
            zoom_rate: 1.0,
            minimum_zoom_distance: 1.0,
            maximum_zoom_distance: f64::MAX,
            is_destroyed: false,
        }
    }

    /// Updates the controller.
    pub fn update(&mut self) {
        // DEVIATION: Requires input event processing and camera manipulation
    }

    /// Returns whether this controller has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this controller.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for ScreenSpaceCameraController {
    fn default() -> Self { Self::new() }
}
