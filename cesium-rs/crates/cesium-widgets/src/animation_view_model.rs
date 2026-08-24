//! Ported from `packages/widgets/Source/Animation/AnimationViewModel.js`.
//!
//! The view model for the Animation widget.
//!
//! DEVIATION: knockout observables/computed (`knockout.track`,
//! `knockout.defineProperty`) are modeled with shared interior-mutable
//! state plus on-read computed methods; knockout `getObservable` live
//! `canExecute` is modeled with [`Command::new_with_can_execute_provider`].
//! JS `arguments`-based command invocation is modeled with
//! `&[serde_json::Value]`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::binary_search::binary_search;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::developer_error::throw_developer_error;
use cesium_core::julian_date::JulianDate;

use crate::clock_view_model::ClockViewModel;
use crate::command::Command;
use crate::create_command::create_command;
use crate::toggle_button_view_model::{ToggleButtonViewModel, ToggleButtonViewModelOptions};

const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The shuttle ring angle representing real-time (1x) speed.
/// Mirrors the test-exposed `AnimationViewModel._realtimeShuttleRingAngle`.
pub const REALTIME_SHUTTLE_RING_ANGLE: f64 = 15.0;

/// The maximum deflection of the shuttle ring in either direction.
/// Mirrors the test-exposed `AnimationViewModel._maxShuttleRingAngle`.
pub const MAX_SHUTTLE_RING_ANGLE: f64 = 105.0;

/// A function that formats a date for display
/// (`AnimationViewModel.DateFormatter`).
pub type DateFormatter = Rc<dyn Fn(&JulianDate, &AnimationViewModel) -> String>;

/// A function that formats a time for display
/// (`AnimationViewModel.TimeFormatter`).
pub type TimeFormatter = Rc<dyn Fn(&JulianDate, &AnimationViewModel) -> String>;

/// The default array of known clock multipliers associated with new
/// instances of the shuttle ring
/// (`AnimationViewModel.defaultTicks`).
pub const DEFAULT_TICKS: [f64; 30] = [
    0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0,
    60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 14400.0, 21600.0, 43200.0,
    86400.0, 172800.0, 345600.0, 604800.0,
];

fn number_comparator(left: &f64, right: &f64) -> f64 {
    left - right
}

fn get_typical_multiplier_index(multiplier: f64, shuttle_ring_ticks: &[f64]) -> usize {
    let index = binary_search(shuttle_ring_ticks, &multiplier, number_comparator);
    if index < 0 {
        (!index) as usize
    } else {
        index as usize
    }
}

fn angle_to_multiplier(angle: f64, shuttle_ring_ticks: &[f64]) -> f64 {
    //Use a linear scale for -1 to 1 between -15 < angle < 15 degrees
    if angle.abs() <= REALTIME_SHUTTLE_RING_ANGLE {
        return angle / REALTIME_SHUTTLE_RING_ANGLE;
    }

    let min_p = REALTIME_SHUTTLE_RING_ANGLE;
    let max_p = MAX_SHUTTLE_RING_ANGLE;
    let min_v = 0.0;
    if angle > 0.0 {
        let max_v = shuttle_ring_ticks[shuttle_ring_ticks.len() - 1].ln();
        let scale = (max_v - min_v) / (max_p - min_p);
        return (min_v + scale * (angle - min_p)).exp();
    }

    let max_v = (-shuttle_ring_ticks[0]).ln();
    let scale = (max_v - min_v) / (max_p - min_p);
    -(min_v + scale * (angle.abs() - min_p)).exp()
}

fn multiplier_to_angle(
    multiplier: f64,
    shuttle_ring_ticks: &[f64],
    clock_view_model: &ClockViewModel,
) -> f64 {
    if clock_view_model.clock_step() == ClockStep::SystemClock {
        return REALTIME_SHUTTLE_RING_ANGLE;
    }

    if multiplier.abs() <= 1.0 {
        return multiplier * REALTIME_SHUTTLE_RING_ANGLE;
    }

    let fastest_multiplier = shuttle_ring_ticks[shuttle_ring_ticks.len() - 1];
    let multiplier = if multiplier > fastest_multiplier {
        fastest_multiplier
    } else if multiplier < -fastest_multiplier {
        -fastest_multiplier
    } else {
        multiplier
    };

    let min_p = REALTIME_SHUTTLE_RING_ANGLE;
    let max_p = MAX_SHUTTLE_RING_ANGLE;
    let min_v = 0.0;

    if multiplier > 0.0 {
        let max_v = fastest_multiplier.ln();
        let scale = (max_v - min_v) / (max_p - min_p);
        return (multiplier.ln() - min_v) / scale + min_p;
    }

    let max_v = (-shuttle_ring_ticks[0]).ln();
    let scale = (max_v - min_v) / (max_p - min_p);
    -((multiplier.abs().ln() - min_v) / scale + min_p)
}

/// Interior-mutable state shared between the view model and the command
/// closures it creates (the Rust analogue of the JS `that = this` capture
/// plus knockout-tracked properties).
struct SharedState {
    all_shuttle_ring_ticks: RefCell<Vec<f64>>,
    sorted_filtered_positive_ticks: RefCell<Vec<f64>>,
    date_formatter: RefCell<DateFormatter>,
    time_formatter: RefCell<TimeFormatter>,
    shuttle_ring_dragging: Cell<bool>,
    snap_to_ticks: Cell<bool>,
}

/// `_canAnimate` computed: whether the clock can currently animate, given
/// the clock range and current/start/stop times. Has the JS side effect of
/// clearing `shouldAnimate` when the result is false.
fn can_animate_impl(clock_view_model: &ClockViewModel, state: &SharedState) -> bool {
    let clock_range = clock_view_model.clock_range();

    if state.shuttle_ring_dragging.get() || clock_range == ClockRange::Unbounded {
        return true;
    }

    let multiplier = clock_view_model.multiplier();
    let current_time = clock_view_model.current_time();
    let start_time = clock_view_model.start_time();

    let result = if clock_range == ClockRange::LoopStop {
        JulianDate::greater_than(&current_time, &start_time)
            || (JulianDate::equals(&current_time, &start_time) && multiplier > 0.0)
    } else {
        let stop_time = clock_view_model.stop_time();
        (JulianDate::greater_than(&current_time, &start_time)
            && JulianDate::less_than(&current_time, &stop_time))
            || (JulianDate::equals(&current_time, &start_time) && multiplier > 0.0)
            || (JulianDate::equals(&current_time, &stop_time) && multiplier < 0.0)
    };

    if !result {
        clock_view_model.set_should_animate(false);
    }
    result
}

/// `_isSystemTimeAvailable` computed: whether the current system time lies
/// within the clock's start/stop range.
fn is_system_time_available_impl(clock_view_model: &ClockViewModel) -> bool {
    let clock_range = clock_view_model.clock_range();
    if clock_range == ClockRange::Unbounded {
        return true;
    }

    let system_time = clock_view_model.system_time();
    JulianDate::greater_than_or_equals(&system_time, &clock_view_model.start_time())
        && JulianDate::less_than_or_equals(&system_time, &clock_view_model.stop_time())
}

/// `_isAnimating` computed.
fn is_animating_impl(clock_view_model: &ClockViewModel, state: &SharedState) -> bool {
    clock_view_model.should_animate()
        && (can_animate_impl(clock_view_model, state) || state.shuttle_ring_dragging.get())
}

/// The view model for the Animation widget.
pub struct AnimationViewModel {
    clock_view_model: ClockViewModel,
    state: Rc<SharedState>,
    slower: Command,
    faster: Command,
    pause_view_model: ToggleButtonViewModel,
    play_reverse_view_model: ToggleButtonViewModel,
    play_forward_view_model: ToggleButtonViewModel,
    play_realtime_view_model: ToggleButtonViewModel,
}

impl AnimationViewModel {
    /// Creates a new animation view model.
    ///
    /// Mirrors `new AnimationViewModel(clockViewModel)`; the JS
    /// `clockViewModel is required.` DeveloperError for an undefined
    /// argument is mirrored by [`AnimationViewModel::try_new`].
    pub fn new(clock_view_model: ClockViewModel) -> Self {
        Self::try_new(Some(clock_view_model))
    }

    /// Creates a new animation view model from an optional clock view
    /// model, mirroring the JS undefined-argument DeveloperError check.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `clock_view_model` is `None`.
    pub fn try_new(clock_view_model: Option<ClockViewModel>) -> Self {
        #[cfg(debug_assertions)]
        if clock_view_model.is_none() {
            throw_developer_error("clockViewModel is required.");
        }
        let clock_view_model = clock_view_model.expect("clockViewModel is required.");

        let state = Rc::new(SharedState {
            all_shuttle_ring_ticks: RefCell::new(Vec::new()),
            sorted_filtered_positive_ticks: RefCell::new(Vec::new()),
            date_formatter: RefCell::new(Rc::new(default_date_formatter)),
            time_formatter: RefCell::new(Rc::new(default_time_formatter)),
            shuttle_ring_dragging: Cell::new(false),
            snap_to_ticks: Cell::new(false),
        });

        Self::set_shuttle_ring_ticks_impl(&state, &DEFAULT_TICKS);

        // pauseCommand
        let clock_for_pause = clock_view_model.clone();
        let state_for_pause = Rc::clone(&state);
        let pause_command = create_command(
            move |_| {
                if clock_for_pause.should_animate() {
                    clock_for_pause.set_should_animate(false);
                } else if can_animate_impl(&clock_for_pause, &state_for_pause) {
                    clock_for_pause.set_should_animate(true);
                }
                None
            },
            None,
        );

        let clock_for_pause_toggled = clock_view_model.clone();
        let state_for_pause_toggled = Rc::clone(&state);
        let pause_view_model = ToggleButtonViewModel::new(
            pause_command,
            ToggleButtonViewModelOptions {
                toggled_computed: Some(Box::new(move || {
                    !is_animating_impl(&clock_for_pause_toggled, &state_for_pause_toggled)
                })),
                tooltip: Some("Pause".to_string()),
                ..Default::default()
            },
        );

        // playReverseCommand
        let clock_for_reverse = clock_view_model.clone();
        let play_reverse_command = create_command(
            move |_| {
                let multiplier = clock_for_reverse.multiplier();
                if multiplier > 0.0 {
                    clock_for_reverse.set_multiplier(-multiplier);
                }
                clock_for_reverse.set_should_animate(true);
                None
            },
            None,
        );

        let clock_for_reverse_toggled = clock_view_model.clone();
        let state_for_reverse_toggled = Rc::clone(&state);
        let play_reverse_view_model = ToggleButtonViewModel::new(
            play_reverse_command,
            ToggleButtonViewModelOptions {
                toggled_computed: Some(Box::new(move || {
                    is_animating_impl(&clock_for_reverse_toggled, &state_for_reverse_toggled)
                        && clock_for_reverse_toggled.multiplier() < 0.0
                })),
                tooltip: Some("Play Reverse".to_string()),
                ..Default::default()
            },
        );

        // playForwardCommand
        let clock_for_forward = clock_view_model.clone();
        let play_forward_command = create_command(
            move |_| {
                let multiplier = clock_for_forward.multiplier();
                if multiplier < 0.0 {
                    clock_for_forward.set_multiplier(-multiplier);
                }
                clock_for_forward.set_should_animate(true);
                None
            },
            None,
        );

        let clock_for_forward_toggled = clock_view_model.clone();
        let state_for_forward_toggled = Rc::clone(&state);
        let play_forward_view_model = ToggleButtonViewModel::new(
            play_forward_command,
            ToggleButtonViewModelOptions {
                toggled_computed: Some(Box::new(move || {
                    is_animating_impl(&clock_for_forward_toggled, &state_for_forward_toggled)
                        && clock_for_forward_toggled.multiplier() > 0.0
                        && clock_for_forward_toggled.clock_step() != ClockStep::SystemClock
                })),
                tooltip: Some("Play Forward".to_string()),
                ..Default::default()
            },
        );

        // playRealtimeCommand.
        // DEVIATION: JS passes `knockout.getObservable(this,
        // "_isSystemTimeAvailable")` as a live canExecute observable; the
        // Rust port uses a computed canExecute provider with the same
        // read-time semantics.
        let clock_for_realtime = clock_view_model.clone();
        let clock_for_realtime_can_execute = clock_view_model.clone();
        let play_realtime_command = Command::new_with_can_execute_provider(
            move |_| {
                clock_for_realtime.set_clock_step(ClockStep::SystemClock);
                None
            },
            move || is_system_time_available_impl(&clock_for_realtime_can_execute),
        );

        let clock_for_realtime_toggled = clock_view_model.clone();
        let clock_for_realtime_tooltip = clock_view_model.clone();
        let play_realtime_view_model = ToggleButtonViewModel::new(
            play_realtime_command,
            ToggleButtonViewModelOptions {
                toggled_computed: Some(Box::new(move || {
                    clock_for_realtime_toggled.clock_step() == ClockStep::SystemClock
                })),
                tooltip_computed: Some(Box::new(move || {
                    if is_system_time_available_impl(&clock_for_realtime_tooltip) {
                        "Today (real-time)".to_string()
                    } else {
                        "Current time not in range".to_string()
                    }
                })),
                ..Default::default()
            },
        );

        // _slower
        let clock_for_slower = clock_view_model.clone();
        let state_for_slower = Rc::clone(&state);
        let slower = create_command(
            move |_| {
                let shuttle_ring_ticks = state_for_slower.all_shuttle_ring_ticks.borrow().clone();
                let multiplier = clock_for_slower.multiplier();
                let index = get_typical_multiplier_index(multiplier, &shuttle_ring_ticks);
                if index > 0 {
                    clock_for_slower.set_multiplier(shuttle_ring_ticks[index - 1]);
                }
                None
            },
            None,
        );

        // _faster
        let clock_for_faster = clock_view_model.clone();
        let state_for_faster = Rc::clone(&state);
        let faster = create_command(
            move |_| {
                let shuttle_ring_ticks = state_for_faster.all_shuttle_ring_ticks.borrow().clone();
                let multiplier = clock_for_faster.multiplier();
                let index = get_typical_multiplier_index(multiplier, &shuttle_ring_ticks) + 1;
                if index < shuttle_ring_ticks.len() {
                    clock_for_faster.set_multiplier(shuttle_ring_ticks[index]);
                }
                None
            },
            None,
        );

        Self {
            clock_view_model,
            state,
            slower,
            faster,
            pause_view_model,
            play_reverse_view_model,
            play_forward_view_model,
            play_realtime_view_model,
        }
    }

    /// Gets the clock view model.
    pub fn clock_view_model(&self) -> &ClockViewModel {
        &self.clock_view_model
    }

    /// Gets a command that decreases the speed of animation.
    pub fn slower(&self) -> &Command {
        &self.slower
    }

    /// Gets a command that increases the speed of animation.
    pub fn faster(&self) -> &Command {
        &self.faster
    }

    /// Gets the pause toggle button view model.
    pub fn pause_view_model(&self) -> &ToggleButtonViewModel {
        &self.pause_view_model
    }

    /// Gets the reverse toggle button view model.
    pub fn play_reverse_view_model(&self) -> &ToggleButtonViewModel {
        &self.play_reverse_view_model
    }

    /// Gets the play toggle button view model.
    pub fn play_forward_view_model(&self) -> &ToggleButtonViewModel {
        &self.play_forward_view_model
    }

    /// Gets the realtime toggle button view model.
    pub fn play_realtime_view_model(&self) -> &ToggleButtonViewModel {
        &self.play_realtime_view_model
    }

    /// Gets whether the shuttle ring is currently being dragged.
    pub fn shuttle_ring_dragging(&self) -> bool {
        self.state.shuttle_ring_dragging.get()
    }

    /// Sets whether the shuttle ring is currently being dragged.
    pub fn set_shuttle_ring_dragging(&self, value: bool) {
        self.state.shuttle_ring_dragging.set(value);
    }

    /// Gets whether dragging the shuttle ring should cause the multiplier
    /// to snap to the defined tick values rather than interpolating
    /// between them.
    pub fn snap_to_ticks(&self) -> bool {
        self.state.snap_to_ticks.get()
    }

    /// Sets whether dragging the shuttle ring should snap to tick values.
    pub fn set_snap_to_ticks(&self, value: bool) {
        self.state.snap_to_ticks.set(value);
    }

    /// Gets the string representation of the current time (`timeLabel`
    /// computed).
    pub fn time_label(&self) -> String {
        let formatter = self.state.time_formatter.borrow().clone();
        formatter(&self.clock_view_model.current_time(), self)
    }

    /// Gets the string representation of the current date (`dateLabel`
    /// computed).
    pub fn date_label(&self) -> String {
        let formatter = self.state.date_formatter.borrow().clone();
        formatter(&self.clock_view_model.current_time(), self)
    }

    /// Gets the string representation of the current multiplier
    /// (`multiplierLabel` computed).
    pub fn multiplier_label(&self) -> String {
        let clock_view_model = &self.clock_view_model;
        if clock_view_model.clock_step() == ClockStep::SystemClock {
            return "Today".to_string();
        }

        let multiplier = clock_view_model.multiplier();

        //If it's a whole number, just return it.
        if multiplier % 1.0 == 0.0 {
            return format!("{}x", multiplier as i64);
        }

        //Convert to decimal string and remove any trailing zeroes
        let mut label = format!("{multiplier:.3}");
        while label.ends_with('0') {
            label.pop();
        }
        format!("{label}x")
    }

    /// Gets the current shuttle ring angle (`shuttleRingAngle` getter).
    pub fn shuttle_ring_angle(&self) -> f64 {
        let ticks = self.state.all_shuttle_ring_ticks.borrow().clone();
        multiplier_to_angle(
            self.clock_view_model.multiplier(),
            &ticks,
            &self.clock_view_model,
        )
    }

    /// Sets the current shuttle ring angle, updating the clock multiplier
    /// accordingly (`shuttleRingAngle` setter).
    pub fn set_shuttle_ring_angle(&self, angle: f64) {
        let angle = angle.clamp(-MAX_SHUTTLE_RING_ANGLE, MAX_SHUTTLE_RING_ANGLE);
        let ticks = self.state.all_shuttle_ring_ticks.borrow().clone();

        let clock_view_model = &self.clock_view_model;
        clock_view_model.set_clock_step(ClockStep::SystemClockMultiplier);

        //If we are at the max angle, simply return the max value in either direction.
        if angle.abs() == MAX_SHUTTLE_RING_ANGLE {
            clock_view_model.set_multiplier(if angle > 0.0 {
                ticks[ticks.len() - 1]
            } else {
                ticks[0]
            });
            return;
        }

        let mut multiplier = angle_to_multiplier(angle, &ticks);
        if self.state.snap_to_ticks.get() {
            multiplier = ticks[get_typical_multiplier_index(multiplier, &ticks)];
        } else if multiplier != 0.0 {
            let positive_multiplier = multiplier.abs();

            if positive_multiplier > 100.0 {
                let num_digits = format!("{positive_multiplier:.0}").len() as i32 - 2;
                let divisor = 10.0_f64.powi(num_digits);
                multiplier = ((multiplier / divisor).round() * divisor) as i32 as f64;
            } else if positive_multiplier > REALTIME_SHUTTLE_RING_ANGLE {
                multiplier = multiplier.round();
            } else if positive_multiplier > 1.0 {
                multiplier = format!("{multiplier:.1}").parse().unwrap();
            } else if positive_multiplier > 0.0 {
                multiplier = format!("{multiplier:.2}").parse().unwrap();
            }
        }
        clock_view_model.set_multiplier(multiplier);
    }

    /// Gets the function which formats a date for display.
    pub fn date_formatter(&self) -> DateFormatter {
        self.state.date_formatter.borrow().clone()
    }

    /// Sets the function which formats a date for display.
    ///
    /// DEVIATION: the JS `dateFormatter must be a function` DeveloperError
    /// is enforced by the type system.
    pub fn set_date_formatter(&self, date_formatter: DateFormatter) {
        *self.state.date_formatter.borrow_mut() = date_formatter;
    }

    /// Gets the function which formats a time for display.
    pub fn time_formatter(&self) -> TimeFormatter {
        self.state.time_formatter.borrow().clone()
    }

    /// Sets the function which formats a time for display.
    ///
    /// DEVIATION: the JS `timeFormatter must be a function` DeveloperError
    /// is enforced by the type system.
    pub fn set_time_formatter(&self, time_formatter: TimeFormatter) {
        *self.state.time_formatter.borrow_mut() = time_formatter;
    }

    /// Gets a copy of the array of positive known clock multipliers to
    /// associate with the shuttle ring.
    pub fn get_shuttle_ring_ticks(&self) -> Vec<f64> {
        self.state.sorted_filtered_positive_ticks.borrow().clone()
    }

    /// Sets the array of positive known clock multipliers to associate
    /// with the shuttle ring.
    ///
    /// Mirrors `setShuttleRingTicks(positiveTicks)`; the JS
    /// `positiveTicks is required.` DeveloperError is mirrored by
    /// [`AnimationViewModel::try_set_shuttle_ring_ticks`] with `None`.
    pub fn set_shuttle_ring_ticks(&self, positive_ticks: &[f64]) {
        Self::set_shuttle_ring_ticks_impl(&self.state, positive_ticks);
    }

    /// Sets the shuttle ring ticks from an optional array, mirroring the
    /// JS undefined-argument DeveloperError check.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `positive_ticks` is `None`.
    pub fn try_set_shuttle_ring_ticks(&self, positive_ticks: Option<&[f64]>) {
        #[cfg(debug_assertions)]
        if positive_ticks.is_none() {
            throw_developer_error("positiveTicks is required.");
        }
        let positive_ticks = positive_ticks.expect("positiveTicks is required.");
        Self::set_shuttle_ring_ticks_impl(&self.state, positive_ticks);
    }

    fn set_shuttle_ring_ticks_impl(state: &SharedState, positive_ticks: &[f64]) {
        let mut sorted_filtered_positive_ticks = Vec::new();
        for &tick in positive_ticks {
            //filter duplicates
            if !sorted_filtered_positive_ticks.contains(&tick) {
                sorted_filtered_positive_ticks.push(tick);
            }
        }
        sorted_filtered_positive_ticks.sort_by(|left, right| {
            left.partial_cmp(right)
                .expect("shuttle ring ticks must be comparable")
        });

        let mut all_ticks = Vec::new();
        for &tick in sorted_filtered_positive_ticks.iter().rev() {
            if tick != 0.0 {
                all_ticks.push(-tick);
            }
        }
        all_ticks.extend_from_slice(&sorted_filtered_positive_ticks);

        *state.sorted_filtered_positive_ticks.borrow_mut() = sorted_filtered_positive_ticks;
        *state.all_shuttle_ring_ticks.borrow_mut() = all_ticks;
    }
}

/// The default date formatter used by new instances
/// (`AnimationViewModel.defaultDateFormatter`).
pub fn default_date_formatter(date: &JulianDate, _view_model: &AnimationViewModel) -> String {
    let gregorian_date = date.to_gregorian_date();
    format!(
        "{} {} {}",
        MONTH_NAMES[gregorian_date.month as usize - 1], gregorian_date.day, gregorian_date.year
    )
}

/// The default time formatter used by new instances
/// (`AnimationViewModel.defaultTimeFormatter`).
pub fn default_time_formatter(date: &JulianDate, view_model: &AnimationViewModel) -> String {
    let gregorian_date = date.to_gregorian_date();
    let millisecond = gregorian_date.millisecond.round() as i32;
    if view_model.clock_view_model().multiplier().abs() < 1.0 {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            gregorian_date.hour, gregorian_date.minute, gregorian_date.second, millisecond
        )
    } else {
        format!(
            "{:02}:{:02}:{:02} UTC",
            gregorian_date.hour, gregorian_date.minute, gregorian_date.second
        )
    }
}
