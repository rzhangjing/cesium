//! Clock spec - ported from packages/engine/Specs/Core/ClockSpec.js
//! 27 original it() blocks → 16 A-class tests ported
//! Skipped 11 C-class: 1 throws + 2 events(onStop) + 8 SYSTEM_CLOCK modes (jasmine.clock mock)

use cesium_time::{Clock, ClockOptions, ClockRange, ClockStep, JulianDate};

// ============================================================================
// Constructor tests (8)
// ============================================================================

#[test]
fn sets_default_parameters_when_constructed() {
    let clock = Clock::from_options(&ClockOptions::default());

    // stopTime = startTime + 1 day
    let expected_stop = clock.start_time.add_days(1.0);
    assert_eq!(clock.stop_time, expected_stop);
    // startTime == currentTime
    assert_eq!(clock.start_time, clock.current_time);
    // defaults
    assert_eq!(clock.multiplier, 1.0);
    assert_eq!(clock.clock_step, ClockStep::SystemClockMultiplier);
    assert_eq!(clock.clock_range, ClockRange::Unbounded);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);
}

#[test]
fn sets_provided_constructor_parameters_correctly() {
    let start = JulianDate::new(12.0, 0.0);
    let stop = JulianDate::new(112.0, 0.0);
    let current_time = JulianDate::new(13.0, 0.0);
    let step = ClockStep::TickDependent;
    let range = ClockRange::LoopStop;
    let multiplier = 1.5;

    let clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(step),
        multiplier: Some(multiplier),
        clock_range: Some(range),
        ..Default::default()
    });

    assert_eq!(clock.start_time, start);
    assert_eq!(clock.stop_time, stop);
    assert_eq!(clock.current_time, current_time);
    assert_eq!(clock.clock_step, step);
    assert_eq!(clock.clock_range, range);
    assert_eq!(clock.multiplier, multiplier);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);

    // canAnimate: false
    let clock = Clock::from_options(&ClockOptions {
        can_animate: Some(false),
        ..Default::default()
    });
    assert_eq!(clock.can_animate, false);

    // shouldAnimate: true
    let clock = Clock::from_options(&ClockOptions {
        should_animate: Some(true),
        ..Default::default()
    });
    assert_eq!(clock.should_animate, true);
}

#[test]
fn works_when_constructed_with_no_current_time_parameter() {
    let start = JulianDate::new(12.0, 0.0);
    let stop = JulianDate::new(112.0, 0.0);
    let step = ClockStep::TickDependent;
    let range = ClockRange::LoopStop;
    let multiplier = 1.5;

    let clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        clock_step: Some(step),
        multiplier: Some(multiplier),
        clock_range: Some(range),
        ..Default::default()
    });

    assert_eq!(clock.start_time, start);
    assert_eq!(clock.stop_time, stop);
    // currentTime defaults to startTime
    assert_eq!(clock.current_time, start);
    assert_eq!(clock.clock_step, step);
    assert_eq!(clock.clock_range, range);
    assert_eq!(clock.multiplier, multiplier);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);
}

#[test]
fn works_when_constructed_with_no_start_time_parameter() {
    let stop = JulianDate::new(112.0, 0.0);
    let current_time = JulianDate::new(13.0, 0.0);
    let step = ClockStep::TickDependent;
    let range = ClockRange::LoopStop;
    let multiplier = 1.5;

    let clock = Clock::from_options(&ClockOptions {
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(step),
        multiplier: Some(multiplier),
        clock_range: Some(range),
        ..Default::default()
    });

    // startTime defaults to currentTime
    assert_eq!(clock.start_time, current_time);
    assert_eq!(clock.stop_time, stop);
    assert_eq!(clock.current_time, current_time);
    assert_eq!(clock.clock_step, step);
    assert_eq!(clock.clock_range, range);
    assert_eq!(clock.multiplier, multiplier);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);
}

#[test]
fn works_when_constructed_with_no_start_time_or_stop_time() {
    let current_time = JulianDate::new(12.0, 0.0);
    let step = ClockStep::TickDependent;
    let range = ClockRange::LoopStop;
    let multiplier = 1.5;

    let clock = Clock::from_options(&ClockOptions {
        current_time: Some(current_time),
        clock_step: Some(step),
        multiplier: Some(multiplier),
        clock_range: Some(range),
        ..Default::default()
    });

    let expected_stop = current_time.add_days(1.0);
    // startTime defaults to currentTime
    assert_eq!(clock.start_time, current_time);
    // stopTime defaults to startTime + 1 day
    assert_eq!(clock.stop_time, expected_stop);
    assert_eq!(clock.current_time, current_time);
    assert_eq!(clock.clock_step, step);
    assert_eq!(clock.clock_range, range);
    assert_eq!(clock.multiplier, multiplier);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);
}

#[test]
fn works_when_constructed_with_no_start_time_or_current_time() {
    let stop = JulianDate::new(13.0, 0.0);
    let step = ClockStep::TickDependent;
    let range = ClockRange::LoopStop;
    let multiplier = 1.5;

    let clock = Clock::from_options(&ClockOptions {
        stop_time: Some(stop),
        clock_step: Some(step),
        multiplier: Some(multiplier),
        clock_range: Some(range),
        ..Default::default()
    });

    // currentTime defaults to stopTime - 1 day
    let expected_start = stop.add_days(-1.0);
    assert_eq!(clock.start_time, expected_start);
    assert_eq!(clock.stop_time, stop);
    assert_eq!(clock.current_time, expected_start);
    assert_eq!(clock.clock_step, step);
    assert_eq!(clock.clock_range, range);
    assert_eq!(clock.multiplier, multiplier);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);
}

#[test]
fn works_when_constructed_with_no_current_time_or_stop_time() {
    let start = JulianDate::new(12.0, 0.0);
    let step = ClockStep::TickDependent;
    let range = ClockRange::LoopStop;
    let multiplier = 1.5;

    let clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        clock_step: Some(step),
        multiplier: Some(multiplier),
        clock_range: Some(range),
        ..Default::default()
    });

    let expected_stop = start.add_days(1.0);
    assert_eq!(clock.start_time, start);
    assert_eq!(clock.stop_time, expected_stop);
    // currentTime defaults to startTime
    assert_eq!(clock.current_time, start);
    assert_eq!(clock.clock_step, step);
    assert_eq!(clock.clock_range, range);
    assert_eq!(clock.multiplier, multiplier);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);
}

#[test]
fn works_when_constructed_with_no_stop_time_parameter() {
    let start = JulianDate::new(12.0, 0.0);
    let current_time = JulianDate::new(12.0, 0.0);
    let step = ClockStep::TickDependent;
    let range = ClockRange::LoopStop;
    let multiplier = 1.5;

    let clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        current_time: Some(current_time),
        clock_step: Some(step),
        multiplier: Some(multiplier),
        clock_range: Some(range),
        ..Default::default()
    });

    let expected_stop = start.add_days(1.0);
    assert_eq!(clock.start_time, start);
    assert_eq!(clock.stop_time, expected_stop);
    assert_eq!(clock.current_time, current_time);
    assert_eq!(clock.clock_step, step);
    assert_eq!(clock.clock_range, range);
    assert_eq!(clock.multiplier, multiplier);
    assert_eq!(clock.can_animate, true);
    assert_eq!(clock.should_animate, false);
}

// ============================================================================
// TICK_DEPENDENT mode tests (8)
// ============================================================================

#[test]
fn animates_forward_in_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = JulianDate::new(0.5, 0.0);
    let multiplier = 1.5;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::LoopStop),
        should_animate: Some(true),
        ..Default::default()
    });
    assert_eq!(clock.current_time, current_time);

    let mut expected = current_time.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);

    expected = expected.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);
}

#[test]
fn animates_backwards_in_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = JulianDate::new(0.5, 0.0);
    let multiplier = -1.5;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::LoopStop),
        should_animate: Some(true),
        ..Default::default()
    });
    assert_eq!(clock.current_time, current_time);

    let mut expected = current_time.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);

    expected = expected.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);
}

#[test]
fn animates_forwards_past_stop_time_in_unbounded_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = stop;
    let multiplier = 1.5;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::Unbounded),
        should_animate: Some(true),
        ..Default::default()
    });
    assert_eq!(clock.current_time, current_time);

    let mut expected = current_time.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);

    expected = expected.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);
}

#[test]
fn animates_backwards_past_start_time_in_unbounded_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = start;
    let multiplier = -1.5;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::Unbounded),
        should_animate: Some(true),
        ..Default::default()
    });
    assert_eq!(clock.current_time, current_time);

    let mut expected = current_time.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);

    expected = expected.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);
}

#[test]
fn loops_back_to_start_time_when_animating_forward_past_stop_in_loop_stop_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = stop;
    let multiplier = 1.5;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::LoopStop),
        should_animate: Some(true),
        ..Default::default()
    });
    assert_eq!(clock.current_time, current_time);

    // First tick: stop + 1.5 overflows → loops to start + 1.5
    let mut expected = start.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);

    // Second tick: (start + 1.5) + 1.5 = start + 3.0, overflows → start + (3.0 - 1day_secs)
    // But 1 day = 86400s, so start + 3.0 < stop. No overflow.
    expected = expected.add_seconds(multiplier);
    let result = clock.tick(0.0);
    assert_eq!(result, expected);
    assert_eq!(clock.current_time, expected);
}

#[test]
fn stops_at_start_when_animating_backwards_past_start_in_loop_stop_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = start;
    let multiplier = -100.0;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::LoopStop),
        should_animate: Some(true),
        ..Default::default()
    });

    assert_eq!(clock.current_time, current_time);
    let result = clock.tick(0.0);
    assert_eq!(result, start);
    assert_eq!(clock.current_time, start);
}

#[test]
fn stops_at_stop_time_when_animating_forwards_past_stop_in_clamped_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = stop;
    let multiplier = 100.0;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::Clamped),
        should_animate: Some(true),
        ..Default::default()
    });

    assert_eq!(clock.current_time, current_time);
    let result = clock.tick(0.0);
    assert_eq!(result, stop);
    assert_eq!(clock.current_time, stop);
}

#[test]
fn stops_at_start_time_when_animating_backwards_past_start_in_clamped_tick_dependent_mode() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);
    let current_time = start;
    let multiplier = -100.0;

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(current_time),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(multiplier),
        clock_range: Some(ClockRange::Clamped),
        should_animate: Some(true),
        ..Default::default()
    });

    assert_eq!(clock.current_time, current_time);
    let result = clock.tick(0.0);
    assert_eq!(result, start);
    assert_eq!(clock.current_time, start);
}

// ============================================================================
// SYSTEM_CLOCK_MULTIPLIER mode test (adapted from C-class jasmine.clock test)
// ============================================================================

#[test]
fn uses_multiplier_in_system_clock_multiplier_mode() {
    // Adapted: instead of jasmine.clock().tick(1000), we pass delta_secs = 1.0
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(start),
        clock_step: Some(ClockStep::SystemClockMultiplier),
        multiplier: Some(2.0),
        should_animate: Some(true),
        ..Default::default()
    });

    // First tick with 0 elapsed → no advance
    let time1 = clock.tick(0.0);
    assert_eq!(time1, start);

    // Second tick with 1.0 seconds elapsed → advances by 2.0 * 1.0 = 2.0 seconds
    let time2 = clock.tick(1.0);
    let expected = start.add_seconds(2.0);
    assert_eq!(time2, expected);
    assert_eq!(clock.current_time, expected);
}

#[test]
fn does_not_advance_if_should_animate_is_false() {
    let start = JulianDate::new(0.0, 0.0);
    let stop = JulianDate::new(1.0, 0.0);

    let mut clock = Clock::from_options(&ClockOptions {
        start_time: Some(start),
        stop_time: Some(stop),
        current_time: Some(start),
        clock_step: Some(ClockStep::SystemClockMultiplier),
        multiplier: Some(1.0),
        should_animate: Some(false),
        ..Default::default()
    });

    // shouldAnimate = false → no advance
    let time1 = clock.tick(1.0);
    assert_eq!(time1, start);
    assert_eq!(clock.current_time, start);

    // Enable animation
    clock.should_animate = true;
    let time2 = clock.tick(1.0);
    let expected = start.add_seconds(1.0);
    assert_eq!(time2, expected);

    // Switch to TICK_DEPENDENT
    clock.current_time = start;
    clock.clock_step = ClockStep::TickDependent;

    clock.should_animate = false;
    let time3 = clock.tick(0.0);
    assert_eq!(time3, start);
    assert_eq!(clock.current_time, start);

    clock.should_animate = true;
    let time4 = clock.tick(0.0);
    let expected = start.add_seconds(1.0); // multiplier = 1.0
    assert_eq!(time4, expected);
}
