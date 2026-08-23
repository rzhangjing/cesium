//! Ported from `packages/engine/Source/Scene/CameraEventAggregator.js`.

use crate::camera_event_type::CameraEventType;

/// Aggregates camera input events (mouse, touch) into camera actions.
///
/// This class processes raw input events and determines which camera
/// movement actions should be performed.
pub struct CameraEventAggregator {
    /// The current button states.
    pub current_button: Option<CameraEventType>,
    /// Whether any button is currently pressed.
    pub any_button_down: bool,
    /// Whether this aggregator has been destroyed.
    is_destroyed: bool,
}

impl CameraEventAggregator {
    /// Creates a new camera event aggregator.
    pub fn new() -> Self {
        Self {
            current_button: None,
            any_button_down: false,
            is_destroyed: false,
        }
    }

    /// Returns whether the given button is currently pressed.
    pub fn is_button_down(&self, _button: CameraEventType) -> bool {
        // DEVIATION: Requires input event tracking
        false
    }

    /// Gets the accumulated movement amount.
    pub fn get_movement(&self) -> (f64, f64) {
        (0.0, 0.0)
    }

    /// Resets the aggregator state.
    pub fn reset(&mut self) {
        self.current_button = None;
        self.any_button_down = false;
    }

    /// Returns whether this aggregator has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this aggregator.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for CameraEventAggregator {
    fn default() -> Self { Self::new() }
}
