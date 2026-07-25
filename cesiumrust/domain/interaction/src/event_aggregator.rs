//! Camera event aggregation system.
//!
//! Maps to CesiumJS `Scene/CameraEventAggregator.js`
//!
//! Aggregates mouse/keyboard events per frame for camera control.

use glam::DVec2;

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
}

/// Camera event types.
/// Maps to CesiumJS `CameraEventType`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CameraEventType {
    /// Left mouse button down.
    LeftDown,
    /// Left mouse button up.
    LeftUp,
    /// Left mouse button drag.
    LeftDrag,
    /// Right mouse button down.
    RightDown,
    /// Right mouse button up.
    RightUp,
    /// Right mouse button drag.
    RightDrag,
    /// Middle mouse button down.
    MiddleDown,
    /// Middle mouse button up.
    MiddleUp,
    /// Middle mouse button drag.
    MiddleDrag,
    /// Mouse wheel scroll.
    Wheel,
    /// Pinch (touch).
    Pinch,
}

/// Aggregated movement data for a single event type.
#[derive(Debug, Clone, Default)]
pub struct AggregateMovement {
    /// Starting position of the movement.
    pub start_position: DVec2,
    /// Ending position of the movement.
    pub end_position: DVec2,
    /// Total movement delta.
    pub movement: DVec2,
    /// Whether the button is currently down.
    pub is_button_down: bool,
    /// Whether a movement occurred this frame.
    pub is_moving: bool,
    /// Time the movement started (seconds).
    pub start_time: f64,
    /// Time the last movement occurred (seconds).
    pub last_time: f64,
}

impl AggregateMovement {
    /// Creates a new empty aggregate movement.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the movement state for a new frame.
    pub fn reset_frame(&mut self) {
        self.movement = DVec2::ZERO;
        self.is_moving = false;
    }

    /// Records a button down event.
    pub fn button_down(&mut self, position: DVec2, time: f64) {
        self.is_button_down = true;
        self.start_position = position;
        self.end_position = position;
        self.start_time = time;
        self.last_time = time;
    }

    /// Records a button up event.
    pub fn button_up(&mut self, time: f64) {
        self.is_button_down = false;
        self.last_time = time;
    }

    /// Records a drag/move event.
    pub fn drag(&mut self, position: DVec2, time: f64) {
        if self.is_button_down {
            self.end_position = position;
            self.movement = self.end_position - self.start_position;
            self.is_moving = true;
            self.last_time = time;
        }
    }

    /// Records a wheel event.
    pub fn wheel(&mut self, delta: f64, time: f64) {
        self.movement = DVec2::new(0.0, delta);
        self.is_moving = true;
        self.last_time = time;
    }
}

/// Aggregates camera events per frame.
/// Maps to CesiumJS `CameraEventAggregator`
#[derive(Debug, Clone)]
pub struct CameraEventAggregator {
    /// Movement state for each event type.
    movements: Vec<(CameraEventType, AggregateMovement)>,
    /// Current frame time.
    current_time: f64,
}

impl Default for CameraEventAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraEventAggregator {
    /// Creates a new event aggregator.
    pub fn new() -> Self {
        Self {
            movements: Vec::new(),
            current_time: 0.0,
        }
    }

    /// Resets all movements for a new frame.
    pub fn reset(&mut self, time: f64) {
        self.current_time = time;
        for (_, movement) in &mut self.movements {
            movement.reset_frame();
        }
    }

    /// Gets or creates the movement state for an event type.
    fn get_movement_mut(&mut self, event_type: CameraEventType) -> &mut AggregateMovement {
        let found = self.movements.iter().position(|(t, _)| *t == event_type);
        let idx = match found {
            Some(idx) => idx,
            None => {
                self.movements.push((event_type, AggregateMovement::new()));
                self.movements.len() - 1
            }
        };
        &mut self.movements[idx].1
    }

    /// Gets the movement state for an event type.
    pub fn get_movement(&self, event_type: CameraEventType) -> Option<&AggregateMovement> {
        self.movements.iter().find(|(t, _)| *t == event_type).map(|(_, m)| m)
    }

    /// Records a button down event.
    pub fn button_down(&mut self, button: MouseButton, position: DVec2) {
        let time = self.current_time;
        let event_type = match button {
            MouseButton::Left => CameraEventType::LeftDown,
            MouseButton::Right => CameraEventType::RightDown,
            MouseButton::Middle => CameraEventType::MiddleDown,
        };
        self.get_movement_mut(event_type).button_down(position, time);

        // Also mark the drag event as button down
        let drag_type = match button {
            MouseButton::Left => CameraEventType::LeftDrag,
            MouseButton::Right => CameraEventType::RightDrag,
            MouseButton::Middle => CameraEventType::MiddleDrag,
        };
        self.get_movement_mut(drag_type).button_down(position, time);
    }

    /// Records a button up event.
    pub fn button_up(&mut self, button: MouseButton) {
        let time = self.current_time;
        let event_type = match button {
            MouseButton::Left => CameraEventType::LeftUp,
            MouseButton::Right => CameraEventType::RightUp,
            MouseButton::Middle => CameraEventType::MiddleUp,
        };
        self.get_movement_mut(event_type).button_up(time);

        // Also mark the drag event as button up
        let drag_type = match button {
            MouseButton::Left => CameraEventType::LeftDrag,
            MouseButton::Right => CameraEventType::RightDrag,
            MouseButton::Middle => CameraEventType::MiddleDrag,
        };
        self.get_movement_mut(drag_type).button_up(time);
    }

    /// Records a mouse move/drag event.
    pub fn mouse_move(&mut self, button: MouseButton, position: DVec2) {
        let time = self.current_time;
        let drag_type = match button {
            MouseButton::Left => CameraEventType::LeftDrag,
            MouseButton::Right => CameraEventType::RightDrag,
            MouseButton::Middle => CameraEventType::MiddleDrag,
        };
        self.get_movement_mut(drag_type).drag(position, time);
    }

    /// Records a wheel scroll event.
    pub fn wheel(&mut self, delta: f64) {
        let time = self.current_time;
        self.get_movement_mut(CameraEventType::Wheel).wheel(delta, time);
    }

    /// Checks if a specific event type is currently moving.
    pub fn is_moving(&self, event_type: CameraEventType) -> bool {
        self.get_movement(event_type).is_some_and(|m| m.is_moving)
    }

    /// Checks if a button is currently down.
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        let drag_type = match button {
            MouseButton::Left => CameraEventType::LeftDrag,
            MouseButton::Right => CameraEventType::RightDrag,
            MouseButton::Middle => CameraEventType::MiddleDrag,
        };
        self.get_movement(drag_type).is_some_and(|m| m.is_button_down)
    }

    /// Gets the movement delta for an event type.
    pub fn get_movement_delta(&self, event_type: CameraEventType) -> DVec2 {
        self.get_movement(event_type).map_or(DVec2::ZERO, |m| m.movement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_movement_new() {
        let movement = AggregateMovement::new();
        assert!(!movement.is_button_down);
        assert!(!movement.is_moving);
        assert_eq!(movement.movement, DVec2::ZERO);
    }

    #[test]
    fn test_button_down_up() {
        let mut movement = AggregateMovement::new();
        movement.button_down(DVec2::new(100.0, 200.0), 0.0);
        assert!(movement.is_button_down);
        assert_eq!(movement.start_position, DVec2::new(100.0, 200.0));

        movement.button_up(1.0);
        assert!(!movement.is_button_down);
    }

    #[test]
    fn test_drag() {
        let mut movement = AggregateMovement::new();
        movement.button_down(DVec2::new(100.0, 100.0), 0.0);
        movement.drag(DVec2::new(150.0, 120.0), 0.5);

        assert!(movement.is_moving);
        assert_eq!(movement.movement, DVec2::new(50.0, 20.0));
    }

    #[test]
    fn test_wheel() {
        let mut movement = AggregateMovement::new();
        movement.wheel(120.0, 0.0);
        assert!(movement.is_moving);
        assert!((movement.movement.y - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_event_aggregator_basic() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);

        agg.button_down(MouseButton::Left, DVec2::new(100.0, 100.0));
        assert!(agg.is_button_down(MouseButton::Left));
        assert!(!agg.is_button_down(MouseButton::Right));

        agg.mouse_move(MouseButton::Left, DVec2::new(150.0, 130.0));
        assert!(agg.is_moving(CameraEventType::LeftDrag));

        let delta = agg.get_movement_delta(CameraEventType::LeftDrag);
        assert!((delta.x - 50.0).abs() < 1e-10);
        assert!((delta.y - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_event_aggregator_reset() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);

        agg.button_down(MouseButton::Left, DVec2::new(100.0, 100.0));
        agg.mouse_move(MouseButton::Left, DVec2::new(200.0, 200.0));

        // Reset for new frame
        agg.reset(1.0 / 60.0);
        assert!(!agg.is_moving(CameraEventType::LeftDrag));
        // Button should still be down
        assert!(agg.is_button_down(MouseButton::Left));
    }

    #[test]
    fn test_event_aggregator_wheel() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);

        agg.wheel(-120.0);
        assert!(agg.is_moving(CameraEventType::Wheel));
        let delta = agg.get_movement_delta(CameraEventType::Wheel);
        assert!((delta.y - (-120.0)).abs() < 1e-10);
    }

    #[test]
    fn test_multiple_buttons() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);

        agg.button_down(MouseButton::Left, DVec2::new(0.0, 0.0));
        agg.button_down(MouseButton::Right, DVec2::new(100.0, 100.0));

        assert!(agg.is_button_down(MouseButton::Left));
        assert!(agg.is_button_down(MouseButton::Right));
        assert!(!agg.is_button_down(MouseButton::Middle));

        agg.button_up(MouseButton::Left);
        assert!(!agg.is_button_down(MouseButton::Left));
        assert!(agg.is_button_down(MouseButton::Right));
    }
}
