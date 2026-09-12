//! Ported from `packages/engine/Source/Core/ScreenSpaceEventHandler.js`.
//!
//! # DEVIATION (mechanism, not behavior)
//!
//! The CesiumJS `ScreenSpaceEventHandler` attaches real DOM listeners
//! (`mousedown`, `mouseup`, `mousemove`, `wheel`, `touchstart`, ...) to an
//! `element` (defaulting to `document`). Those listeners translate raw browser
//! events into the typed payloads below and invoke the matching registered
//! action via `getInputAction(type, modifiers)(event)`.
//!
//! This is a headless port with no DOM. The action *registry*
//! (`setInputAction`/`getInputAction`/`removeInputAction`), the key scheme
//! (`getInputEventKey`), the typed event payloads, and the dispatch semantics
//! (`look up the action for a `(type, modifiers)` key, then invoke it with the
//! event`) are reproduced faithfully. Instead of DOM listeners, callers feed
//! synthetic events through [`ScreenSpaceEventHandler::dispatch_input_event`],
//! which performs the exact same lookup-and-invoke step. The DOM-only internal
//! bookkeeping (`_buttonDown`, `_isPinching`, `_primaryPosition`,
//! `_positions`, `_touchHoldTimer`, `_element`, ...) is therefore not modeled.

use crate::cartesian2::Cartesian2;
use crate::keyboard_event_modifier::KeyboardEventModifier;
use crate::screen_space_event_type::ScreenSpaceEventType;
use std::collections::HashMap;

/// An event that occurs at a single position on screen.
///
/// Port of the `ScreenSpaceEventHandler.PositionedEvent` typedef.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionedEvent {
    /// The position of the event.
    pub position: Cartesian2,
}

/// An event that starts at one position and ends at another.
///
/// Port of the `ScreenSpaceEventHandler.MotionEvent` typedef.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionEvent {
    /// The starting position of the event.
    pub start_position: Cartesian2,
    /// The ending position of the event.
    pub end_position: Cartesian2,
}

/// An event that occurs at two positions on screen.
///
/// Port of the `ScreenSpaceEventHandler.TwoPointEvent` typedef.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoPointEvent {
    /// The first position of the event.
    pub position1: Cartesian2,
    /// The second position of the event.
    pub position2: Cartesian2,
}

/// An event that starts at two positions on screen and moves to two other
/// positions.
///
/// Port of the `ScreenSpaceEventHandler.TwoPointMotionEvent` typedef.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoPointMotionEvent {
    /// The first position of the event.
    pub position1: Cartesian2,
    /// The second position of the event.
    pub position2: Cartesian2,
    /// The first position on the previous event.
    pub previous_position1: Cartesian2,
    /// The second position on the previous event.
    pub previous_position2: Cartesian2,
}

/// The computed pinch-movement payload delivered to a `PINCH_MOVE` action.
///
/// Port of `ScreenSpaceEventHandler`'s module-level `touchPinchMovementEvent`.
/// Unlike the raw two-point positions, `ScreenSpaceEventHandler` reduces the
/// two touch points into a `distance` (start/end) pair and an `angleAndHeight`
/// (start/end) pair before invoking the action; this is the shape the
/// `CameraEventAggregator` pinch listener consumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PinchMovementEvent {
    /// The pinch distance, carried in the `y` component (`x` is always `0.0`).
    pub distance: MotionEvent,
    /// The pinch angle (`x`, radians) and center height (`y`).
    pub angle_and_height: MotionEvent,
}

impl PinchMovementEvent {
    /// Computes the pinch-movement payload from two current and two previous
    /// touch positions.
    ///
    /// Port of the `fireTouchMoveEvents` two-touch branch: `dist` is a quarter
    /// of the distance between the two points, `angle` is `atan2(dY, dX)`, and
    /// the height is `0.125 * (position1.y + position2.y)`.
    pub fn from_positions(
        position1: Cartesian2,
        position2: Cartesian2,
        previous_position1: Cartesian2,
        previous_position2: Cartesian2,
    ) -> Self {
        let d_x = position2.x - position1.x;
        let d_y = position2.y - position1.y;
        let dist = (d_x * d_x + d_y * d_y).sqrt() * 0.25;

        let prev_d_x = previous_position2.x - previous_position1.x;
        let prev_d_y = previous_position2.y - previous_position1.y;
        let prev_dist = (prev_d_x * prev_d_x + prev_d_y * prev_d_y).sqrt() * 0.25;

        let c_y = (position2.y + position1.y) * 0.125;
        let prev_c_y = (previous_position2.y + previous_position1.y) * 0.125;
        let angle = d_y.atan2(d_x);
        let prev_angle = prev_d_y.atan2(prev_d_x);

        Self {
            distance: MotionEvent {
                start_position: Cartesian2::new(0.0, prev_dist),
                end_position: Cartesian2::new(0.0, dist),
            },
            angle_and_height: MotionEvent {
                start_position: Cartesian2::new(prev_angle, prev_c_y),
                end_position: Cartesian2::new(angle, c_y),
            },
        }
    }
}

/// The payload delivered to a registered input action.
///
/// CesiumJS uses distinct callback signatures per event type
/// (`PositionedEventCallback`, `MotionEventCallback`,
/// `TwoPointEventCallback`, `TwoPointMotionEventCallback`,
/// `WheelEventCallback`). Rust closures registered under a single key must
/// share one signature, so the possible payloads are unified into this enum.
/// The `Wheel` variant carries the raw `delta` number, matching the CesiumJS
/// wheel callback which receives a plain `number` rather than an object.
pub enum ScreenSpaceInputEvent {
    /// A single-position event (down, up, click, double-click).
    Positioned(PositionedEvent),
    /// A start/end motion event (mouse move).
    Motion(MotionEvent),
    /// A two-position event (pinch start/end).
    TwoPoint(TwoPointEvent),
    /// A two-position motion event (pinch move).
    TwoPointMotion(TwoPointMotionEvent),
    /// The computed pinch-movement payload delivered to a `PINCH_MOVE` action.
    PinchMotion(PinchMovementEvent),
    /// A mouse-wheel event carrying the wheel `delta`.
    Wheel(f64),
}

/// The action registered for an input event.
///
/// Port of the callback stored in `_inputEvents[key]`. `FnMut` is required
/// because CesiumJS actions commonly mutate captured state.
pub type InputAction = Box<dyn FnMut(&ScreenSpaceInputEvent)>;

/// Builds the registry key for a `(type, modifiers)` pair.
///
/// Port of the module-private `getInputEventKey(type, modifiers)`:
/// * no modifiers -> `"${type}"` (the numeric discriminant),
/// * with modifiers -> `"${type}+${sorted.join("+")}"`.
///
/// CesiumJS accepts `modifiers` as `undefined`, a single modifier, or an
/// array. This port models "no modifiers" with an empty slice; CesiumJS
/// call sites never pass an empty (but defined) array, so the observable keys
/// are identical.
fn get_input_event_key(
    type_: ScreenSpaceEventType,
    modifiers: &[KeyboardEventModifier],
) -> String {
    if modifiers.is_empty() {
        return format!("{}", type_ as i32);
    }

    let mut modifier_list = modifiers.to_vec();
    modifier_list.sort();
    let joined: Vec<String> = modifier_list
        .iter()
        .map(|modifier| format!("{}", *modifier as i32))
        .collect();
    format!("{}+{}", type_ as i32, joined.join("+"))
}

/// Handles user input events. Custom functions can be added to be executed on
/// when the user enters input.
///
/// Port of `ScreenSpaceEventHandler`. See the module-level DEVIATION note for
/// the headless adaptation of the DOM-listener dispatch mechanism.
pub struct ScreenSpaceEventHandler {
    /// Port of `_inputEvents` (keyed by [`get_input_event_key`]).
    input_events: HashMap<String, InputAction>,
    /// Backs [`ScreenSpaceEventHandler::is_destroyed`]. CesiumJS relies on
    /// `destroyObject` making post-destroy use throw; this flag models that.
    is_destroyed: bool,
}

impl ScreenSpaceEventHandler {
    /// The amount of time, in milliseconds, that mouse events will be disabled
    /// after receiving any touch events, such that any emulated mouse events
    /// will be ignored.
    ///
    /// Port of `ScreenSpaceEventHandler.mouseEmulationIgnoreMilliseconds`.
    pub const MOUSE_EMULATION_IGNORE_MILLISECONDS: f64 = 800.0;

    /// The amount of time, in milliseconds, before a touch on the screen
    /// becomes a touch and hold.
    ///
    /// Port of `ScreenSpaceEventHandler.touchHoldDelayMilliseconds`.
    pub const TOUCH_HOLD_DELAY_MILLISECONDS: f64 = 1500.0;

    /// Creates a new `ScreenSpaceEventHandler`.
    ///
    /// Port of the `ScreenSpaceEventHandler(element)` constructor. The `element`
    /// argument (defaulting to `document`) only selects the DOM node that
    /// listeners are attached to; it is meaningless in this headless port and
    /// is therefore not modeled (see the module-level DEVIATION note).
    pub fn new() -> Self {
        Self {
            input_events: HashMap::new(),
            is_destroyed: false,
        }
    }

    /// Sets a function to be executed on an input event.
    ///
    /// Port of `setInputAction(action, type, modifiers)`. CesiumJS throws a
    /// `DeveloperError` when `action` or `type` is undefined; the Rust type
    /// system makes both mandatory, so those debug checks are unrepresentable
    /// and are omitted.
    pub fn set_input_action<F>(
        &mut self,
        action: F,
        type_: ScreenSpaceEventType,
        modifiers: &[KeyboardEventModifier],
    ) where
        F: FnMut(&ScreenSpaceInputEvent) + 'static,
    {
        let key = get_input_event_key(type_, modifiers);
        self.input_events.insert(key, Box::new(action));
    }

    /// Returns the function to be executed on an input event.
    ///
    /// Port of `getInputAction(type, modifiers)`, returning `None` where CesiumJS
    /// returns `undefined`.
    pub fn get_input_action(
        &self,
        type_: ScreenSpaceEventType,
        modifiers: &[KeyboardEventModifier],
    ) -> Option<&InputAction> {
        let key = get_input_event_key(type_, modifiers);
        self.input_events.get(&key)
    }

    /// Removes the function to be executed on an input event.
    ///
    /// Port of `removeInputAction(type, modifiers)`.
    pub fn remove_input_action(
        &mut self,
        type_: ScreenSpaceEventType,
        modifiers: &[KeyboardEventModifier],
    ) {
        let key = get_input_event_key(type_, modifiers);
        self.input_events.remove(&key);
    }

    /// Dispatches a synthetic input event to the registered action, if any.
    ///
    /// # DEVIATION
    ///
    /// CesiumJS has no such public method: its DOM listeners perform this
    /// lookup-and-invoke internally. This headless port exposes it so that
    /// callers (and tests) can feed events, reproducing the exact dispatch step
    /// `const action = getInputAction(type, modifiers); if (defined(action)) action(event);`.
    pub fn dispatch_input_event(
        &mut self,
        type_: ScreenSpaceEventType,
        modifiers: &[KeyboardEventModifier],
        event: &ScreenSpaceInputEvent,
    ) {
        let key = get_input_event_key(type_, modifiers);
        if let Some(action) = self.input_events.get_mut(&key) {
            action(event);
        }
    }

    /// Returns `true` if this object was destroyed; otherwise, `false`.
    ///
    /// Port of `isDestroyed()`.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Removes listeners held by this object.
    ///
    /// Port of `destroy()`. `unregisterListeners(this)` is a no-op here because
    /// no DOM listeners are attached; the action registry is cleared and the
    /// handler is flagged destroyed (modeling `destroyObject`).
    pub fn destroy(&mut self) {
        self.input_events.clear();
        self.is_destroyed = true;
    }
}

impl Default for ScreenSpaceEventHandler {
    fn default() -> Self {
        Self::new()
    }
}
