//! Inertial camera motion (coasting after a gesture is released).
//!
//! Pure f64 domain logic with **no** Bevy / render-unit dependency. Pixel-space
//! motion is kept as-is; the pixel→radian/meter scaling is applied by the
//! adapter boundary (see [`crate::camera_controller::CameraController::coast_inertia`]).
//!
//! Maps to CesiumJS `Scene/ScreenSpaceCameraController.js` inertia helpers, and
//! to the Rust blueprint `cesium-rs/crates/cesium-scene/src/screen_space_camera_controller.rs`:
//! - `InertiaState` — blueprint L164-173 (`Spin`/`Zoom`/`Translate`/`Tilt`).
//! - `decay(time, coefficient)` — blueprint L107-113 (`exp(-tau*time)`,
//!   `tau = (1 - coefficient) * 25`).
//! - `activateInertia` — blueprint L766-786 (re-enable a state and disable the
//!   conflicting states from CesiumJS's `_inertiaDisablers`).
//! - `maintainInertia` — blueprint L796-875 (taper the last movement with the
//!   decay exponential while the button is up, so the camera coasts to a stop).
//!
//! The CesiumJS `inertiaMaxClickTimeThreshold` guard (blueprint L98-102) is
//! reproduced here as [`INERTIA_MAX_CLICK_TIME_THRESHOLD`].

use glam::DVec2;

/// If the time between mouse-down and mouse-up is not below this threshold
/// (seconds), the gesture is treated as a deliberate hold and the camera will
/// **not** coast with inertia.
///
/// CesiumJS `inertiaMaxClickTimeThreshold` (blueprint L102).
pub const INERTIA_MAX_CLICK_TIME_THRESHOLD: f64 = 0.4;

/// The motion below which coasting is considered stopped (pixels).
///
/// CesiumJS bails out of `maintainInertia` once `Cartesian2.distance(start, end)
/// < 0.5` (blueprint L865); a near-zero exponential can otherwise produce NaN or
/// an endless stream of sub-pixel updates.
pub const INERTIA_STOP_DISTANCE: f64 = 0.5;

/// `decay(time, coefficient)` — the decreasing exponential used to taper
/// inertial motion.
///
/// Returns `exp(-tau * time)` where `tau = (1 - coefficient) * 25`. A larger
/// `coefficient` (closer to `1.0`) yields a smaller `tau` and therefore a
/// slower decay (the motion coasts longer). Negative `time` clamps to `0.0`.
///
/// Faithful to blueprint L107-113.
///
/// # Arguments
/// * `time` - Elapsed time since the gesture was released (seconds).
/// * `coefficient` - The inertia coefficient in `[0, 1]` (e.g. CesiumJS
///   `inertiaSpin`/`inertiaZoom`/`inertiaTranslate`/`inertiaTilt`).
#[inline]
pub fn decay(time: f64, coefficient: f64) -> f64 {
    if time < 0.0 {
        return 0.0;
    }
    let tau = (1.0 - coefficient) * 25.0;
    (-tau * time).exp()
}

/// The four inertia movement states, replacing CesiumJS's string field names.
///
/// Maps to blueprint `InertiaState` (L164-173):
/// - [`InertiaState::Spin`] — `_lastInertiaSpinMovement`.
/// - [`InertiaState::Zoom`] — `_lastInertiaZoomMovement`.
/// - [`InertiaState::Translate`] — `_lastInertiaTranslateMovement`.
/// - [`InertiaState::Tilt`] — `_lastInertiaTiltMovement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InertiaState {
    /// Rotational spin (rotate3D / spin3D coasting).
    Spin,
    /// Zoom coasting.
    Zoom,
    /// Translate (pan) coasting.
    Translate,
    /// Tilt coasting.
    Tilt,
}

impl InertiaState {
    /// All states in declaration order.
    pub const ALL: [InertiaState; 4] = [
        InertiaState::Spin,
        InertiaState::Zoom,
        InertiaState::Translate,
        InertiaState::Tilt,
    ];

    /// Stable index used for internal storage.
    #[inline]
    const fn index(self) -> usize {
        match self {
            InertiaState::Spin => 0,
            InertiaState::Zoom => 1,
            InertiaState::Translate => 2,
            InertiaState::Tilt => 3,
        }
    }

    /// The states whose inertia CesiumJS's `_inertiaDisablers` map turns off
    /// when `self` is activated (blueprint L776-780).
    ///
    /// - `Zoom` disables `[Spin, Translate, Tilt]`.
    /// - `Tilt` disables `[Spin, Translate]`.
    /// - `Spin` / `Translate` disable nothing.
    #[inline]
    const fn disablers(self) -> &'static [InertiaState] {
        match self {
            InertiaState::Zoom => &[
                InertiaState::Spin,
                InertiaState::Translate,
                InertiaState::Tilt,
            ],
            InertiaState::Tilt => &[InertiaState::Spin, InertiaState::Translate],
            InertiaState::Spin | InertiaState::Translate => &[],
        }
    }
}

/// The `{ startPosition, endPosition, motion, inertiaEnabled }` object CesiumJS
/// stores under each `_lastInertia*Movement` field.
///
/// Maps to blueprint `InertiaMovementState` (L178-188). Positions are pixel
/// coordinates; `motion` is half of the last movement delta (blueprint L852-853).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertiaMovementState {
    /// `startPosition` — the anchor pixel of the coasting motion.
    pub start_position: DVec2,
    /// `endPosition` — `start_position + motion * decay(...)` for this frame.
    pub end_position: DVec2,
    /// `motion` — half of the last movement's delta (pixels).
    pub motion: DVec2,
    /// `inertiaEnabled` — whether this state is allowed to coast.
    pub inertia_enabled: bool,
}

impl Default for InertiaMovementState {
    fn default() -> Self {
        Self {
            start_position: DVec2::ZERO,
            end_position: DVec2::ZERO,
            motion: DVec2::ZERO,
            inertia_enabled: true,
        }
    }
}

/// A per-frame inertia sample: the timing and coefficient needed to evaluate
/// [`InertiaController::maintain`].
///
/// Bundling these keeps the public API within clippy's argument budget while
/// mirroring the blueprint's `(decayCoef, pressTime, releaseTime, now)` inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertiaSample {
    /// The inertia coefficient in `[0, 1]` passed to [`decay`].
    pub decay_coef: f64,
    /// The button press timestamp (milliseconds).
    pub press_time: f64,
    /// The button release timestamp (milliseconds).
    pub release_time: f64,
    /// The current timestamp (milliseconds).
    pub now: f64,
}

impl InertiaSample {
    /// Creates a new sample.
    pub fn new(decay_coef: f64, press_time: f64, release_time: f64, now: f64) -> Self {
        Self {
            decay_coef,
            press_time,
            release_time,
            now,
        }
    }

    /// The press→release duration in seconds (`(release - press) / 1000`).
    ///
    /// Blueprint L824.
    #[inline]
    pub fn click_threshold(&self) -> f64 {
        (self.release_time - self.press_time) / 1000.0
    }

    /// The elapsed time since release in seconds (`(now - release) / 1000`).
    ///
    /// Blueprint L831.
    #[inline]
    pub fn from_now(&self) -> f64 {
        (self.now - self.release_time) / 1000.0
    }
}

/// Holds the four [`InertiaMovementState`] slots and drives the coasting decay.
///
/// Maps to the CesiumJS `ScreenSpaceCameraController` inertia fields plus its
/// `activateInertia` / `maintainInertia` helpers. This is a pure domain object:
/// it neither reads a live aggregator nor touches the camera — callers capture
/// the last movement on release, then feed a per-frame [`InertiaSample`] and
/// apply the returned delta (see
/// [`crate::camera_controller::CameraController::coast_inertia`]).
#[derive(Debug, Clone, Default)]
pub struct InertiaController {
    states: [Option<InertiaMovementState>; 4],
}

impl InertiaController {
    /// Creates an empty controller with no captured inertia.
    pub fn new() -> Self {
        Self {
            states: [None, None, None, None],
        }
    }

    /// The stored state for `slot`, if any.
    #[inline]
    pub fn state(&self, slot: InertiaState) -> Option<&InertiaMovementState> {
        self.states[slot.index()].as_ref()
    }

    /// Mutable access to the stored state for `slot`, if any.
    #[inline]
    pub fn state_mut(&mut self, slot: InertiaState) -> Option<&mut InertiaMovementState> {
        self.states[slot.index()].as_mut()
    }

    /// Clears every stored state (e.g. on a mode change or camera reset).
    pub fn clear(&mut self) {
        self.states = [None, None, None, None];
    }

    /// Records the last movement of a gesture so it can coast on release.
    ///
    /// `motion` is stored as half of `(last_end - last_start)` (blueprint
    /// L852-853) and the state is enabled. Called by the adapter when a drag
    /// gesture ends.
    pub fn capture(&mut self, slot: InertiaState, last_start: DVec2, last_end: DVec2) {
        let state = self.states[slot.index()].get_or_insert_with(Default::default);
        state.start_position = last_start;
        state.end_position = last_end;
        state.motion = (last_end - last_start) * 0.5;
        state.inertia_enabled = true;
    }

    /// `activateInertia(controller, inertiaStateName)` (blueprint L766-786).
    ///
    /// Re-enables inertia on `slot` and disables it on the states listed in
    /// CesiumJS's `_inertiaDisablers` map. `None` (CesiumJS `undefined`, e.g.
    /// `look3D`) is a no-op. Only existing slots are mutated, exactly as the
    /// blueprint guards each write with `if let Some(...)`.
    pub fn activate(&mut self, slot: Option<InertiaState>) {
        let slot = match slot {
            Some(slot) => slot,
            None => return,
        };

        if let Some(state) = self.states[slot.index()].as_mut() {
            state.inertia_enabled = true;
        }
        for &other in slot.disablers() {
            if let Some(state) = self.states[other.index()].as_mut() {
                state.inertia_enabled = false;
            }
        }
    }

    /// Disables coasting for `slot` so it stops immediately on the next
    /// [`Self::maintain`] call (the "released ⇒ stop" path).
    pub fn deactivate(&mut self, slot: InertiaState) {
        if let Some(state) = self.states[slot.index()].as_mut() {
            state.inertia_enabled = false;
        }
    }

    /// `maintainInertia(...)` (blueprint L796-875), reduced to its pure math.
    ///
    /// Tapers the captured motion with the [`decay`] exponential and returns the
    /// delta (pixels) to apply this frame, or `None` when coasting should stop.
    /// Coasting stops when:
    /// - nothing was captured for `slot`;
    /// - the state was disabled (`inertia_enabled == false`);
    /// - the press→release duration reached [`INERTIA_MAX_CLICK_TIME_THRESHOLD`]
    ///   (a deliberate hold, not a flick);
    /// - the decayed delta is NaN or shorter than [`INERTIA_STOP_DISTANCE`].
    ///
    /// The stored state's `end_position` is updated in place (blueprint L858-860)
    /// so repeated calls observe the tapered motion.
    pub fn maintain(&mut self, slot: InertiaState, sample: &InertiaSample) -> Option<DVec2> {
        let state = self.states[slot.index()].as_mut()?;
        if !state.inertia_enabled {
            return None;
        }
        if sample.click_threshold() >= INERTIA_MAX_CLICK_TIME_THRESHOLD {
            return None;
        }

        let d = decay(sample.from_now(), sample.decay_coef);
        let delta = state.motion * d;
        state.end_position = state.start_position + delta;

        if delta.x.is_nan() || delta.y.is_nan() || delta.length() < INERTIA_STOP_DISTANCE {
            return None;
        }
        Some(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_MS: f64 = 1000.0 / 60.0;

    #[test]
    fn decay_zero_time_is_one() {
        assert!((decay(0.0, 0.9) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn decay_negative_time_clamps_to_zero() {
        assert!((decay(-1.0, 0.9)).abs() < 1e-12);
    }

    #[test]
    fn decay_matches_exponential_formula() {
        // tau = (1 - 0.8) * 25 = 5 → decay(1.0) = exp(-5).
        let expected = (-5.0_f64).exp();
        assert!((decay(1.0, 0.8) - expected).abs() < 1e-12);
    }

    #[test]
    fn decay_higher_coefficient_decays_slower() {
        // coefficient 0.95 (tau = 1.25) decays slower than 0.5 (tau = 12.5).
        assert!(decay(1.0, 0.95) > decay(1.0, 0.5));
    }

    /// Task requirement: an initial velocity `v0` decays below `0.01 * v0`
    /// after 60 frames (≈1 second at 60 fps).
    #[test]
    fn inertia_decays_below_one_percent_after_60_frames() {
        let mut controller = InertiaController::new();
        // motion = (last_end - last_start) * 0.5 = (1000, 0) → |v0| = 1000 px.
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        let v0 = 1000.0;

        // A quick flick: press and release at the same instant (threshold 0).
        let sample = InertiaSample::new(0.8, 0.0, 0.0, 60.0 * FRAME_MS);
        let delta = controller.maintain(InertiaState::Spin, &sample).expect("still coasting");

        let speed = delta.length();
        assert!(
            speed < 0.01 * v0,
            "after 60 frames speed {speed} should be < {} (1% of v0)",
            0.01 * v0
        );
        // And it has not yet been clamped to a full stop.
        assert!(speed >= INERTIA_STOP_DISTANCE);
    }

    /// Task requirement: with inertia disabled the motion stops immediately.
    #[test]
    fn maintain_stops_immediately_when_disabled() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Translate, DVec2::ZERO, DVec2::new(400.0, 0.0));
        controller.deactivate(InertiaState::Translate);

        let sample = InertiaSample::new(0.9, 0.0, 0.0, FRAME_MS);
        assert!(controller.maintain(InertiaState::Translate, &sample).is_none());
    }

    #[test]
    fn maintain_returns_none_without_capture() {
        let mut controller = InertiaController::new();
        let sample = InertiaSample::new(0.9, 0.0, 0.0, FRAME_MS);
        assert!(controller.maintain(InertiaState::Zoom, &sample).is_none());
    }

    #[test]
    fn maintain_suppressed_for_deliberate_hold() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        // Held for 0.5s ≥ INERTIA_MAX_CLICK_TIME_THRESHOLD → no coasting.
        let sample = InertiaSample::new(0.9, 0.0, 500.0, 516.0);
        assert!(controller.maintain(InertiaState::Spin, &sample).is_none());
    }

    #[test]
    fn maintain_stops_when_delta_below_stop_distance() {
        let mut controller = InertiaController::new();
        // Small motion → decays under INERTIA_STOP_DISTANCE quickly.
        controller.capture(InertiaState::Tilt, DVec2::ZERO, DVec2::new(2.0, 0.0));
        let sample = InertiaSample::new(0.5, 0.0, 0.0, 5000.0);
        assert!(controller.maintain(InertiaState::Tilt, &sample).is_none());
    }

    #[test]
    fn maintain_monotonically_decays() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(4000.0, 0.0));

        let mut previous = f64::INFINITY;
        for frame in 1..=30 {
            let sample = InertiaSample::new(0.85, 0.0, 0.0, frame as f64 * FRAME_MS);
            match controller.maintain(InertiaState::Spin, &sample) {
                Some(delta) => {
                    let speed = delta.length();
                    assert!(speed < previous, "speed must monotonically decrease");
                    previous = speed;
                }
                None => break,
            }
        }
        assert!(previous < f64::INFINITY);
    }

    #[test]
    fn activate_zoom_disables_conflicting_states() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Translate, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Tilt, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Zoom, DVec2::ZERO, DVec2::new(100.0, 0.0));

        controller.activate(Some(InertiaState::Zoom));

        assert!(controller.state(InertiaState::Zoom).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Spin).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Translate).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Tilt).unwrap().inertia_enabled);
    }

    #[test]
    fn activate_tilt_disables_spin_and_translate() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Translate, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Tilt, DVec2::ZERO, DVec2::new(100.0, 0.0));

        controller.activate(Some(InertiaState::Tilt));

        assert!(controller.state(InertiaState::Tilt).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Spin).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Translate).unwrap().inertia_enabled);
    }

    #[test]
    fn activate_spin_disables_nothing() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Zoom, DVec2::ZERO, DVec2::new(100.0, 0.0));

        controller.activate(Some(InertiaState::Spin));

        assert!(controller.state(InertiaState::Spin).unwrap().inertia_enabled);
        assert!(controller.state(InertiaState::Zoom).unwrap().inertia_enabled);
    }

    #[test]
    fn activate_none_is_noop() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.activate(None);
        assert!(controller.state(InertiaState::Spin).unwrap().inertia_enabled);
    }

    #[test]
    fn disabled_state_stops_coasting_after_activation_conflict() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        controller.capture(InertiaState::Zoom, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        // A zoom gesture wins → spin inertia is disabled.
        controller.activate(Some(InertiaState::Zoom));

        let sample = InertiaSample::new(0.9, 0.0, 0.0, FRAME_MS);
        assert!(controller.maintain(InertiaState::Spin, &sample).is_none());
        assert!(controller.maintain(InertiaState::Zoom, &sample).is_some());
    }

    #[test]
    fn clear_removes_all_states() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.clear();
        assert!(controller.state(InertiaState::Spin).is_none());
    }

    #[test]
    fn capture_stores_half_delta_motion() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Translate, DVec2::new(10.0, 20.0), DVec2::new(50.0, 80.0));
        let state = controller.state(InertiaState::Translate).unwrap();
        assert!((state.motion.x - 20.0).abs() < 1e-12);
        assert!((state.motion.y - 30.0).abs() < 1e-12);
        assert_eq!(state.start_position, DVec2::new(10.0, 20.0));
    }

    #[test]
    fn inertia_state_all_covers_four_slots() {
        assert_eq!(InertiaState::ALL.len(), 4);
        for slot in InertiaState::ALL {
            assert!(slot.index() < 4);
        }
    }

    #[test]
    fn sample_thresholds_are_in_seconds() {
        let sample = InertiaSample::new(0.9, 100.0, 300.0, 800.0);
        assert!((sample.click_threshold() - 0.2).abs() < 1e-12);
        assert!((sample.from_now() - 0.5).abs() < 1e-12);
    }
}
