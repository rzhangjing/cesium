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

/// A start/end position pair for one aggregated sub-movement.
///
/// Port of the blueprint `MouseMovement` (`camera_event_aggregator.rs` L96-102),
/// the `{ startPosition, endPosition }` object CesiumJS nests inside a pinch.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StartEnd {
    /// The aggregated start position.
    pub start_position: DVec2,
    /// The aggregated end position.
    pub end_position: DVec2,
}

/// Aggregated two-finger pinch movement (touch).
///
/// Port of the blueprint `PinchMovement` (`camera_event_aggregator.rs` L108-116)
/// and the CesiumJS `{ distance, angleAndHeight, prevAngle }` shape
/// (`CameraEventAggregator.js` L111-143):
/// - `distance` holds the two-finger separation (scalar in `.y`); its delta
///   drives zoom.
/// - `angle_and_height` holds the finger-line angle (radians, `.x`) and the
///   midpoint height (`.y`); the angle delta drives twist.
/// - `prev_angle` keeps the angle aggregation from flipping over 360°.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PinchMovement {
    /// Finger separation (start/end), scalar stored in `.y`.
    pub distance: StartEnd,
    /// Finger angle (`.x`, radians) and midpoint height (`.y`), start/end.
    pub angle_and_height: StartEnd,
    /// Two-finger midpoint (center between the fingers), start/end. Its
    /// per-frame delta is the common translation of both fingers and drives
    /// the two-finger **drag → translate** gesture (M2.6). CesiumJS does not
    /// track this (its two-finger drag is zoom+twist only), so it is an
    /// addition for the spin/translate touch model.
    pub midpoint: StartEnd,
    /// Previous angle for the anti-flip wrap.
    pub prev_angle: f64,
}

/// Computes the pinch metrics `(separation, angle, midpoint_height)` for two
/// finger positions.
///
/// `angle` is the direction of the `finger1 → finger2` line in radians; the
/// midpoint height is the average screen `y`, matching CesiumJS's pinch
/// `angleAndHeight`.
fn pinch_metrics(finger1: DVec2, finger2: DVec2) -> (f64, f64, f64) {
    let delta = finger2 - finger1;
    let distance = delta.length();
    let angle = delta.y.atan2(delta.x);
    let height = (finger1.y + finger2.y) * 0.5;
    (distance, angle, height)
}

/// Aggregates camera events per frame.
/// Maps to CesiumJS `CameraEventAggregator`
#[derive(Debug, Clone)]
pub struct CameraEventAggregator {
    /// Movement state for each event type.
    movements: Vec<(CameraEventType, AggregateMovement)>,
    /// Current frame time.
    current_time: f64,
    /// Aggregated two-finger pinch movement.
    pinch: PinchMovement,
    /// Whether a pinch gesture is currently in progress.
    pinching: bool,
    /// Whether the next pinch move should re-seed the per-frame start values.
    pinch_update_pending: bool,
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
            pinch: PinchMovement::default(),
            pinching: false,
            pinch_update_pending: false,
        }
    }

    /// Resets all movements for a new frame.
    pub fn reset(&mut self, time: f64) {
        self.current_time = time;
        for (_, movement) in &mut self.movements {
            movement.reset_frame();
        }
        // Re-seed the pinch start/prev-angle on the first move of the new frame
        // (CesiumJS `update[key] = true` on reset, CameraEventAggregator.js).
        if self.pinching {
            self.pinch_update_pending = true;
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

    // ========================================================================
    // Pinch (touch) aggregation
    // ========================================================================

    /// Begins a two-finger pinch gesture.
    ///
    /// Maps to CesiumJS `PINCH_START` (CameraEventAggregator.js L84-98): seeds
    /// the distance / angle-and-height start and end from the initial fingers.
    pub fn pinch_start(&mut self, finger1: DVec2, finger2: DVec2) {
        let (distance, angle, height) = pinch_metrics(finger1, finger2);
        let midpoint = (finger1 + finger2) * 0.5;
        self.pinching = true;
        self.pinch_update_pending = true;
        self.pinch = PinchMovement {
            distance: StartEnd {
                start_position: DVec2::new(0.0, distance),
                end_position: DVec2::new(0.0, distance),
            },
            angle_and_height: StartEnd {
                start_position: DVec2::new(angle, height),
                end_position: DVec2::new(angle, height),
            },
            midpoint: StartEnd {
                start_position: midpoint,
                end_position: midpoint,
            },
            prev_angle: angle,
        };
        let time = self.current_time;
        self.get_movement_mut(CameraEventType::Pinch)
            .button_down(midpoint, time);
    }

    /// Updates an in-progress pinch with the current finger positions.
    ///
    /// Faithful to CesiumJS `PINCH_MOVE` (CameraEventAggregator.js L111-143): the
    /// first move of a frame re-seeds the start and `prevAngle`, later moves
    /// aggregate into the end, and the angle is wrapped to stay within `π` of
    /// `prevAngle` so it never flips over 360°.
    pub fn pinch_move(&mut self, finger1: DVec2, finger2: DVec2) {
        if !self.pinching {
            return;
        }
        let (distance, angle, height) = pinch_metrics(finger1, finger2);
        let midpoint = (finger1 + finger2) * 0.5;

        if self.pinch_update_pending {
            self.pinch.distance.start_position = DVec2::new(0.0, distance);
            self.pinch.angle_and_height.start_position = DVec2::new(angle, height);
            self.pinch.midpoint.start_position = midpoint;
            self.pinch.prev_angle = angle;
            self.pinch_update_pending = false;
        }
        self.pinch.distance.end_position = DVec2::new(0.0, distance);
        self.pinch.angle_and_height.end_position = DVec2::new(angle, height);
        self.pinch.midpoint.end_position = midpoint;

        // Anti-flip wrap (CesiumJS L129-138).
        let mut wrapped = angle;
        let prev = self.pinch.prev_angle;
        let two_pi = std::f64::consts::TAU;
        while wrapped >= prev + std::f64::consts::PI {
            wrapped -= two_pi;
        }
        while wrapped < prev - std::f64::consts::PI {
            wrapped += two_pi;
        }
        self.pinch.angle_and_height.end_position.x = wrapped;

        let time = self.current_time;
        self.get_movement_mut(CameraEventType::Pinch)
            .drag((finger1 + finger2) * 0.5, time);
    }

    /// Ends the pinch gesture.
    ///
    /// Maps to CesiumJS `PINCH_END` (CameraEventAggregator.js L101-108).
    pub fn pinch_end(&mut self) {
        self.pinching = false;
        self.pinch_update_pending = false;
        let time = self.current_time;
        self.get_movement_mut(CameraEventType::Pinch).button_up(time);
    }

    /// Whether a pinch gesture is currently in progress.
    pub fn is_pinching(&self) -> bool {
        self.pinching
    }

    /// Whether the pinch event slot has `is_button_down` set (i.e. between
    /// `pinch_start` and `pinch_end`).
    pub fn is_button_down_pinch(&self) -> bool {
        self.get_movement(CameraEventType::Pinch).is_some_and(|m| m.is_button_down)
    }

    /// The aggregated pinch movement for this frame.
    pub fn pinch(&self) -> &PinchMovement {
        &self.pinch
    }

    /// The change in finger separation this frame (pixels); drives zoom.
    pub fn pinch_distance_delta(&self) -> f64 {
        self.pinch.distance.end_position.y - self.pinch.distance.start_position.y
    }

    /// The change in finger angle this frame (radians); drives twist.
    pub fn pinch_angle_delta(&self) -> f64 {
        self.pinch.angle_and_height.end_position.x - self.pinch.prev_angle
    }

    /// The change in the two-finger midpoint this frame (pixels); drives the
    /// two-finger **drag → translate** gesture. Like the distance/angle deltas
    /// it is re-seeded each frame, so it is the per-frame common translation
    /// of both fingers rather than the cumulative offset since `pinch_start`.
    pub fn pinch_midpoint_delta(&self) -> DVec2 {
        self.pinch.midpoint.end_position - self.pinch.midpoint.start_position
    }

    /// The twist delta in pixels, reproducing CesiumJS's
    /// `(-angle * canvas.clientWidth) / 12` scaling
    /// (CameraEventAggregator.js L139-142). The canvas width is supplied by the
    /// adapter boundary so the domain stays resolution-independent.
    pub fn pinch_twist_pixels(&self, canvas_width: f64) -> f64 {
        (-self.pinch_angle_delta() * canvas_width) / 12.0
    }

    // ========================================================================
    // Semantic gesture deltas (feed the controller's spin / look actions)
    // ========================================================================

    /// The aggregated spin gesture delta (left-drag, pixels).
    ///
    /// Feeds [`crate::camera_controller::CameraController::spin`]. Maps to the
    /// CesiumJS default `LEFT_DRAG → spin3D` binding; whether a left-drag
    /// becomes a spin, pan, or look is decided by the controller (based on
    /// picking), not the aggregator.
    pub fn spin_delta(&self) -> DVec2 {
        self.get_movement_delta(CameraEventType::LeftDrag)
    }

    /// The aggregated look gesture delta (right-drag, pixels).
    ///
    /// Feeds [`crate::camera_controller::CameraController::look`]. The binding is
    /// a port default; the application may re-map it.
    pub fn look_delta(&self) -> DVec2 {
        self.get_movement_delta(CameraEventType::RightDrag)
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

    #[test]
    fn test_pinch_start_move_end_lifecycle() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        assert!(!agg.is_pinching());

        agg.pinch_start(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        assert!(agg.is_pinching());
        assert!(agg.is_button_down_pinch());

        agg.pinch_end();
        assert!(!agg.is_pinching());
    }

    #[test]
    fn test_pinch_zoom_distance_delta() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        agg.pinch_start(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        // First move seeds the per-frame start; second aggregates into the end.
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(200.0, 0.0));
        assert!((agg.pinch_distance_delta() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_pinch_angle_delta_drives_twist() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        agg.pinch_start(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0)); // seed angle 0
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(0.0, 100.0)); // rotate to 90°
        assert!((agg.pinch_angle_delta() - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        // Pixel scaling reproduces CesiumJS (-angle * width) / 12.
        let expected = (-std::f64::consts::FRAC_PI_2 * 1200.0) / 12.0;
        assert!((agg.pinch_twist_pixels(1200.0) - expected).abs() < 1e-9);
    }

    #[test]
    fn test_pinch_angle_anti_flip_over_360() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        agg.pinch_start(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        // Seed prev_angle just below +π.
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(-100.0, 1.0));
        // Cross to just above -π; the wrap keeps the delta tiny instead of ~2π.
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(-100.0, -1.0));
        let delta = agg.pinch_angle_delta();
        assert!(delta.abs() < 0.1, "anti-flip failed, delta = {delta}");
    }

    #[test]
    fn test_pinch_midpoint_delta_drives_translate() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        agg.pinch_start(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        // First move seeds the per-frame midpoint start; second extends the end.
        // Both fingers drift right by 50 px → midpoint delta = (+50, 0).
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        agg.pinch_move(DVec2::new(50.0, 0.0), DVec2::new(150.0, 0.0));
        let mid = agg.pinch_midpoint_delta();
        assert!((mid.x - 50.0).abs() < 1e-9, "midpoint x delta = {}", mid.x);
        assert!(mid.y.abs() < 1e-9);
    }

    #[test]
    fn test_pinch_midpoint_delta_re_seeds_each_frame() {
        let mut agg = CameraEventAggregator::new();
        agg.pinch_start(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        // Frame 1: midpoint drifts +30 x.
        agg.reset(0.0);
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        agg.pinch_move(DVec2::new(30.0, 0.0), DVec2::new(130.0, 0.0));
        assert!((agg.pinch_midpoint_delta().x - 30.0).abs() < 1e-9);
        // Frame 2 re-seeds: drift is measured from frame 2's own start, not
        // cumulative from pinch_start.
        agg.reset(1.0 / 60.0);
        agg.pinch_move(DVec2::new(30.0, 0.0), DVec2::new(130.0, 0.0));
        agg.pinch_move(DVec2::new(30.0, 40.0), DVec2::new(130.0, 40.0));
        let mid = agg.pinch_midpoint_delta();
        assert!(mid.x.abs() < 1e-9, "x re-seeded to 0, got {}", mid.x);
        assert!((mid.y - 40.0).abs() < 1e-9, "y delta = {}", mid.y);
    }

    #[test]
    fn test_pinch_move_ignored_without_start() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        agg.pinch_move(DVec2::new(0.0, 0.0), DVec2::new(100.0, 0.0));
        assert!(!agg.is_pinching());
        assert!((agg.pinch_distance_delta()).abs() < 1e-12);
    }

    #[test]
    fn test_spin_delta_reads_left_drag() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        agg.button_down(MouseButton::Left, DVec2::new(0.0, 0.0));
        agg.mouse_move(MouseButton::Left, DVec2::new(30.0, 40.0));
        let delta = agg.spin_delta();
        assert!((delta.x - 30.0).abs() < 1e-9);
        assert!((delta.y - 40.0).abs() < 1e-9);
    }

    #[test]
    fn test_look_delta_reads_right_drag() {
        let mut agg = CameraEventAggregator::new();
        agg.reset(0.0);
        agg.button_down(MouseButton::Right, DVec2::new(0.0, 0.0));
        agg.mouse_move(MouseButton::Right, DVec2::new(10.0, -5.0));
        let delta = agg.look_delta();
        assert!((delta.x - 10.0).abs() < 1e-9);
        assert!((delta.y - (-5.0)).abs() < 1e-9);
    }
}
