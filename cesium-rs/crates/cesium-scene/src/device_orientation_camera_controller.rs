//! Ported from `packages/engine/Source/Scene/DeviceOrientationCameraController.js`.

/// Device orientation camera controller.
///
/// Controls the camera using device orientation sensors on mobile.
pub struct DeviceOrientationCameraController {
    /// Whether the controller is active.
    pub enabled: bool,
}

impl DeviceOrientationCameraController {
    /// Creates a new DeviceOrientationCameraController.
    pub fn new() -> Self { Self { enabled: false } }
}

impl Default for DeviceOrientationCameraController {
    fn default() -> Self { Self::new() }
}
