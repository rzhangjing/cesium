//! Ported from `packages/engine/Source/Scene/CameraEventAggregator.js`.
//!
//! Aggregates input events. For example, suppose the following inputs are
//! received between frames: left mouse button down, mouse move, mouse move,
//! left mouse button up. These events are aggregated into one event with a
//! start and end position of the mouse.
//!
//! # DEVIATION (mechanism, not behavior)
//!
//! The CesiumJS aggregator owns a [`ScreenSpaceEventHandler`] and, at
//! construction, registers *closures* into it (`listenToWheel`, `listenToPinch`,
//! `listenMouseButtonDownUp`, `listenMouseMove`). Those closures capture the
//! aggregator's own maps (`_movement`, `_isDown`, `_update`, `_pressTime`, ...)
//! and mutate them, while the handler itself is owned by the aggregator. That
//! closure-into-own-handler pattern is a circular ownership + aliasable
//! mutable borrow, which safe Rust forbids.
//!
//! This port keeps the aggregation *state* as direct fields and replaces the
//! closure dispatch with a single feed entry point,
//! [`CameraEventAggregator::handle_input_event`]. The driver (the `Scene`, or a
//! test) pushes raw [`ScreenSpaceInputEvent`]s through it; the same per-key
//! aggregation, press/release stamping, button counting, and modifier-transfer
//! logic then runs. The registry key scheme, the initialized key set, and every
//! public getter's observable result are reproduced faithfully.
//!
//! [`ScreenSpaceEventHandler`]: cesium_core::screen_space_event_handler::ScreenSpaceEventHandler

use crate::camera_event_type::CameraEventType;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::get_timestamp::get_timestamp;
use cesium_core::keyboard_event_modifier::KeyboardEventModifier;
use cesium_core::screen_space_event_handler::{
    MotionEvent, PinchMovementEvent, ScreenSpaceInputEvent,
};
use cesium_core::screen_space_event_type::ScreenSpaceEventType;
use std::collections::HashMap;

/// All `CameraEventType` values, in enum-declaration order.
///
/// Mirrors the CesiumJS `for (const typeName in CameraEventType)` iteration.
const ALL_CAMERA_EVENT_TYPES: [CameraEventType; 5] = [
    CameraEventType::LeftDrag,
    CameraEventType::MiddleDrag,
    CameraEventType::RightDrag,
    CameraEventType::Wheel,
    CameraEventType::Pinch,
];

/// The seven keyboard-modifier combinations the aggregator listens for.
///
/// Port of the module-level `keyboardModifierCombinations`.
fn keyboard_modifier_combinations() -> [Vec<KeyboardEventModifier>; 7] {
    use KeyboardEventModifier::{Alt, Ctrl, Shift};
    [
        vec![Shift],
        vec![Ctrl],
        vec![Alt],
        vec![Shift, Ctrl],
        vec![Shift, Alt],
        vec![Ctrl, Alt],
        vec![Shift, Ctrl, Alt],
    ]
}

/// Every modifier variant the aggregator initializes/cancels: the base
/// (no-modifier) variant followed by the seven combinations.
fn all_modifier_variants() -> Vec<Vec<KeyboardEventModifier>> {
    let mut variants = Vec::with_capacity(8);
    variants.push(Vec::new());
    variants.extend(keyboard_modifier_combinations());
    variants
}

/// Builds the aggregation-map key for a `(type, modifiers)` pair.
///
/// Port of the module-private `getKey(type, modifiers)`: `"${type}"` with no
/// modifiers, otherwise `"${type}+${sorted.join("+")}"`. An empty slice models
/// CesiumJS `undefined`.
fn get_key(type_: CameraEventType, modifiers: &[KeyboardEventModifier]) -> String {
    if modifiers.is_empty() {
        return format!("{}", type_ as u8);
    }

    let mut modifier_list = modifiers.to_vec();
    modifier_list.sort();
    let joined: Vec<String> = modifier_list
        .iter()
        .map(|modifier| format!("{}", *modifier as i32))
        .collect();
    format!("{}+{}", type_ as u8, joined.join("+"))
}

/// The aggregated start/end position pair for a mouse-drag or wheel key.
///
/// Port of the `_movement[key]` object `{ startPosition, endPosition }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseMovement {
    /// The aggregated start position.
    pub start_position: Cartesian2,
    /// The aggregated end position.
    pub end_position: Cartesian2,
}

/// The aggregated pinch movement for a pinch key.
///
/// Port of the pinch-shaped `_movement[key]` object
/// `{ distance, angleAndHeight, prevAngle }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PinchMovement {
    /// The aggregated pinch distance (start/end).
    pub distance: MouseMovement,
    /// The aggregated pinch angle-and-height (start/end).
    pub angle_and_height: MouseMovement,
    /// The previous angle, used to keep aggregation from flipping over 360°.
    pub prev_angle: f64,
}

/// The aggregated movement stored under a key.
///
/// CesiumJS stores a duck-typed object whose shape depends on the key: mouse and
/// wheel keys hold `{ startPosition, endPosition }`, while pinch keys hold
/// `{ distance, angleAndHeight, prevAngle }`. This enum makes that split
/// explicit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Movement {
    /// A mouse-drag or wheel movement.
    Mouse(MouseMovement),
    /// A pinch movement.
    Pinch(PinchMovement),
}

/// The start/end position of the last (non-aggregated) move event.
///
/// Port of the `_lastMovement[key]` object `{ startPosition, endPosition, valid }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LastMovement {
    /// The start position of the last move.
    pub start_position: Cartesian2,
    /// The end position of the last move.
    pub end_position: Cartesian2,
    /// Whether the last movement is valid (has been populated).
    pub valid: bool,
}

/// Aggregates camera input events (mouse, touch, wheel) between animation
/// frames.
///
/// Port of `CameraEventAggregator`. See the module-level DEVIATION note for the
/// headless adaptation of the closure-registration mechanism.
#[derive(Debug)]
pub struct CameraEventAggregator {
    /// Port of `_update` (per key: `true` = no movement aggregated this frame).
    update: HashMap<String, bool>,
    /// Port of `_movement`.
    movement: HashMap<String, Movement>,
    /// Port of `_lastMovement`.
    last_movement: HashMap<String, LastMovement>,
    /// Port of `_isDown`.
    is_down: HashMap<String, bool>,
    /// Port of `_eventStartPosition`.
    event_start_position: HashMap<String, Cartesian2>,
    /// Port of `_pressTime`. Milliseconds from [`get_timestamp`] model the
    /// CesiumJS `Date`; only differences are observable, so a monotonic clock is
    /// a faithful substitute. `None` models an absent (`undefined`) entry.
    press_time: HashMap<String, f64>,
    /// Port of `_releaseTime`. See [`CameraEventAggregator::press_time`].
    release_time: HashMap<String, f64>,
    /// Port of `_buttonsDown`.
    buttons_down: i32,
    /// Port of `_currentMousePosition`.
    current_mouse_position: Cartesian2,
    /// Port of the `canvas.clientWidth` the pinch listener reads when scaling
    /// the aggregated angle. Stored rather than read live (headless); the
    /// `Scene` should refresh it via [`CameraEventAggregator::set_canvas_client_width`].
    canvas_client_width: f64,
    /// Backs [`CameraEventAggregator::is_destroyed`].
    is_destroyed: bool,
}

impl CameraEventAggregator {
    /// Creates a new `CameraEventAggregator`.
    ///
    /// Port of the `CameraEventAggregator(canvas)` constructor. CesiumJS throws a
    /// `DeveloperError` when `canvas` is undefined; the headless port takes the
    /// canvas's client width (used only for pinch-angle scaling) instead of a DOM
    /// node. The constructor then registers the wheel/pinch/mouse listeners for
    /// the base variant and the seven modifier combinations, which populates the
    /// aggregation maps for every `(type, modifier)` key; the equivalent
    /// initialization is performed here.
    pub fn new(canvas_client_width: f64) -> Self {
        let mut aggregator = Self {
            update: HashMap::new(),
            movement: HashMap::new(),
            last_movement: HashMap::new(),
            is_down: HashMap::new(),
            event_start_position: HashMap::new(),
            press_time: HashMap::new(),
            release_time: HashMap::new(),
            buttons_down: 0,
            current_mouse_position: Cartesian2::ZERO,
            canvas_client_width,
            is_destroyed: false,
        };

        for modifiers in all_modifier_variants() {
            aggregator.init_wheel(&modifiers);
            aggregator.init_pinch(&modifiers);
            aggregator.init_mouse_button_down_up(&modifiers, CameraEventType::LeftDrag);
            aggregator.init_mouse_button_down_up(&modifiers, CameraEventType::RightDrag);
            aggregator.init_mouse_button_down_up(&modifiers, CameraEventType::MiddleDrag);
            aggregator.init_mouse_move(&modifiers);
        }

        aggregator
    }

    /// Updates the canvas client width used for pinch-angle scaling.
    pub fn set_canvas_client_width(&mut self, canvas_client_width: f64) {
        self.canvas_client_width = canvas_client_width;
    }

    // ---- Construction-time initialization (port of the `listen*` setup) ----

    /// Port of the map initialization performed by `listenToWheel`.
    fn init_wheel(&mut self, modifiers: &[KeyboardEventModifier]) {
        let key = get_key(CameraEventType::Wheel, modifiers);
        self.update.insert(key.clone(), true);
        self.movement.insert(
            key.clone(),
            Movement::Mouse(MouseMovement {
                start_position: Cartesian2::ZERO,
                end_position: Cartesian2::ZERO,
            }),
        );
        self.last_movement.entry(key).or_insert(LastMovement {
            start_position: Cartesian2::ZERO,
            end_position: Cartesian2::ZERO,
            valid: false,
        });
    }

    /// Port of the map initialization performed by `listenToPinch`.
    fn init_pinch(&mut self, modifiers: &[KeyboardEventModifier]) {
        let key = get_key(CameraEventType::Pinch, modifiers);
        self.update.insert(key.clone(), true);
        self.is_down.insert(key.clone(), false);
        self.event_start_position
            .insert(key.clone(), Cartesian2::ZERO);
        self.movement.insert(
            key,
            Movement::Pinch(PinchMovement {
                distance: MouseMovement {
                    start_position: Cartesian2::ZERO,
                    end_position: Cartesian2::ZERO,
                },
                angle_and_height: MouseMovement {
                    start_position: Cartesian2::ZERO,
                    end_position: Cartesian2::ZERO,
                },
                prev_angle: 0.0,
            }),
        );
    }

    /// Port of the map initialization performed by `listenMouseButtonDownUp`.
    fn init_mouse_button_down_up(
        &mut self,
        modifiers: &[KeyboardEventModifier],
        type_: CameraEventType,
    ) {
        let key = get_key(type_, modifiers);
        self.is_down.insert(key.clone(), false);
        self.event_start_position
            .insert(key.clone(), Cartesian2::ZERO);
        self.last_movement.entry(key).or_insert(LastMovement {
            start_position: Cartesian2::ZERO,
            end_position: Cartesian2::ZERO,
            valid: false,
        });
    }

    /// Port of the map initialization performed by `listenMouseMove` (its
    /// per-`CameraEventType` loop, not the closure body).
    fn init_mouse_move(&mut self, modifiers: &[KeyboardEventModifier]) {
        for type_ in ALL_CAMERA_EVENT_TYPES {
            let key = get_key(type_, modifiers);
            self.update.insert(key.clone(), true);
            self.last_movement.entry(key.clone()).or_insert(LastMovement {
                start_position: Cartesian2::ZERO,
                end_position: Cartesian2::ZERO,
                valid: false,
            });
            self.movement.entry(key).or_insert(Movement::Mouse(MouseMovement {
                start_position: Cartesian2::ZERO,
                end_position: Cartesian2::ZERO,
            }));
        }
    }

    // ---- Event feed (port of the registered closures) ----

    /// Feeds a raw screen-space input event into the aggregator.
    ///
    /// # DEVIATION
    ///
    /// Replaces the CesiumJS DOM-listener closures: the driver calls this with
    /// the same `(type, modifiers, payload)` the corresponding listener would
    /// have received, and the matching aggregation logic runs. Payloads that do
    /// not match the event type are ignored (CesiumJS listeners simply read the
    /// fields they expect).
    pub fn handle_input_event(
        &mut self,
        type_: ScreenSpaceEventType,
        modifiers: &[KeyboardEventModifier],
        event: &ScreenSpaceInputEvent,
    ) {
        match type_ {
            ScreenSpaceEventType::Wheel => {
                if let ScreenSpaceInputEvent::Wheel(delta) = event {
                    self.handle_wheel(modifiers, *delta);
                }
            }
            ScreenSpaceEventType::PinchStart => {
                if let ScreenSpaceInputEvent::TwoPoint(two_point) = event {
                    self.handle_pinch_start(modifiers, two_point.position1, two_point.position2);
                }
            }
            ScreenSpaceEventType::PinchEnd => self.handle_pinch_end(modifiers),
            ScreenSpaceEventType::PinchMove => {
                if let ScreenSpaceInputEvent::PinchMotion(pinch) = event {
                    self.handle_pinch_move(modifiers, pinch);
                }
            }
            ScreenSpaceEventType::LeftDown
            | ScreenSpaceEventType::RightDown
            | ScreenSpaceEventType::MiddleDown => {
                let drag_type = match type_ {
                    ScreenSpaceEventType::LeftDown => CameraEventType::LeftDrag,
                    ScreenSpaceEventType::RightDown => CameraEventType::RightDrag,
                    _ => CameraEventType::MiddleDrag,
                };
                if let ScreenSpaceInputEvent::Positioned(positioned) = event {
                    self.handle_mouse_down(modifiers, drag_type, positioned.position);
                }
            }
            ScreenSpaceEventType::LeftUp
            | ScreenSpaceEventType::RightUp
            | ScreenSpaceEventType::MiddleUp => {
                let drag_type = match type_ {
                    ScreenSpaceEventType::LeftUp => CameraEventType::LeftDrag,
                    ScreenSpaceEventType::RightUp => CameraEventType::RightDrag,
                    _ => CameraEventType::MiddleDrag,
                };
                // The CesiumJS up closure ignores its own modifier and cancels
                // the base variant and every combination of the drag type.
                self.handle_mouse_up(drag_type);
            }
            ScreenSpaceEventType::MouseMove => {
                if let ScreenSpaceInputEvent::Motion(motion) = event {
                    self.handle_mouse_move(modifiers, motion);
                }
            }
            // Click / double-click are not aggregated by the camera controller.
            _ => {}
        }
    }

    /// Port of the `listenToWheel` `WHEEL` closure.
    fn handle_wheel(&mut self, modifiers: &[KeyboardEventModifier], delta: f64) {
        let key = get_key(CameraEventType::Wheel, modifiers);
        let arc_length = 7.5 * delta.to_radians();
        let now = get_timestamp();
        self.press_time.insert(key.clone(), now);
        self.release_time.insert(key.clone(), now);

        if let Some(Movement::Mouse(movement)) = self.movement.get_mut(&key) {
            movement.end_position.x = 0.0;
            movement.end_position.y = arc_length;
        }
        if let Some(last_movement) = self.last_movement.get_mut(&key) {
            last_movement.end_position.x = 0.0;
            last_movement.end_position.y = arc_length;
            last_movement.valid = true;
        }
        self.update.insert(key, false);
    }

    /// Port of the `listenToPinch` `PINCH_START` closure.
    fn handle_pinch_start(
        &mut self,
        modifiers: &[KeyboardEventModifier],
        position1: Cartesian2,
        position2: Cartesian2,
    ) {
        let key = get_key(CameraEventType::Pinch, modifiers);
        self.buttons_down += 1;
        self.is_down.insert(key.clone(), true);
        self.press_time.insert(key.clone(), get_timestamp());
        // Compute center position and store as the start point.
        let center = Cartesian2::lerp_new(&position1, &position2, 0.5);
        self.event_start_position.insert(key, center);
    }

    /// Port of the `listenToPinch` `PINCH_END` closure.
    fn handle_pinch_end(&mut self, modifiers: &[KeyboardEventModifier]) {
        let key = get_key(CameraEventType::Pinch, modifiers);
        self.buttons_down = (self.buttons_down - 1).max(0);
        self.is_down.insert(key.clone(), false);
        self.release_time.insert(key, get_timestamp());
    }

    /// Port of the `listenToPinch` `PINCH_MOVE` closure.
    fn handle_pinch_move(
        &mut self,
        modifiers: &[KeyboardEventModifier],
        pinch: &PinchMovementEvent,
    ) {
        let key = get_key(CameraEventType::Pinch, modifiers);
        if !self.is_down.get(&key).copied().unwrap_or(false) {
            return;
        }

        let update = self.update.get(&key).copied().unwrap_or(false);
        let mut clear_update = false;
        if let Some(Movement::Pinch(movement)) = self.movement.get_mut(&key) {
            if !update {
                // Aggregate several input events into a single animation frame.
                movement.distance.end_position = pinch.distance.end_position;
                movement.angle_and_height.end_position = pinch.angle_and_height.end_position;
            } else {
                movement.distance.start_position = pinch.distance.start_position;
                movement.distance.end_position = pinch.distance.end_position;
                movement.angle_and_height.start_position = pinch.angle_and_height.start_position;
                movement.angle_and_height.end_position = pinch.angle_and_height.end_position;
                clear_update = true;
                movement.prev_angle = movement.angle_and_height.start_position.x;
            }

            // Make sure our aggregation of angles does not "flip" over 360 degrees.
            let mut angle = movement.angle_and_height.end_position.x;
            let prev_angle = movement.prev_angle;
            let two_pi = std::f64::consts::TAU;
            while angle >= prev_angle + std::f64::consts::PI {
                angle -= two_pi;
            }
            while angle < prev_angle - std::f64::consts::PI {
                angle += two_pi;
            }
            movement.angle_and_height.end_position.x =
                (-angle * self.canvas_client_width) / 12.0;
            movement.angle_and_height.start_position.x =
                (-prev_angle * self.canvas_client_width) / 12.0;
        }
        if clear_update {
            self.update.insert(key, false);
        }
    }

    /// Port of the `listenMouseButtonDownUp` down closure.
    fn handle_mouse_down(
        &mut self,
        modifiers: &[KeyboardEventModifier],
        type_: CameraEventType,
        position: Cartesian2,
    ) {
        let key = get_key(type_, modifiers);
        self.buttons_down += 1;
        if let Some(last_movement) = self.last_movement.get_mut(&key) {
            last_movement.valid = false;
        }
        self.is_down.insert(key.clone(), true);
        self.press_time.insert(key.clone(), get_timestamp());
        self.event_start_position.insert(key, position);
    }

    /// Port of the `listenMouseButtonDownUp` up closure, which cancels the base
    /// variant and all seven modifier combinations of `type_`.
    fn handle_mouse_up(&mut self, type_: CameraEventType) {
        for modifiers in all_modifier_variants() {
            let cancel_key = get_key(type_, &modifiers);
            self.cancel_mouse_down_action(&cancel_key);
        }
    }

    /// Port of `cancelMouseDownAction`.
    fn cancel_mouse_down_action(&mut self, cancel_key: &str) {
        if self.is_down.get(cancel_key).copied().unwrap_or(false) {
            self.buttons_down = (self.buttons_down - 1).max(0);
        }
        self.is_down.insert(cancel_key.to_string(), false);
        self.release_time
            .insert(cancel_key.to_string(), get_timestamp());
    }

    /// Port of `refreshMouseDownStatus`, which transfers an active button-down
    /// from a previous modifier combination to the current one.
    fn refresh_mouse_down_status(
        &mut self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) {
        let current_key = get_key(type_, modifiers);
        let type_prefix = format!("{}", type_ as u8);

        // Snapshot the keys to cancel (CesiumJS iterates a copy of the entries).
        let keys_to_cancel: Vec<String> = self
            .is_down
            .iter()
            .filter(|(down_key, down_value)| {
                down_key.starts_with(&type_prefix) && **down_value && **down_key != current_key
            })
            .map(|(down_key, _)| down_key.clone())
            .collect();

        let any_button_is_down = !keys_to_cancel.is_empty();
        for down_key in keys_to_cancel {
            self.cancel_mouse_down_action(&down_key);
        }

        if !any_button_is_down {
            return;
        }

        // If a button is pressed, it is transferred to the current modifier.
        self.last_movement.entry(current_key.clone()).or_insert(LastMovement {
            start_position: Cartesian2::ZERO,
            end_position: Cartesian2::ZERO,
            valid: false,
        });
        self.buttons_down += 1;
        if let Some(last_movement) = self.last_movement.get_mut(&current_key) {
            last_movement.valid = false;
        }
        self.is_down.insert(current_key.clone(), true);
        self.press_time.insert(current_key, get_timestamp());
    }

    /// Port of the `listenMouseMove` `MOUSE_MOVE` closure.
    fn handle_mouse_move(
        &mut self,
        modifiers: &[KeyboardEventModifier],
        motion: &MotionEvent,
    ) {
        for type_ in ALL_CAMERA_EVENT_TYPES {
            let key = get_key(type_, modifiers);
            self.refresh_mouse_down_status(type_, modifiers);

            if !self.is_down.get(&key).copied().unwrap_or(false) {
                continue;
            }

            let update = self.update.get(&key).copied().unwrap_or(false);
            // Only mouse-shaped movements are aggregated here. CesiumJS would
            // also touch a pinch-shaped `_movement[key]` (adding stray
            // `endPosition` fields) if a real mouse move arrived while a pinch
            // button were down, but touch and mouse are mutually exclusive in
            // practice (mouse emulation is suppressed during touch), so that
            // duck-typed quirk is dead code and is not reproduced.
            if let Some(Movement::Mouse(movement)) = self.movement.get_mut(&key) {
                if !update {
                    movement.end_position = motion.end_position;
                } else {
                    let (start, end) = (movement.start_position, movement.end_position);
                    if let Some(last_movement) = self.last_movement.get_mut(&key) {
                        last_movement.start_position = start;
                        last_movement.end_position = end;
                        last_movement.valid = true;
                    }
                    movement.start_position = motion.start_position;
                    movement.end_position = motion.end_position;
                    self.update.insert(key.clone(), false);
                }
            }
        }

        self.current_mouse_position = motion.end_position;
    }

    // ---- Public API (port of the prototype methods / properties) ----

    /// Gets the current mouse position.
    ///
    /// Port of the `currentMousePosition` property.
    pub fn current_mouse_position(&self) -> Cartesian2 {
        self.current_mouse_position
    }

    /// Gets whether any mouse button is down, a touch has started, or the wheel
    /// has been moved.
    ///
    /// Port of the `anyButtonDown` property.
    pub fn any_button_down(&self) -> bool {
        let wheel_keys = [
            get_key(CameraEventType::Wheel, &[]),
            get_key(CameraEventType::Wheel, &[KeyboardEventModifier::Shift]),
            get_key(CameraEventType::Wheel, &[KeyboardEventModifier::Ctrl]),
            get_key(CameraEventType::Wheel, &[KeyboardEventModifier::Alt]),
        ];
        let wheel_moved = wheel_keys
            .iter()
            .any(|key| !self.update.get(key).copied().unwrap_or(false));
        self.buttons_down > 0 || wheel_moved
    }

    /// Gets if a mouse button down or touch has started and has been moved.
    ///
    /// Port of `isMoving(type, modifier)`.
    pub fn is_moving(
        &self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> bool {
        let key = get_key(type_, modifiers);
        !self.update.get(&key).copied().unwrap_or(false)
    }

    /// Gets the aggregated start and end position of the current event.
    ///
    /// Port of `getMovement(type, modifier)`.
    pub fn get_movement(
        &self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> Option<&Movement> {
        let key = get_key(type_, modifiers);
        self.movement.get(&key)
    }

    /// Gets the start and end position of the last move event (not the
    /// aggregated event).
    ///
    /// Port of `getLastMovement(type, modifier)`, which returns the movement
    /// only when it is `valid` and `undefined` otherwise.
    pub fn get_last_movement(
        &self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> Option<&LastMovement> {
        let key = get_key(type_, modifiers);
        self.last_movement.get(&key).filter(|movement| movement.valid)
    }

    /// Gets whether the mouse button is down or a touch has started.
    ///
    /// Port of `isButtonDown(type, modifier)`.
    pub fn is_button_down(
        &self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> bool {
        let key = get_key(type_, modifiers);
        self.is_down.get(&key).copied().unwrap_or(false)
    }

    /// Gets the mouse position that started the aggregation.
    ///
    /// Port of `getStartMousePosition(type, modifier)`. For the wheel type it
    /// returns the current mouse position, matching CesiumJS. An absent entry
    /// (CesiumJS `undefined`) yields [`Cartesian2::ZERO`].
    pub fn get_start_mouse_position(
        &self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> Cartesian2 {
        if type_ == CameraEventType::Wheel {
            return self.current_mouse_position;
        }
        let key = get_key(type_, modifiers);
        self.event_start_position
            .get(&key)
            .copied()
            .unwrap_or(Cartesian2::ZERO)
    }

    /// Gets the time the button was pressed or the touch was started.
    ///
    /// Port of `getButtonPressTime(type, modifier)`; the returned millisecond
    /// value models the CesiumJS `Date` (see the `press_time` field docs).
    pub fn get_button_press_time(
        &self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> Option<f64> {
        let key = get_key(type_, modifiers);
        self.press_time.get(&key).copied()
    }

    /// Gets the time the button was released or the touch was ended.
    ///
    /// Port of `getButtonReleaseTime(type, modifier)`.
    pub fn get_button_release_time(
        &self,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> Option<f64> {
        let key = get_key(type_, modifiers);
        self.release_time.get(&key).copied()
    }

    /// Signals that all of the events have been handled and the aggregator
    /// should be reset to handle new events.
    ///
    /// Port of `reset()`.
    pub fn reset(&mut self) {
        for value in self.update.values_mut() {
            *value = true;
        }
    }

    /// Returns `true` if this object was destroyed; otherwise, `false`.
    ///
    /// Port of `isDestroyed()`.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Removes mouse listeners held by this object.
    ///
    /// Port of `destroy()`. There are no DOM listeners to unregister in the
    /// headless port; the aggregation maps are cleared and the aggregator is
    /// flagged destroyed (modeling `destroyObject`).
    pub fn destroy(&mut self) {
        self.update.clear();
        self.movement.clear();
        self.last_movement.clear();
        self.is_down.clear();
        self.event_start_position.clear();
        self.press_time.clear();
        self.release_time.clear();
        self.is_destroyed = true;
    }
}
