//! Batch 1 spec mirrors: pure-logic widget ViewModels.
//!
//! Mirrors (one `#[test]` per JS `it`):
//! - `packages/widgets/Specs/createCommandSpec.js` (6 its)
//! - `packages/widgets/Specs/ClockViewModelSpec.js` (3 its)
//! - `packages/widgets/Specs/Animation/AnimationViewModelSpec.js` (25 its)
//! - `packages/widgets/Specs/HomeButton/HomeButtonViewModelSpec.js` (6 its)
//! - `packages/widgets/Specs/FullscreenButton/FullscreenButtonViewModelSpec.js` (6 its)
//! - `packages/widgets/Specs/InfoBox/InfoBoxViewModelSpec.js` (4 its)
//! - `packages/widgets/Specs/NavigationHelpButton/NavigationHelpButtonViewModelSpec.js` (2 its)
//! - `packages/widgets/Specs/SelectionIndicator/SelectionIndicatorViewModelSpec.js` (8 its)
//! - `packages/widgets/Specs/VRButton/VRButtonViewModelSpec.js` (7 its)
//!
//! There is no `ToggleButtonViewModelSpec.js` in CesiumJS; ToggleButton
//! coverage comes through the AnimationViewModel toggle buttons.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::clock::Clock;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::julian_date::JulianDate;
use cesium_test_utils::expect_to_throw_dev_error;

use cesium_widgets::animation_view_model::{
    AnimationViewModel, MAX_SHUTTLE_RING_ANGLE, REALTIME_SHUTTLE_RING_ANGLE,
};
use cesium_widgets::clock_view_model::ClockViewModel;
use cesium_widgets::create_command::{create_command, create_command_with_can_execute_provider};
use cesium_widgets::fullscreen_button_view_model::{
    FullscreenButtonViewModel, FullscreenElement,
};
use cesium_widgets::info_box_view_model::InfoBoxViewModel;
use cesium_widgets::knockout::{MockDocument, MockDomElement};
use cesium_widgets::navigation_help_button_view_model::NavigationHelpButtonViewModel;
use cesium_widgets::observables::ObservableCell;
use cesium_widgets::selection_indicator_view_model::{SelectionIndicatorViewModel, SelectionScene};
use cesium_widgets::vr_button_view_model::{VrButtonViewModel, VrScene};
use serde_json::json;

// ===========================================================================
// Widgets/createCommand — packages/widgets/Specs/createCommandSpec.js
// ===========================================================================

/// Mirrors `it("works with default value of canExecute")`.
#[test]
fn create_command_works_with_default_value_of_can_execute() {
    let called = Rc::new(Cell::new(false));
    let called_flag = Rc::clone(&called);
    let command = create_command(
        move |_| {
            called_flag.set(true);
            Some(json!(5))
        },
        None,
    );
    assert!(command.can_execute());
    assert_eq!(command.execute(), Some(json!(5)));
    assert!(called.get());
}

/// Mirrors `it("throws when canExecute value is false")`.
#[test]
fn create_command_throws_when_can_execute_value_is_false() {
    let called = Rc::new(Cell::new(false));
    let called_flag = Rc::clone(&called);
    let command = create_command(
        move |_| {
            called_flag.set(true);
            Some(json!(5))
        },
        Some(false),
    );
    expect_to_throw_dev_error(|| {
        command.execute();
    });
    assert!(!called.get());
}

/// Mirrors `it("throws without a func parameter")`.
///
/// DEVIATION: the JS `func is required.` DeveloperError is enforced by the
/// Rust type system (`func` is a required parameter of `create_command`),
/// so there is no runtime case to exercise; kept as an ignored anchor for
/// spec traceability.
#[test]
#[ignore = "DEVIATION: enforced by the type system (func is a required parameter)"]
fn create_command_throws_without_a_func_parameter() {}

/// Mirrors `it("works with custom canExecute observable")`.
#[test]
fn create_command_works_with_custom_can_execute_observable() {
    let called = Rc::new(Cell::new(false));
    let can_execute = ObservableCell::new(false);

    let called_flag = Rc::clone(&called);
    let provider_observable = can_execute.clone();
    let command = create_command_with_can_execute_provider(
        move |_| {
            called_flag.set(true);
            Some(json!(5))
        },
        move || provider_observable.get(),
    );

    assert!(!command.can_execute());
    expect_to_throw_dev_error(|| {
        command.execute();
    });
    assert!(!called.get());

    can_execute.set(true);

    assert!(command.can_execute());
    assert_eq!(command.execute(), Some(json!(5)));
    assert!(called.get());
}

/// Mirrors `it("calls pre/post events")`.
#[test]
fn create_command_calls_pre_post_events() {
    let command = create_command(|_| Some(json!(5)), None);
    let my_arg = json!({});

    let before_calls = Rc::new(Cell::new(0));
    let before_cancel_flag = Rc::new(Cell::new(false));
    let before_args = Rc::new(Cell::new(false));
    {
        let before_calls = Rc::clone(&before_calls);
        let before_cancel_flag = Rc::clone(&before_cancel_flag);
        let before_args = Rc::clone(&before_args);
        command.before_execute.add_listener(move |info| {
            before_calls.set(before_calls.get() + 1);
            before_cancel_flag.set(info.cancel.get());
            before_args.set(info.args == vec![json!({})]);
        });
    }

    let after_calls = Rc::new(Cell::new(0));
    let after_result = Rc::new(Cell::new(false));
    {
        let after_calls = Rc::clone(&after_calls);
        let after_result = Rc::clone(&after_result);
        command.after_execute.add_listener(move |result| {
            after_calls.set(after_calls.get() + 1);
            after_result.set(*result == json!(5));
        });
    }

    assert_eq!(command.call(&[my_arg]), Some(json!(5)));

    assert_eq!(before_calls.get(), 1);
    assert!(!before_cancel_flag.get()); // cancel: false
    assert!(before_args.get()); // args: getArguments(myArg)

    assert_eq!(after_calls.get(), 1);
    assert!(after_result.get());
}

/// Mirrors `it("cancels a command if beforeExecute sets cancel to true")`.
#[test]
fn create_command_cancels_a_command_if_before_execute_sets_cancel_to_true() {
    let called = Rc::new(Cell::new(false));
    let called_flag = Rc::clone(&called);
    let command = create_command(
        move |_| {
            called_flag.set(true);
            Some(json!(5))
        },
        None,
    );
    let my_arg = json!({});

    let before_calls = Rc::new(Cell::new(0));
    {
        let before_calls = Rc::clone(&before_calls);
        command.before_execute.add_listener(move |info| {
            before_calls.set(before_calls.get() + 1);
            info.cancel.set(true);
        });
    }

    let after_calls = Rc::new(Cell::new(0));
    {
        let after_calls = Rc::clone(&after_calls);
        command.after_execute.add_listener(move |_| {
            after_calls.set(after_calls.get() + 1);
        });
    }

    assert_eq!(command.call(&[my_arg]), None);

    assert_eq!(before_calls.get(), 1);
    assert!(!called.get());
    assert_eq!(after_calls.get(), 0);
}

// ===========================================================================
// Widgets/ClockViewModel — packages/widgets/Specs/ClockViewModelSpec.js
// ===========================================================================

/// Mirrors `it("default constructor creates a clock")`.
#[test]
fn clock_view_model_default_constructor_creates_a_clock() {
    let clock_view_model = ClockViewModel::new(None);
    // `clock` is defined (always present in the Rust port).
    let _clock = clock_view_model.clock();
}

/// Mirrors `it("constructor sets expected properties")`.
#[test]
fn clock_view_model_constructor_sets_expected_properties() {
    let clock = Rc::new(RefCell::new(Clock::new(
        None, None, None, None, None, None, None, None,
    )));
    clock.borrow_mut().start_time = JulianDate::from_iso8601("2012-01-01T00:00:00").unwrap();
    clock.borrow_mut().stop_time = JulianDate::from_iso8601("2012-01-02T00:00:00").unwrap();
    clock
        .borrow_mut()
        .set_current_time(JulianDate::from_iso8601("2012-01-01T12:00:00").unwrap());
    clock.borrow_mut().set_multiplier(1.0);
    clock.borrow_mut().set_clock_step(ClockStep::TickDependent);
    clock.borrow_mut().clock_range = ClockRange::Unbounded;
    clock.borrow_mut().set_should_animate(false);

    let clock_view_model = ClockViewModel::new(Some(clock.clone()));
    assert!(Rc::ptr_eq(clock_view_model.clock(), &clock));
    assert!(JulianDate::equals(
        &clock_view_model.start_time(),
        &clock.borrow().start_time
    ));
    assert!(JulianDate::equals(
        &clock_view_model.stop_time(),
        &clock.borrow().stop_time
    ));
    assert!(JulianDate::equals(
        &clock_view_model.current_time(),
        clock.borrow().current_time()
    ));
    assert_eq!(clock_view_model.multiplier(), clock.borrow().get_multiplier());
    assert_eq!(clock_view_model.clock_step(), clock.borrow().get_clock_step());
    assert_eq!(clock_view_model.clock_range(), clock.borrow().clock_range);
    let _system_time = clock_view_model.system_time(); // toBeDefined
    assert!(!clock_view_model.should_animate());
}

/// Mirrors `it("observables are updated from the clock")`.
#[test]
fn clock_view_model_observables_are_updated_from_the_clock() {
    let clock = Rc::new(RefCell::new(Clock::new(
        None, None, None, None, None, None, None, None,
    )));
    clock.borrow_mut().start_time = JulianDate::from_iso8601("2012-01-01T00:00:00").unwrap();
    clock.borrow_mut().stop_time = JulianDate::from_iso8601("2012-01-02T00:00:00").unwrap();
    clock
        .borrow_mut()
        .set_current_time(JulianDate::from_iso8601("2012-01-01T12:00:00").unwrap());
    clock.borrow_mut().set_multiplier(1.0);
    clock.borrow_mut().set_clock_step(ClockStep::TickDependent);
    clock.borrow_mut().clock_range = ClockRange::Unbounded;
    clock.borrow_mut().set_should_animate(false);

    let clock_rc = clock.clone();
    let clock_view_model = ClockViewModel::new(Some(clock_rc));
    assert!(Rc::ptr_eq(clock_view_model.clock(), &clock));
    assert!(JulianDate::equals(
        &clock_view_model.start_time(),
        &clock.borrow().start_time
    ));
    assert!(JulianDate::equals(
        &clock_view_model.stop_time(),
        &clock.borrow().stop_time
    ));
    assert!(JulianDate::equals(
        &clock_view_model.current_time(),
        clock.borrow().current_time()
    ));
    assert_eq!(clock_view_model.multiplier(), clock.borrow().get_multiplier());
    assert_eq!(clock_view_model.clock_step(), clock.borrow().get_clock_step());
    assert_eq!(clock_view_model.clock_range(), clock.borrow().clock_range);
    assert_eq!(
        clock_view_model.should_animate(),
        clock.borrow().get_should_animate()
    );
    let _system_time = clock_view_model.system_time(); // toBeDefined

    clock.borrow_mut().start_time = JulianDate::from_iso8601("2013-01-01T00:00:00").unwrap();
    clock.borrow_mut().stop_time = JulianDate::from_iso8601("2013-01-02T00:00:00").unwrap();
    clock
        .borrow_mut()
        .set_current_time(JulianDate::from_iso8601("2013-01-01T12:00:00").unwrap());
    clock.borrow_mut().set_multiplier(2.0);
    clock
        .borrow_mut()
        .set_clock_step(ClockStep::SystemClockMultiplier);
    clock.borrow_mut().clock_range = ClockRange::Clamped;
    clock.borrow_mut().set_should_animate(true);

    // Values are stale until the clock ticks (the knockout observables are
    // only refreshed by the onTick subscription).
    assert!(!JulianDate::equals(
        &clock_view_model.start_time(),
        &clock.borrow().start_time
    ));
    assert!(!JulianDate::equals(
        &clock_view_model.stop_time(),
        &clock.borrow().stop_time
    ));
    assert!(!JulianDate::equals(
        &clock_view_model.current_time(),
        clock.borrow().current_time()
    ));
    assert_ne!(clock_view_model.multiplier(), clock.borrow().get_multiplier());
    assert_ne!(clock_view_model.clock_step(), clock.borrow().get_clock_step());
    assert_ne!(clock_view_model.clock_range(), clock.borrow().clock_range);
    assert_ne!(
        clock_view_model.should_animate(),
        clock.borrow().get_should_animate()
    );

    clock.borrow_mut().tick();

    assert!(JulianDate::equals(
        &clock_view_model.start_time(),
        &clock.borrow().start_time
    ));
    assert!(JulianDate::equals(
        &clock_view_model.stop_time(),
        &clock.borrow().stop_time
    ));
    assert!(JulianDate::equals(
        &clock_view_model.current_time(),
        clock.borrow().current_time()
    ));
    assert_eq!(clock_view_model.multiplier(), clock.borrow().get_multiplier());
    assert_eq!(clock_view_model.clock_step(), clock.borrow().get_clock_step());
    assert_eq!(clock_view_model.clock_range(), clock.borrow().clock_range);
    assert_eq!(
        clock_view_model.should_animate(),
        clock.borrow().get_should_animate()
    );
}

// ===========================================================================
// Widgets/Animation/AnimationViewModel —
// packages/widgets/Specs/Animation/AnimationViewModelSpec.js
// ===========================================================================

fn verify_paused_state(view_model: &AnimationViewModel) {
    assert!(view_model.pause_view_model().toggled());
    assert!(!view_model.play_reverse_view_model().toggled());
    assert!(!view_model.play_forward_view_model().toggled());
    assert!(!view_model.play_realtime_view_model().toggled());
}

fn verify_forward_state(view_model: &AnimationViewModel) {
    assert!(!view_model.pause_view_model().toggled());
    assert!(!view_model.play_reverse_view_model().toggled());
    assert!(view_model.play_forward_view_model().toggled());
    assert!(!view_model.play_realtime_view_model().toggled());
}

fn verify_reverse_state(view_model: &AnimationViewModel) {
    assert!(!view_model.pause_view_model().toggled());
    assert!(view_model.play_reverse_view_model().toggled());
    assert!(!view_model.play_forward_view_model().toggled());
    assert!(!view_model.play_realtime_view_model().toggled());
}

fn verify_realtime_state(view_model: &AnimationViewModel) {
    assert!(!view_model.pause_view_model().toggled());
    assert!(!view_model.play_reverse_view_model().toggled());
    assert!(!view_model.play_forward_view_model().toggled());
    assert!(view_model.play_realtime_view_model().toggled());
    assert_eq!(view_model.shuttle_ring_angle(), REALTIME_SHUTTLE_RING_ANGLE);
}

/// Mirrors `it("constructor sets expected properties")`.
#[test]
fn animation_view_model_constructor_sets_expected_properties() {
    let clock_view_model = ClockViewModel::new(None);
    let animation_view_model = AnimationViewModel::new(clock_view_model.clone());
    // JS: `animationViewModel.clockViewModel === clockViewModel` — the
    // Rust port shares the underlying clock handle.
    assert!(Rc::ptr_eq(
        animation_view_model.clock_view_model().clock(),
        clock_view_model.clock()
    ));
}

/// Mirrors `it("setTimeFormatter overrides the default formatter")`.
#[test]
fn animation_view_model_set_time_formatter_overrides_the_default_formatter() {
    let clock_view_model = ClockViewModel::new(None);
    let animation_view_model = AnimationViewModel::new(clock_view_model.clone());

    let expected_string = "My Time";
    let expected_time = clock_view_model.current_time();
    let checked = Rc::new(Cell::new(false));
    let checked_flag = Rc::clone(&checked);
    let my_custom_formatter: Rc<dyn Fn(&JulianDate, &AnimationViewModel) -> String> =
        Rc::new(move |date, _view_model| {
            assert!(JulianDate::equals(date, &expected_time));
            checked_flag.set(true);
            expected_string.to_string()
        });
    animation_view_model.set_time_formatter(my_custom_formatter.clone());

    assert_eq!(animation_view_model.time_label(), expected_string);
    assert!(checked.get());
    assert!(Rc::ptr_eq(
        &animation_view_model.time_formatter(),
        &my_custom_formatter
    ));
}

/// Mirrors `it("defaultTimeFormatter produces expected result")`.
#[test]
fn animation_view_model_default_time_formatter_produces_expected_result() {
    let clock_view_model = ClockViewModel::new(None);
    let animation_view_model = AnimationViewModel::new(clock_view_model.clone());

    let date = JulianDate::from_iso8601("2012-03-05T06:07:08.89Z").unwrap();
    let formatter = animation_view_model.time_formatter();

    clock_view_model.set_multiplier(1.0);
    let mut expected_result = "06:07:08 UTC";
    let mut result = formatter(&date, &animation_view_model);
    assert_eq!(result, expected_result);

    clock_view_model.set_multiplier(-1.0);
    expected_result = "06:07:08 UTC";
    result = formatter(&date, &animation_view_model);
    assert_eq!(result, expected_result);

    clock_view_model.set_multiplier(-0.5);
    expected_result = "06:07:08.890";
    result = formatter(&date, &animation_view_model);
    assert_eq!(result, expected_result);

    clock_view_model.set_multiplier(0.5);
    expected_result = "06:07:08.890";
    result = formatter(&date, &animation_view_model);
    assert_eq!(result, expected_result);
}

/// Mirrors `it("setDateFormatter overrides the default formatter")`.
#[test]
fn animation_view_model_set_date_formatter_overrides_the_default_formatter() {
    let clock_view_model = ClockViewModel::new(None);
    let animation_view_model = AnimationViewModel::new(clock_view_model.clone());

    let expected_string = "My Date";
    let expected_time = clock_view_model.current_time();
    let checked = Rc::new(Cell::new(false));
    let checked_flag = Rc::clone(&checked);
    let my_custom_formatter: Rc<dyn Fn(&JulianDate, &AnimationViewModel) -> String> =
        Rc::new(move |date, _view_model| {
            assert!(JulianDate::equals(date, &expected_time));
            checked_flag.set(true);
            expected_string.to_string()
        });
    animation_view_model.set_date_formatter(my_custom_formatter.clone());

    assert_eq!(animation_view_model.date_label(), expected_string);
    assert!(checked.get());
    assert!(Rc::ptr_eq(
        &animation_view_model.date_formatter(),
        &my_custom_formatter
    ));
}

/// Mirrors `it("defaultDateFormatter produces expected result")`.
#[test]
fn animation_view_model_default_date_formatter_produces_expected_result() {
    let animation_view_model = AnimationViewModel::new(ClockViewModel::new(None));
    let formatter = animation_view_model.date_formatter();

    let months = [
        ("2012-01-05T06:07:08.89Z", "Jan 5 2012"),
        ("2012-02-05T06:07:08.89Z", "Feb 5 2012"),
        ("2012-03-05T06:07:08.89Z", "Mar 5 2012"),
        ("2012-04-05T06:07:08.89Z", "Apr 5 2012"),
        ("2012-05-05T06:07:08.89Z", "May 5 2012"),
        ("2012-06-05T06:07:08.89Z", "Jun 5 2012"),
        ("2012-07-05T06:07:08.89Z", "Jul 5 2012"),
        ("2012-08-05T06:07:08.89Z", "Aug 5 2012"),
        ("2012-09-05T06:07:08.89Z", "Sep 5 2012"),
        ("2012-10-05T06:07:08.89Z", "Oct 5 2012"),
        ("2012-11-05T06:07:08.89Z", "Nov 5 2012"),
        ("2012-12-05T06:07:08.89Z", "Dec 5 2012"),
    ];
    for (iso, expected_result) in months {
        let date = JulianDate::from_iso8601(iso).unwrap();
        let result = formatter(&date, &animation_view_model);
        assert_eq!(result, expected_result);
    }
}

/// Mirrors `it("correctly formats speed label")`.
#[test]
fn animation_view_model_correctly_formats_speed_label() {
    let clock_view_model = ClockViewModel::new(None);
    let animation_view_model = AnimationViewModel::new(clock_view_model.clone());

    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_multiplier(123.1);
    assert_eq!(animation_view_model.multiplier_label(), "123.1x");

    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_multiplier(123.12);
    assert_eq!(animation_view_model.multiplier_label(), "123.12x");

    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_multiplier(123.123);
    assert_eq!(animation_view_model.multiplier_label(), "123.123x");

    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_multiplier(123.1236);
    assert_eq!(animation_view_model.multiplier_label(), "123.124x");

    clock_view_model.set_clock_step(ClockStep::SystemClock);
    assert_eq!(animation_view_model.multiplier_label(), "Today");

    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_multiplier(15.0);
    assert_eq!(animation_view_model.multiplier_label(), "15x");
}

/// Mirrors `it("pause button restores current state")`.
#[test]
fn animation_view_model_pause_button_restores_current_state() {
    let clock_view_model = ClockViewModel::new(None);
    clock_view_model.set_start_time(JulianDate::from_iso8601("2012-01-01T00:00:00").unwrap());
    clock_view_model.set_stop_time(JulianDate::from_iso8601("2012-01-02T00:00:00").unwrap());
    clock_view_model
        .set_current_time(JulianDate::from_iso8601("2012-01-01T12:00:00").unwrap());
    clock_view_model.set_multiplier(1.0);
    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_clock_range(ClockRange::Unbounded);
    clock_view_model.set_should_animate(false);

    let view_model = AnimationViewModel::new(clock_view_model.clone());

    //Starts out paused
    verify_paused_state(&view_model);

    //Toggling paused restores state when animating forward
    view_model.pause_view_model().command().execute();
    verify_forward_state(&view_model);

    //Executing paused command restores paused state
    view_model.pause_view_model().command().execute();
    verify_paused_state(&view_model);

    //Setting the multiplier to negative and unpausing animates backward
    clock_view_model.set_multiplier(-1.0);
    view_model.pause_view_model().command().execute();
    verify_reverse_state(&view_model);
}

/// Mirrors `it("animating forwards negates the multiplier if it is negative")`.
#[test]
fn animation_view_model_animating_forwards_negates_the_multiplier_if_it_is_negative() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());
    let multiplier = -100.0;
    clock_view_model.set_multiplier(multiplier);
    view_model.play_forward_view_model().command().execute();
    assert_eq!(clock_view_model.multiplier(), -multiplier);
}

/// Mirrors `it("animating backwards negates the multiplier if it is positive")`.
#[test]
fn animation_view_model_animating_backwards_negates_the_multiplier_if_it_is_positive() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());
    let multiplier = 100.0;
    clock_view_model.set_multiplier(multiplier);
    view_model.play_reverse_view_model().command().execute();
    assert_eq!(clock_view_model.multiplier(), -multiplier);
}

/// Mirrors `it("animating backwards pauses with a bounded startTime")`.
#[test]
fn animation_view_model_animating_backwards_pauses_with_a_bounded_start_time() {
    let center_time = JulianDate::from_iso8601("2012-01-01T12:00:00").unwrap();

    let clock_view_model = ClockViewModel::new(None);
    clock_view_model.set_start_time(JulianDate::from_iso8601("2012-01-01T00:00:00").unwrap());
    clock_view_model.set_stop_time(JulianDate::from_iso8601("2012-01-02T00:00:00").unwrap());
    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_current_time(center_time.clone());
    clock_view_model.set_should_animate(false);

    let view_model = AnimationViewModel::new(clock_view_model.clone());
    verify_paused_state(&view_model);

    //Play in reverse while clamped
    clock_view_model.set_multiplier(-1.0);
    clock_view_model.set_clock_range(ClockRange::Clamped);
    view_model.play_reverse_view_model().command().execute();

    verify_reverse_state(&view_model);

    //Set current time to start time
    clock_view_model.set_current_time(clock_view_model.start_time());

    //Should now be paused
    verify_paused_state(&view_model);

    //Animate in reverse again.
    clock_view_model.set_current_time(center_time);
    clock_view_model.set_clock_range(ClockRange::LoopStop);
    view_model.play_reverse_view_model().command().execute();

    verify_reverse_state(&view_model);

    //Set current time to start time
    clock_view_model.set_current_time(clock_view_model.start_time());

    //Should now be paused
    verify_paused_state(&view_model);

    //Reversing in start state while bounded should have no effect
    view_model.play_reverse_view_model().command().execute();
    verify_paused_state(&view_model);

    //Set to unbounded and reversing should be okay
    clock_view_model.set_clock_range(ClockRange::Unbounded);
    view_model.play_reverse_view_model().command().execute();
    verify_reverse_state(&view_model);
}

/// Mirrors `it("dragging shuttle ring does not pause with bounded start or stop Time")`.
#[test]
fn animation_view_model_dragging_shuttle_ring_does_not_pause_with_bounded_start_or_stop_time() {
    let center_time = JulianDate::from_iso8601("2012-01-01T12:00:00").unwrap();

    let clock_view_model = ClockViewModel::new(None);
    clock_view_model.set_start_time(JulianDate::from_iso8601("2012-01-01T00:00:00").unwrap());
    clock_view_model.set_stop_time(JulianDate::from_iso8601("2012-01-02T00:00:00").unwrap());
    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_clock_range(ClockRange::Clamped);
    clock_view_model.set_multiplier(1.0);

    let view_model = AnimationViewModel::new(clock_view_model.clone());
    verify_paused_state(&view_model);

    //Play forward while clamped
    clock_view_model.set_current_time(center_time.clone());
    view_model.play_forward_view_model().command().execute();
    verify_forward_state(&view_model);

    //Set current time to stop time, which won't stop while dragging
    view_model.set_shuttle_ring_dragging(true);
    clock_view_model.set_current_time(clock_view_model.stop_time());
    verify_forward_state(&view_model);

    //Drag complete stops.
    view_model.set_shuttle_ring_dragging(false);
    verify_paused_state(&view_model);

    //Do the same thing with start time
    clock_view_model.set_current_time(center_time);
    view_model.play_reverse_view_model().command().execute();
    verify_reverse_state(&view_model);

    view_model.set_shuttle_ring_dragging(true);
    clock_view_model.set_current_time(clock_view_model.start_time());
    verify_reverse_state(&view_model);

    //Drag complete stops.
    view_model.set_shuttle_ring_dragging(false);
    verify_paused_state(&view_model);
}

/// Mirrors `it("animating forward pauses with a bounded stopTime")`.
#[test]
fn animation_view_model_animating_forward_pauses_with_a_bounded_stop_time() {
    let center_time = JulianDate::from_iso8601("2012-01-01T12:00:00").unwrap();

    let clock_view_model = ClockViewModel::new(None);
    clock_view_model.set_start_time(JulianDate::from_iso8601("2012-01-01T00:00:00").unwrap());
    clock_view_model.set_stop_time(JulianDate::from_iso8601("2012-01-02T00:00:00").unwrap());
    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_current_time(center_time.clone());
    clock_view_model.set_should_animate(false);

    let view_model = AnimationViewModel::new(clock_view_model.clone());
    verify_paused_state(&view_model);

    //Play forward while clamped
    clock_view_model.set_multiplier(1.0);
    clock_view_model.set_clock_range(ClockRange::Clamped);
    view_model.play_forward_view_model().command().execute();

    verify_forward_state(&view_model);

    //Set current time to stop time
    clock_view_model.set_current_time(clock_view_model.stop_time());

    //Should now be paused
    verify_paused_state(&view_model);

    //Playing in stop state while bounded should have no effect
    view_model.play_forward_view_model().command().execute();
    verify_paused_state(&view_model);

    //Set to unbounded and playing should be okay
    clock_view_model.set_clock_range(ClockRange::Unbounded);
    view_model.play_forward_view_model().command().execute();
    verify_forward_state(&view_model);
}

/// Mirrors `it("slower has no effect if at the slowest speed")`.
#[test]
fn animation_view_model_slower_has_no_effect_if_at_the_slowest_speed() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());
    view_model.set_shuttle_ring_ticks(&[0.0, 1.0, 2.0]);
    let slowest_multiplier = -2.0;
    clock_view_model.set_multiplier(slowest_multiplier);
    view_model.slower().execute();
    assert_eq!(clock_view_model.multiplier(), slowest_multiplier);
}

/// Mirrors `it("faster has no effect if at the faster speed")`.
#[test]
fn animation_view_model_faster_has_no_effect_if_at_the_faster_speed() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());
    view_model.set_shuttle_ring_ticks(&[0.0, 1.0, 2.0]);
    let fastest_multiplier = 2.0;
    clock_view_model.set_multiplier(fastest_multiplier);
    view_model.faster().execute();
    assert_eq!(clock_view_model.multiplier(), fastest_multiplier);
}

/// Mirrors `it("slower and faster cycle through defined multipliers")`.
#[test]
fn animation_view_model_slower_and_faster_cycle_through_defined_multipliers() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());

    let multipliers = view_model.get_shuttle_ring_ticks();
    let length = multipliers.len();

    //Start at slowest speed
    clock_view_model.set_multiplier(multipliers[0]);

    //Cycle through them all with faster
    for i in 1..length {
        view_model.faster().execute();
        assert_eq!(clock_view_model.multiplier(), multipliers[i]);
    }

    //We should be at the fastest time now.
    assert_eq!(clock_view_model.multiplier(), multipliers[length - 1]);

    //Cycle through them all with slower
    for i in (0..=length - 2).rev() {
        view_model.slower().execute();
        assert_eq!(clock_view_model.multiplier(), multipliers[i]);
    }

    //We should be at the slowest time now.
    assert_eq!(clock_view_model.multiplier(), multipliers[0]);
}

/// Mirrors `it("Realtime canExecute and tooltip depends on clock settings")`.
#[test]
fn animation_view_model_realtime_can_execute_and_tooltip_depend_on_clock_settings() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());

    //UNBOUNDED but available when start/stop time does not include realtime
    clock_view_model.set_system_time(JulianDate::now());
    clock_view_model.set_clock_range(ClockRange::Unbounded);
    clock_view_model.set_start_time(JulianDate::add_seconds(&clock_view_model.system_time(), -60.0));
    clock_view_model.set_stop_time(JulianDate::add_seconds(&clock_view_model.system_time(), -30.0));
    assert!(view_model.play_realtime_view_model().command().can_execute());
    assert_eq!(
        view_model.play_realtime_view_model().tooltip(),
        "Today (real-time)"
    );

    //CLAMPED but unavailable when start/stop time does not include realtime
    clock_view_model.set_clock_range(ClockRange::Clamped);
    clock_view_model.set_start_time(JulianDate::add_seconds(&clock_view_model.system_time(), -60.0));
    clock_view_model.set_stop_time(JulianDate::add_seconds(&clock_view_model.system_time(), -30.0));
    assert!(!view_model.play_realtime_view_model().command().can_execute());
    assert_eq!(
        view_model.play_realtime_view_model().tooltip(),
        "Current time not in range"
    );

    //CLAMPED but available when start/stop time includes realtime
    clock_view_model.set_clock_range(ClockRange::Clamped);
    clock_view_model.set_start_time(JulianDate::add_seconds(&clock_view_model.system_time(), -60.0));
    clock_view_model.set_stop_time(JulianDate::add_seconds(&clock_view_model.system_time(), 60.0));
    assert!(view_model.play_realtime_view_model().command().can_execute());
    assert_eq!(
        view_model.play_realtime_view_model().tooltip(),
        "Today (real-time)"
    );

    //LOOP_STOP but unavailable when start/stop time does not include realtime
    clock_view_model.set_clock_range(ClockRange::LoopStop);
    clock_view_model.set_start_time(JulianDate::add_seconds(&clock_view_model.system_time(), -60.0));
    clock_view_model.set_stop_time(JulianDate::add_seconds(&clock_view_model.system_time(), -30.0));
    assert!(!view_model.play_realtime_view_model().command().can_execute());
    assert_eq!(
        view_model.play_realtime_view_model().tooltip(),
        "Current time not in range"
    );

    //LOOP_STOP but available when start/stop time includes realtime
    clock_view_model.set_clock_range(ClockRange::LoopStop);
    clock_view_model.set_start_time(JulianDate::add_seconds(&clock_view_model.system_time(), -60.0));
    clock_view_model.set_stop_time(JulianDate::add_seconds(&clock_view_model.system_time(), 60.0));
    assert!(view_model.play_realtime_view_model().command().can_execute());
    assert_eq!(
        view_model.play_realtime_view_model().tooltip(),
        "Today (real-time)"
    );
}

/// Mirrors `it("User action breaks out of realtime mode")`.
#[test]
fn animation_view_model_user_action_breaks_out_of_realtime_mode() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());
    clock_view_model.set_clock_step(ClockStep::TickDependent);
    clock_view_model.set_clock_range(ClockRange::Unbounded);

    view_model.play_realtime_view_model().command().execute();
    verify_realtime_state(&view_model);
    assert_eq!(clock_view_model.multiplier(), 1.0);

    //Pausing breaks realtime state
    view_model.pause_view_model().command().execute();
    verify_paused_state(&view_model);
    assert_eq!(clock_view_model.multiplier(), 1.0);

    view_model.play_realtime_view_model().command().execute();
    verify_realtime_state(&view_model);

    //Reverse breaks realtime state
    view_model.play_reverse_view_model().command().execute();
    verify_reverse_state(&view_model);
    assert_eq!(clock_view_model.multiplier(), -1.0);

    view_model.play_realtime_view_model().command().execute();
    verify_realtime_state(&view_model);

    //Play does not break realtime state
    view_model.play_forward_view_model().command().execute();
    verify_realtime_state(&view_model);
    assert_eq!(clock_view_model.multiplier(), 1.0);

    view_model.play_realtime_view_model().command().execute();
    verify_realtime_state(&view_model);

    //Shuttle ring change breaks realtime state
    view_model.set_shuttle_ring_angle(view_model.shuttle_ring_angle() + 1.0);
    verify_forward_state(&view_model);
}

/// Mirrors `it("real time mode toggles off but not back on when shouldAnimate changes")`.
#[test]
fn animation_view_model_real_time_mode_toggles_off_but_not_back_on_when_should_animate_changes() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());

    view_model.play_realtime_view_model().command().execute();
    verify_realtime_state(&view_model);

    clock_view_model.set_should_animate(false);
    assert!(!view_model.play_realtime_view_model().toggled());

    clock_view_model.set_should_animate(true);
    assert!(!view_model.play_realtime_view_model().toggled());
}

/// Mirrors `it("Shuttle ring angles set expected multipliers")`.
#[test]
fn animation_view_model_shuttle_ring_angles_set_expected_multipliers() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());

    let shuttle_ring_ticks = view_model.get_shuttle_ring_ticks();
    let max_multiplier = shuttle_ring_ticks[shuttle_ring_ticks.len() - 1];
    let min_multiplier = -max_multiplier;

    //Max angle should produce max speed
    view_model.set_shuttle_ring_angle(MAX_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), max_multiplier);

    //Min angle should produce min speed
    view_model.set_shuttle_ring_angle(-MAX_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), min_multiplier);

    //REALTIME_SHUTTLE_RING_ANGLE degrees is always 1x
    view_model.set_shuttle_ring_angle(REALTIME_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), 1.0);

    view_model.set_shuttle_ring_angle(-REALTIME_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), -1.0);

    //For large values, the shuttleRingAngle should always round to the first two digits.
    view_model.set_shuttle_ring_angle(45.0);
    assert_eq!(clock_view_model.multiplier(), 85.0);

    view_model.set_shuttle_ring_angle(-90.0);
    assert_eq!(clock_view_model.multiplier(), -66000.0);

    view_model.set_shuttle_ring_angle(0.0);
    assert_eq!(clock_view_model.multiplier(), 0.0);
}

/// Mirrors `it("Shuttle ring angles set expected multipliers when snapping to ticks")`.
#[test]
fn animation_view_model_shuttle_ring_angles_set_expected_multipliers_when_snapping_to_ticks() {
    let clock_view_model = ClockViewModel::new(None);
    let view_model = AnimationViewModel::new(clock_view_model.clone());
    view_model.set_snap_to_ticks(true);

    let shuttle_ring_ticks = view_model.get_shuttle_ring_ticks();
    let max_multiplier = shuttle_ring_ticks[shuttle_ring_ticks.len() - 1];
    let min_multiplier = -max_multiplier;

    //Max angle should produce max speed
    view_model.set_shuttle_ring_angle(MAX_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), max_multiplier);

    //Min angle should produce min speed
    view_model.set_shuttle_ring_angle(-MAX_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), min_multiplier);

    //REALTIME_SHUTTLE_RING_ANGLE degrees is always 1x
    view_model.set_shuttle_ring_angle(REALTIME_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), 1.0);

    view_model.set_shuttle_ring_angle(-REALTIME_SHUTTLE_RING_ANGLE);
    assert_eq!(clock_view_model.multiplier(), -1.0);

    //For large values, the shuttleRingAngle should always round to the first two digits.
    view_model.set_shuttle_ring_angle(45.0);
    assert_eq!(clock_view_model.multiplier(), 120.0);

    view_model.set_shuttle_ring_angle(-90.0);
    assert_eq!(clock_view_model.multiplier(), -43200.0);

    view_model.set_shuttle_ring_angle(0.0);
    assert_eq!(
        clock_view_model.multiplier(),
        cesium_widgets::animation_view_model::DEFAULT_TICKS[0]
    );
}

/// Mirrors `it("throws when constructed without arguments")`.
#[test]
fn animation_view_model_throws_when_constructed_without_arguments() {
    expect_to_throw_dev_error(|| {
        let _ = AnimationViewModel::try_new(None);
    });
}

/// Mirrors `it("setting timeFormatter throws with non-function")`.
///
/// DEVIATION: the JS `timeFormatter must be a function` DeveloperError is
/// enforced by the Rust type system (`set_time_formatter` takes a typed
/// `TimeFormatter`), so there is no runtime case to exercise; kept as an
/// ignored anchor for spec traceability.
#[test]
#[ignore = "DEVIATION: enforced by the type system (TimeFormatter is a required function type)"]
fn animation_view_model_setting_time_formatter_throws_with_non_function() {}

/// Mirrors `it("setting dateFormatter throws with non-function")`.
///
/// DEVIATION: the JS `dateFormatter must be a function` DeveloperError is
/// enforced by the Rust type system (`set_date_formatter` takes a typed
/// `DateFormatter`), so there is no runtime case to exercise; kept as an
/// ignored anchor for spec traceability.
#[test]
#[ignore = "DEVIATION: enforced by the type system (DateFormatter is a required function type)"]
fn animation_view_model_setting_date_formatter_throws_with_non_function() {}

/// Mirrors `it("setting shuttleRingTicks throws with undefined")`.
#[test]
fn animation_view_model_setting_shuttle_ring_ticks_throws_with_undefined() {
    let view_model = AnimationViewModel::new(ClockViewModel::new(None));
    expect_to_throw_dev_error(|| {
        view_model.try_set_shuttle_ring_ticks(None);
    });
}

/// Mirrors `it("returns a copy of shuttleRingTicks when getting")`.
#[test]
fn animation_view_model_returns_a_copy_of_shuttle_ring_ticks_when_getting() {
    let view_model = AnimationViewModel::new(ClockViewModel::new(None));
    let original_ticks = [0.0, 1.0, 2.0];
    view_model.set_shuttle_ring_ticks(&original_ticks);

    let mut ticks = view_model.get_shuttle_ring_ticks();
    ticks.push(99.0);
    ticks[0] = -99.0;
    assert_eq!(view_model.get_shuttle_ring_ticks(), vec![0.0, 1.0, 2.0]);
}

/// Mirrors `it("sorts shuttleRingTicks when setting")`.
#[test]
fn animation_view_model_sorts_shuttle_ring_ticks_when_setting() {
    let view_model = AnimationViewModel::new(ClockViewModel::new(None));
    let ticks = [4.0, 0.0, 8.0, 2.0];

    view_model.set_shuttle_ring_ticks(&ticks);
    assert_eq!(view_model.get_shuttle_ring_ticks(), vec![0.0, 2.0, 4.0, 8.0]);
}

// ===========================================================================
// Widgets/HomeButton/HomeButtonViewModel —
// packages/widgets/Specs/HomeButton/HomeButtonViewModelSpec.js
// ===========================================================================

use std::cell::RefCell;

use cesium_widgets::home_button_view_model::{HomeButtonViewModel, HomeCamera};

/// Mock camera injected in place of `scene.camera` (DEVIATION: the JS spec
/// uses a real WebGL Scene; the Rust port uses trait dependency injection).
#[derive(Default)]
struct MockHomeCamera {
    fly_home_calls: Rc<Cell<i32>>,
    last_duration: Rc<RefCell<Option<f64>>>,
}

impl HomeCamera for MockHomeCamera {
    fn fly_home(&self, duration: Option<f64>) {
        self.fly_home_calls.set(self.fly_home_calls.get() + 1);
        *self.last_duration.borrow_mut() = duration;
    }
}

/// Mirrors `it("constructor sets default values")`.
#[test]
fn home_button_view_model_constructor_sets_default_values() {
    let camera: Rc<dyn HomeCamera> = Rc::new(MockHomeCamera::default());
    let view_model = HomeButtonViewModel::new(Rc::clone(&camera), None);
    // JS: `expect(viewModel.scene).toBe(scene)` — the injected camera
    // handle is shared, mirroring the JS reference identity check.
    assert!(Rc::ptr_eq(view_model.camera(), &camera));
}

/// Mirrors `it("throws if scene is undefined")`.
#[test]
fn home_button_view_model_throws_if_scene_is_undefined() {
    expect_to_throw_dev_error(|| {
        let _ = HomeButtonViewModel::try_new(None, None);
    });
}

// The remaining "works in ..." specs are sanity checks to make sure the
// code executes; the actual position of the camera at the end of the
// command is tied to the implementation of various camera features. The
// 3D case runs through the engine wiring (`impl HomeCamera for Scene`);
// the morph-dependent cases stay ignored: the Scene port completes
// morphs synchronously (DEVIATION), so the JS transition-timing sanity
// they guard has no headless analogue yet.

/// Mirrors `it("works in 3D")`.
#[test]
fn home_button_view_model_works_in_3d() {
    // JS: `scene.render(); viewModel.command();`
    let mut scene = cesium_scene::scene::Scene::new();
    scene.render(&JulianDate::now());
    let scene = Rc::new(scene) as Rc<dyn HomeCamera>;
    let view_model = HomeButtonViewModel::new(Rc::clone(&scene), None);
    view_model.command().execute();
}

/// Mirrors `it("works in 2D")`.
#[test]
#[ignore = "morph-dependent sanity kept ignored: the Scene port morphs synchronously (DEVIATION)"]
fn home_button_view_model_works_in_2d() {}

/// Mirrors `it("works in Columbus View")`.
#[test]
#[ignore = "morph-dependent sanity kept ignored: the Scene port morphs synchronously (DEVIATION)"]
fn home_button_view_model_works_in_columbus_view() {}

/// Mirrors `it("works while morphing")`.
#[test]
#[ignore = "morph-dependent sanity kept ignored: the Scene port morphs synchronously (DEVIATION)"]
fn home_button_view_model_works_while_morphing() {}

// ===========================================================================
// Widgets/FullscreenButton/FullscreenButtonViewModel —
// packages/widgets/Specs/FullscreenButton/FullscreenButtonViewModelSpec.js
// ===========================================================================

use cesium_widgets::fullscreen_button_view_model::FullscreenSource;

/// Mock of the browser `Fullscreen` static facade (DEVIATION: the JS
/// reads the engine `Fullscreen` statics directly; the widget takes an
/// injected source — the engine facade itself is wired through
/// `impl FullscreenSource for cesium_core::fullscreen::Fullscreen`).
#[derive(Default)]
struct MockFullscreenSource {
    enabled: Cell<bool>,
    fullscreen: Cell<bool>,
    request_calls: Cell<i32>,
    exit_calls: Cell<i32>,
}

impl MockFullscreenSource {
    fn enabled_source() -> Rc<Self> {
        Rc::new(Self {
            enabled: Cell::new(true),
            ..Default::default()
        })
    }
}

impl FullscreenSource for MockFullscreenSource {
    fn enabled(&self) -> bool {
        self.enabled.get()
    }

    fn fullscreen(&self) -> bool {
        self.fullscreen.get()
    }

    fn request_fullscreen(&self) {
        self.request_calls.set(self.request_calls.get() + 1);
    }

    fn exit_fullscreen(&self) {
        self.exit_calls.set(self.exit_calls.get() + 1);
    }
}

/// Mirrors `it("constructor sets default values")`.
#[test]
fn fullscreen_button_view_model_constructor_sets_default_values() {
    let mut view_model =
        FullscreenButtonViewModel::new(MockFullscreenSource::enabled_source(), None);
    // JS: `expect(viewModel.fullscreenElement).toBe(document.body)`.
    assert_eq!(view_model.fullscreen_element(), FullscreenElement::Body);
    assert!(!view_model.is_destroyed());
    view_model.destroy();
    assert!(view_model.is_destroyed());
}

/// Mirrors `it("constructor sets expected values")`.
///
/// DEVIATION: the JS passes a live `Element`; the Rust port models the
/// element identity with [`FullscreenElement`].
#[test]
fn fullscreen_button_view_model_constructor_sets_expected_values() {
    let mut view_model = FullscreenButtonViewModel::new(
        MockFullscreenSource::enabled_source(),
        Some(FullscreenElement::Id("testElement".to_string())),
    );
    assert_eq!(
        view_model.fullscreen_element(),
        FullscreenElement::Id("testElement".to_string())
    );
    view_model.destroy();
}

/// Mirrors `it("constructor can take an element id")`.
#[test]
fn fullscreen_button_view_model_constructor_can_take_an_element_id() {
    let view_model = FullscreenButtonViewModel::new(
        MockFullscreenSource::enabled_source(),
        Some(FullscreenElement::Id("testElement".to_string())),
    );
    assert_eq!(
        view_model.fullscreen_element(),
        FullscreenElement::Id("testElement".to_string())
    );
}

/// Mirrors `it("isFullscreenEnabled work as expected")`.
#[test]
fn fullscreen_button_view_model_is_fullscreen_enabled_works_as_expected() {
    let source = MockFullscreenSource::enabled_source();
    let view_model = FullscreenButtonViewModel::new(Rc::clone(&source) as Rc<dyn FullscreenSource>, None);
    // JS: `expect(viewModel.isFullscreenEnabled).toEqual(Fullscreen.enabled)`;
    // the injected source stands in for the `Fullscreen.enabled` static.
    assert_eq!(view_model.is_fullscreen_enabled(), source.enabled());
    view_model.set_is_fullscreen_enabled(false);
    assert!(!view_model.is_fullscreen_enabled());
}

/// Mirrors `it("can get and set fullscreenElement")`.
#[test]
fn fullscreen_button_view_model_can_get_and_set_fullscreen_element() {
    let view_model =
        FullscreenButtonViewModel::new(MockFullscreenSource::enabled_source(), None);
    assert_ne!(
        view_model.fullscreen_element(),
        FullscreenElement::Id("testElement".to_string())
    );
    view_model.set_fullscreen_element(FullscreenElement::Id("testElement".to_string()));
    assert_eq!(
        view_model.fullscreen_element(),
        FullscreenElement::Id("testElement".to_string())
    );
}

/// Mirrors `it("throws is setting fullscreenElement is not an Element")`.
#[test]
fn fullscreen_button_view_model_throws_if_setting_fullscreen_element_is_not_an_element() {
    let view_model =
        FullscreenButtonViewModel::new(MockFullscreenSource::enabled_source(), None);
    expect_to_throw_dev_error(|| {
        view_model.try_set_fullscreen_element(None);
    });
}

// ===========================================================================
// Widgets/InfoBox/InfoBoxViewModel —
// packages/widgets/Specs/InfoBox/InfoBoxViewModelSpec.js
// ===========================================================================

/// Mirrors `it("constructor sets expected values")`.
#[test]
fn info_box_view_model_constructor_sets_expected_values() {
    let view_model = InfoBoxViewModel::new();
    assert!(!view_model.enable_camera());
    assert!(!view_model.is_camera_tracking());
    assert!(!view_model.show_info());
    // cameraClicked/closeClicked are defined.
    let _camera_clicked = view_model.camera_clicked();
    let _close_clicked = view_model.close_clicked();
    // maxHeightOffset(0) is defined.
    let _max_height_offset = view_model.max_height_offset(0.0);
}

/// Mirrors `it("sets description")`.
#[test]
fn info_box_view_model_sets_description() {
    let safe_string = "<p>This is a test.</p>";
    let mut view_model = InfoBoxViewModel::new();
    view_model.set_description(safe_string);
    assert_eq!(view_model.description(), safe_string);
}

/// Mirrors `it("indicates missing description")`.
#[test]
fn info_box_view_model_indicates_missing_description() {
    let mut view_model = InfoBoxViewModel::new();
    assert!(view_model.bodyless());
    view_model.set_description("Testing");
    assert!(!view_model.bodyless());
}

/// Mirrors `it("camera icon changes when tracking is not available, unless due to active tracking")`.
#[test]
fn info_box_view_model_camera_icon_changes_when_tracking_is_not_available() {
    let mut view_model = InfoBoxViewModel::new();
    view_model.set_enable_camera(true);
    view_model.set_is_camera_tracking(false);
    let enabled_tracking_path = view_model.camera_icon_path();

    view_model.set_enable_camera(false);
    view_model.set_is_camera_tracking(false);
    assert_ne!(view_model.camera_icon_path(), enabled_tracking_path);

    let disable_tracking_path = view_model.camera_icon_path();

    view_model.set_enable_camera(true);
    view_model.set_is_camera_tracking(true);
    assert_eq!(view_model.camera_icon_path(), disable_tracking_path);

    view_model.set_enable_camera(false);
    view_model.set_is_camera_tracking(true);
    assert_eq!(view_model.camera_icon_path(), disable_tracking_path);
}

// ===========================================================================
// Widgets/NavigationHelpButton/NavigationHelpButtonViewModel —
// packages/widgets/Specs/NavigationHelpButton/NavigationHelpButtonViewModelSpec.js
// ===========================================================================

/// Mirrors `it("Can construct")`.
#[test]
fn navigation_help_button_view_model_can_construct() {
    let view_model = NavigationHelpButtonViewModel::new();
    assert!(!view_model.show_instructions());
}

/// Mirrors `it("invoking command toggles showing")`.
#[test]
fn navigation_help_button_view_model_invoking_command_toggles_showing() {
    let view_model = NavigationHelpButtonViewModel::new();
    assert!(!view_model.show_instructions());

    view_model.command().execute();
    assert!(view_model.show_instructions());

    view_model.command().execute();
    assert!(!view_model.show_instructions());
}

// ===========================================================================
// Widgets/SelectionIndicator/SelectionIndicatorViewModel —
// packages/widgets/Specs/SelectionIndicator/SelectionIndicatorViewModelSpec.js
// ===========================================================================

use cesium_core::cartesian2::Cartesian2;
use cesium_scene::tween_collection::{TweenCollection, TweenOptions};

/// Mock scene injected in place of the JS WebGL Scene (DEVIATION: the JS
/// spec uses `createScene()`; the Rust port uses trait dependency
/// injection). Hosts a real [`TweenCollection`] so the `_scale`
/// animations run through the same `scene.tweens` path as the engine.
#[derive(Default)]
struct MockSelectionScene {
    tweens: Rc<RefCell<TweenCollection>>,
}

impl SelectionScene for MockSelectionScene {
    fn world_to_window_coordinates(&self, _position: &Cartesian3) -> Option<Cartesian2> {
        None
    }

    fn add_tween(&self, options: TweenOptions) -> u64 {
        self.tweens.borrow_mut().add(options)
    }
}

/// Mirrors the spec-level shared DOM fixtures:
/// `selectionIndicatorElement` is a 20x20 div inside `container`.
fn selection_indicator_fixtures() -> (MockDomElement, MockDomElement) {
    let selection_indicator_element = MockDomElement::new("div").with_client_size(20, 20);
    let container = MockDomElement::new("div")
        .with_parent_client_size(100, 100);
    (selection_indicator_element, container)
}

/// Mirrors `it("constructor sets expected values")`.
#[test]
fn selection_indicator_view_model_constructor_sets_expected_values() {
    let (selection_indicator_element, container) = selection_indicator_fixtures();
    let scene: Rc<dyn SelectionScene> = Rc::new(MockSelectionScene::default());
    let view_model = SelectionIndicatorViewModel::new(
        Rc::clone(&scene),
        selection_indicator_element.clone(),
        container.clone(),
    );
    assert!(Rc::ptr_eq(view_model.scene(), &scene));
    assert_eq!(
        view_model.selection_indicator_element(),
        &selection_indicator_element
    );
    assert_eq!(view_model.container(), &container);
    // computeScreenSpacePosition is defined.
    let _compute_screen_space_position = view_model.compute_screen_space_position();
}

/// Mirrors `it("throws if scene is undefined")`.
#[test]
fn selection_indicator_view_model_throws_if_scene_is_undefined() {
    let (selection_indicator_element, container) = selection_indicator_fixtures();
    expect_to_throw_dev_error(|| {
        let _ = SelectionIndicatorViewModel::try_new(
            None,
            Some(selection_indicator_element),
            Some(container),
        );
    });
}

/// Mirrors `it("throws if selectionIndicatorElement is undefined")`.
#[test]
fn selection_indicator_view_model_throws_if_selection_indicator_element_is_undefined() {
    let (_, container) = selection_indicator_fixtures();
    let scene: Rc<dyn SelectionScene> = Rc::new(MockSelectionScene::default());
    expect_to_throw_dev_error(|| {
        let _ = SelectionIndicatorViewModel::try_new(Some(scene), None, Some(container));
    });
}

/// Mirrors `it("throws if container is undefined")`.
#[test]
fn selection_indicator_view_model_throws_if_container_is_undefined() {
    let (selection_indicator_element, _) = selection_indicator_fixtures();
    let scene: Rc<dyn SelectionScene> = Rc::new(MockSelectionScene::default());
    expect_to_throw_dev_error(|| {
        let _ = SelectionIndicatorViewModel::try_new(
            Some(scene),
            Some(selection_indicator_element),
            None,
        );
    });
}

/// Mirrors `it("can animate selection element")`.
///
/// The animations run through the mock's `scene.tweens` (mirroring the
/// JS `scene.tweens.addProperty`); advancing the collection drives the
/// view model `_scale` to the tween stop values.
#[test]
fn selection_indicator_view_model_can_animate_selection_element() {
    let (selection_indicator_element, container) = selection_indicator_fixtures();
    let scene = Rc::new(MockSelectionScene::default());
    let view_model = SelectionIndicatorViewModel::new(
        Rc::clone(&scene) as Rc<dyn SelectionScene>,
        selection_indicator_element,
        container,
    );
    view_model.animate_appear();
    view_model.animate_depart();
    // JS: two `scene.tweens.addProperty` calls.
    assert_eq!(scene.tweens.borrow().len(), 2);

    // Advancing past the 0.8s duration completes both tweens; the
    // depart tween (current -> 1.5) runs last, mirroring the JS.
    let start = JulianDate::now();
    scene.tweens.borrow_mut().update(&start);
    scene
        .tweens
        .borrow_mut()
        .update(&JulianDate::add_seconds_new(&start, 1.0));
    assert_eq!(view_model.scale(), 1.5);
    assert_eq!(view_model.transform(), "scale(1.5)");
}

/// Mirrors `it("can use custom screen space positions")`.
#[test]
fn selection_indicator_view_model_can_use_custom_screen_space_positions() {
    let (selection_indicator_element, container) = selection_indicator_fixtures();
    let scene: Rc<dyn SelectionScene> = Rc::new(MockSelectionScene::default());
    let mut view_model = SelectionIndicatorViewModel::new(
        scene,
        selection_indicator_element,
        container,
    );
    view_model.set_show_selection(true);
    view_model.set_position(Some(Cartesian3::new(1.0, 2.0, 3.0)));
    view_model.set_compute_screen_space_position(Rc::new(|position: &Cartesian3| {
        Some(Cartesian2::new(position.x, position.y))
    }));
    view_model.update();
    // Negative half the test size, plus viewModel.position.x (1)
    assert_eq!(view_model.screen_position_x(), "-9px");
    // Negative half the test size, plus viewModel.position.y (2)
    assert_eq!(view_model.screen_position_y(), "-8px");
}

/// Mirrors `it("hides the indicator when position is unknown")`.
#[test]
fn selection_indicator_view_model_hides_the_indicator_when_position_is_unknown() {
    let (selection_indicator_element, container) = selection_indicator_fixtures();
    let scene: Rc<dyn SelectionScene> = Rc::new(MockSelectionScene::default());
    let mut view_model = SelectionIndicatorViewModel::new(
        scene,
        selection_indicator_element,
        container,
    );
    assert!(!view_model.is_visible());
    view_model.set_show_selection(true);
    assert!(!view_model.is_visible());
    view_model.set_position(Some(Cartesian3::new(1.0, 2.0, 3.0)));
    assert!(view_model.is_visible());
    view_model.set_show_selection(false);
    assert!(!view_model.is_visible());
}

/// Mirrors `it("can move the indicator off screen")`.
#[test]
fn selection_indicator_view_model_can_move_the_indicator_off_screen() {
    let (selection_indicator_element, container) = selection_indicator_fixtures();
    let scene: Rc<dyn SelectionScene> = Rc::new(MockSelectionScene::default());
    let mut view_model = SelectionIndicatorViewModel::new(
        scene,
        selection_indicator_element,
        container,
    );
    view_model.set_show_selection(true);
    view_model.set_position(Some(Cartesian3::new(1.0, 2.0, 3.0)));
    view_model.set_compute_screen_space_position(Rc::new(|_position: &Cartesian3| None));
    view_model.update();
    assert_eq!(view_model.screen_position_x(), "-1000px");
    assert_eq!(view_model.screen_position_y(), "-1000px");
}

// ===========================================================================
// Widgets/VRButton/VRButtonViewModel —
// packages/widgets/Specs/VRButton/VRButtonViewModelSpec.js
// ===========================================================================

use cesium_core::event::Event;
use cesium_widgets::knockout::ElementOrId;

/// Mock scene standing in for the JS `Scene` used by VRButtonViewModel
/// (records `useWebVR` writes, hosts the `preRender` event and a
/// settable orthographic flag).
#[derive(Default)]
struct MockVrScene {
    use_web_vr: Rc<Cell<bool>>,
    pre_render: Event<JulianDate>,
    orthographic: Cell<bool>,
}

impl VrScene for MockVrScene {
    fn set_use_web_vr(&self, value: bool) {
        self.use_web_vr.set(value);
    }

    fn pre_render(&self) -> &Event<JulianDate> {
        &self.pre_render
    }

    fn camera_is_orthographic(&self) -> bool {
        self.orthographic.get()
    }
}

/// Mirrors `it("constructor sets default values")`.
#[test]
fn vr_button_view_model_constructor_sets_default_values() {
    let document = MockDocument::new();
    let scene: Rc<dyn VrScene> = Rc::new(MockVrScene::default());
    let mut view_model = VrButtonViewModel::new(scene, None, &document);
    // JS: `expect(viewModel.vrElement).toBe(document.body)`.
    assert_eq!(view_model.vr_element(), *document.body());
    assert!(!view_model.is_destroyed());
    view_model.destroy();
    assert!(view_model.is_destroyed());
}

/// Mirrors `it("constructor sets expected values")`.
#[test]
fn vr_button_view_model_constructor_sets_expected_values() {
    let document = MockDocument::new();
    let scene: Rc<dyn VrScene> = Rc::new(MockVrScene::default());
    let test_element = MockDomElement::new("span");
    let mut view_model = VrButtonViewModel::new(
        scene,
        Some(ElementOrId::Element(test_element.clone())),
        &document,
    );
    assert_eq!(view_model.vr_element(), test_element);
    view_model.destroy();
}

/// Mirrors `it("constructor can take an element id")`.
#[test]
fn vr_button_view_model_constructor_can_take_an_element_id() {
    let mut document = MockDocument::new();
    let test_element = MockDomElement::new("span").with_id("testElement");
    document.append(test_element.clone());
    let scene: Rc<dyn VrScene> = Rc::new(MockVrScene::default());
    let mut view_model = VrButtonViewModel::new(
        scene,
        Some(ElementOrId::Id("testElement".to_string())),
        &document,
    );
    assert_eq!(view_model.vr_element(), test_element);
    view_model.destroy();
    document.remove("testElement");
}

/// Mirrors `it("isVREnabled work as expected")`.
#[test]
fn vr_button_view_model_is_vr_enabled_works_as_expected() {
    let document = MockDocument::new();
    let scene: Rc<dyn VrScene> = Rc::new(MockVrScene::default());
    let view_model = VrButtonViewModel::new(scene, None, &document);
    // JS: `expect(viewModel.isVREnabled).toEqual(Fullscreen.enabled)`;
    // headless `fullscreen_enabled()` is always false.
    assert_eq!(
        view_model.is_vr_enabled(),
        cesium_widgets::knockout::fullscreen_enabled()
    );
    view_model.set_is_vr_enabled(false);
    assert!(!view_model.is_vr_enabled());
}

/// Mirrors `it("can get and set vrElement")`.
#[test]
fn vr_button_view_model_can_get_and_set_vr_element() {
    let document = MockDocument::new();
    let scene: Rc<dyn VrScene> = Rc::new(MockVrScene::default());
    let view_model = VrButtonViewModel::new(scene, None, &document);
    let test_element = MockDomElement::new("span");
    assert_ne!(view_model.vr_element(), test_element);
    view_model.set_vr_element(test_element.clone());
    assert_eq!(view_model.vr_element(), test_element);
}

/// Mirrors `it("throws when constructed without a scene")`.
#[test]
fn vr_button_view_model_throws_when_constructed_without_a_scene() {
    let document = MockDocument::new();
    expect_to_throw_dev_error(|| {
        let _ = VrButtonViewModel::try_new(None, None, &document);
    });
}

/// Mirrors `it("throws is setting vrElement is not an Element")`.
#[test]
fn vr_button_view_model_throws_if_setting_vr_element_is_not_an_element() {
    let document = MockDocument::new();
    let scene: Rc<dyn VrScene> = Rc::new(MockVrScene::default());
    let view_model = VrButtonViewModel::new(scene, None, &document);
    expect_to_throw_dev_error(|| {
        view_model.try_set_vr_element(None);
    });
}

/// Wiring test (no JS mirror; task #18): the constructor installs the
/// `scene.preRender` subscription (`_eventHelper.add(scene.preRender,
/// ...)`) refreshing the orthographic flag, and `destroy` removes it
/// (`_eventHelper.removeAll()`).
#[test]
fn vr_button_view_model_tracks_orthographic_through_pre_render() {
    let document = MockDocument::new();
    let mock = Rc::new(MockVrScene::default());
    let scene = Rc::clone(&mock) as Rc<dyn VrScene>;
    let mut view_model = VrButtonViewModel::new(scene, None, &document);

    assert_eq!(mock.pre_render.number_of_listeners(), 1);
    assert!(!view_model.is_orthographic());

    mock.orthographic.set(true);
    mock.pre_render.raise_event(&JulianDate::now());
    assert!(view_model.is_orthographic());

    view_model.destroy();
    assert_eq!(mock.pre_render.number_of_listeners(), 0);
    mock.orthographic.set(false);
    mock.pre_render.raise_event(&JulianDate::now());
    // The subscription is gone: the flag is no longer refreshed.
    assert!(view_model.is_orthographic());
}

/// Wiring test (no JS mirror; task #18): the engine `Scene` plugs into
/// the view model through `impl VrScene for Scene` — the `preRender`
/// subscription is installed/removed on the real event and `useWebVR`
/// writes reach the scene flag.
#[test]
fn vr_button_view_model_wires_the_engine_scene() {
    let document = MockDocument::new();
    let scene = Rc::new(cesium_scene::scene::Scene::new());
    assert_eq!(scene.pre_render().number_of_listeners(), 0);

    let mut view_model = VrButtonViewModel::new(
        Rc::clone(&scene) as Rc<dyn VrScene>,
        None,
        &document,
    );
    assert_eq!(scene.pre_render().number_of_listeners(), 1);

    // The headless default camera is perspective, so a preRender pass
    // keeps the flag false (mirrors the JS subscription body).
    scene.pre_render().raise_event(&JulianDate::now());
    assert!(!view_model.is_orthographic());

    view_model.destroy();
    assert_eq!(scene.pre_render().number_of_listeners(), 0);
}
