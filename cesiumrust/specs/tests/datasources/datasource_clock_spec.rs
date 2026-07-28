//! Tests for DataSourceClock + CustomDataSource
//! - DataSourceClockSpec.js: 5 it() → 4 A-class (1 C-class: throws omitted)
//! - CustomDataSourceSpec.js: 6 it() → 2 A-class (4 C-class: events omitted)

use cesium_datasource::custom_data_source::CustomDataSource;
use cesium_datasource::datasource_clock::DataSourceClock;
use cesium_time::{ClockRange, ClockStep, JulianDate};

fn jd(seconds: f64) -> JulianDate {
    JulianDate::new(0.0, seconds)
}

// === DataSourceClock: merge assigns unassigned properties ===

#[test]
fn test_datasource_clock_merge_assigns_unassigned() {
    let source = DataSourceClock {
        start_time: Some(jd(100.0)),
        stop_time: Some(jd(200.0)),
        current_time: Some(jd(150.0)),
        clock_range: Some(ClockRange::Clamped),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(2.0),
    };

    let mut target = DataSourceClock::new();
    target.merge(&source);

    assert_eq!(target.start_time, Some(jd(100.0)));
    assert_eq!(target.stop_time, Some(jd(200.0)));
    assert_eq!(target.current_time, Some(jd(150.0)));
    assert_eq!(target.clock_range, Some(ClockRange::Clamped));
    assert_eq!(target.clock_step, Some(ClockStep::TickDependent));
    assert_eq!(target.multiplier, Some(2.0));
}

// === DataSourceClock: merge does not assign assigned properties ===

#[test]
fn test_datasource_clock_merge_preserves_assigned() {
    let source = DataSourceClock {
        start_time: Some(jd(100.0)),
        stop_time: Some(jd(200.0)),
        current_time: Some(jd(150.0)),
        clock_range: Some(ClockRange::Clamped),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(2.0),
    };

    let mut target = DataSourceClock {
        start_time: Some(jd(300.0)),
        stop_time: Some(jd(400.0)),
        current_time: Some(jd(350.0)),
        clock_range: Some(ClockRange::LoopStop),
        clock_step: Some(ClockStep::SystemClockMultiplier),
        multiplier: Some(5.0),
    };
    target.merge(&source);

    // All should remain unchanged
    assert_eq!(target.start_time, Some(jd(300.0)));
    assert_eq!(target.stop_time, Some(jd(400.0)));
    assert_eq!(target.current_time, Some(jd(350.0)));
    assert_eq!(target.clock_range, Some(ClockRange::LoopStop));
    assert_eq!(target.clock_step, Some(ClockStep::SystemClockMultiplier));
    assert_eq!(target.multiplier, Some(5.0));
}

// === DataSourceClock: clone works ===

#[test]
fn test_datasource_clock_clone() {
    let source = DataSourceClock {
        start_time: Some(jd(100.0)),
        stop_time: Some(jd(200.0)),
        current_time: Some(jd(150.0)),
        clock_range: Some(ClockRange::Clamped),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(3.0),
    };

    let cloned = source.clone();
    assert_eq!(cloned.start_time, source.start_time);
    assert_eq!(cloned.stop_time, source.stop_time);
    assert_eq!(cloned.current_time, source.current_time);
    assert_eq!(cloned.clock_range, source.clock_range);
    assert_eq!(cloned.clock_step, source.clock_step);
    assert_eq!(cloned.multiplier, source.multiplier);
}

// === DataSourceClock: gets value as a clock instance ===

#[test]
fn test_datasource_clock_get_value() {
    let source = DataSourceClock {
        start_time: Some(jd(100.0)),
        stop_time: Some(jd(200.0)),
        current_time: Some(jd(150.0)),
        clock_range: Some(ClockRange::Clamped),
        clock_step: Some(ClockStep::TickDependent),
        multiplier: Some(2.0),
    };

    let clock = source.get_value();
    assert_eq!(clock.start_time, jd(100.0));
    assert_eq!(clock.stop_time, jd(200.0));
    assert_eq!(clock.current_time, jd(150.0));
    assert_eq!(clock.clock_range, ClockRange::Clamped);
    assert_eq!(clock.clock_step, ClockStep::TickDependent);
    assert_eq!(clock.multiplier, 2.0);

    // With unset fields → defaults
    let source2 = DataSourceClock {
        start_time: Some(jd(100.0)),
        stop_time: Some(jd(200.0)),
        current_time: Some(jd(150.0)),
        clock_range: None,
        clock_step: None,
        multiplier: None,
    };

    let clock2 = source2.get_value();
    assert_eq!(clock2.start_time, jd(100.0));
    assert_eq!(clock2.stop_time, jd(200.0));
    assert_eq!(clock2.current_time, jd(150.0));
    assert_eq!(clock2.clock_range, ClockRange::Unbounded);
    assert_eq!(clock2.clock_step, ClockStep::SystemClockMultiplier);
    assert_eq!(clock2.multiplier, 1.0);
}

// === CustomDataSource: constructor has expected defaults ===

#[test]
fn test_custom_datasource_constructor_defaults() {
    let ds = CustomDataSource::new("TestDS");
    assert_eq!(ds.name(), "TestDS");
    assert!(ds.show());
    assert!(!ds.is_loading());
    assert!(ds.clock().is_none());
    assert_eq!(ds.entities().len(), 0);
}

// === CustomDataSource: show sets underlying entity collection show ===

#[test]
fn test_custom_datasource_show() {
    let mut ds = CustomDataSource::new("TestDS");
    assert!(ds.entities().show());

    ds.set_show(false);
    assert!(!ds.show());
    assert!(!ds.entities().show());

    ds.set_show(true);
    assert!(ds.show());
    assert!(ds.entities().show());
}
