use cesium_core::clock::Clock;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::julian_date::JulianDate;

fn make_clock() -> Clock {
    let start = JulianDate::from_iso8601("2020-01-01T00:00:00Z").unwrap();
    let stop = JulianDate::add_days(&start, 1.0);
    Clock::new(
        Some(start),
        Some(stop),
        None,
        Some(1.0),
        Some(ClockStep::SystemClockMultiplier),
        Some(ClockRange::Unbounded),
        Some(true),
        Some(true),
    )
}

#[test]
fn default_constructor() {
    let clock = Clock::new(None, None, None, None, None, None, None, None);
    // Should not panic and should have valid defaults
    assert!(clock.can_animate);
}

#[test]
fn constructor_with_times() {
    let clock = make_clock();
    let start = JulianDate::from_iso8601("2020-01-01T00:00:00Z").unwrap();
    let diff = JulianDate::seconds_difference(clock.current_time(), &start);
    assert!(diff.abs() < 1.0);
}

#[test]
fn multiplier_is_accessible() {
    let clock = make_clock();
    assert_eq!(clock.get_multiplier(), 1.0);
}

#[test]
fn clock_range_is_accessible() {
    let clock = make_clock();
    assert_eq!(clock.clock_range, ClockRange::Unbounded);
}

#[test]
fn tick_advances_time() {
    let mut clock = make_clock();
    let before = clock.current_time().clone();
    let after = clock.tick();
    // After tick, time should have advanced (or at least not gone backwards with positive multiplier)
    let diff = JulianDate::seconds_difference(&after, &before);
    assert!(diff >= 0.0, "tick should not go backwards: diff={}", diff);
}
