//! Ported from `packages/engine/Source/Scene/DeviceOrientationCameraController.js`.

/// A device orientation camera controller.
///
/// DEVIATION: requires device orientation API for full implementation.
pub struct DeviceOrientationCameraController {
    _private: (),
}

impl DeviceOrientationCameraController {
    /// Creates a new device orientation camera controller.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DeviceOrientationCameraController {
    fn default() -> Self { Self::new() }
}
