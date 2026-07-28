//! Clock - simulation clock for time management.
//! Maps to CesiumJS `Core/Clock.js`, `Core/ClockRange.js`, `Core/ClockStep.js`

use crate::julian_date::JulianDate;
use serde::{Deserialize, Serialize};

/// Determines how the clock behaves when start/stop time is reached.
/// Maps to CesiumJS `ClockRange`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClockRange {
    /// Clock always advances in its current direction.
    #[default]
    Unbounded,
    /// Clock will not advance past start/stop time.
    Clamped,
    /// Clock loops back to start when stop is reached.
    LoopStop,
}

/// Determines how much time advances with each tick.
/// Maps to CesiumJS `ClockStep`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClockStep {
    /// Advances by a fixed number of seconds (multiplier).
    TickDependent,
    /// Advances by elapsed system time * multiplier.
    #[default]
    SystemClockMultiplier,
    /// Sets clock to current system time.
    SystemClock,
}

/// A simple clock for keeping track of simulated time.
/// Maps to CesiumJS `Clock`
#[derive(Debug, Clone)]
pub struct Clock {
    /// The start time of the clock.
    pub start_time: JulianDate,
    /// The stop time of the clock.
    pub stop_time: JulianDate,
    /// The current time.
    pub current_time: JulianDate,
    /// How much time advances per tick (seconds or multiplier).
    pub multiplier: f64,
    /// Determines tick behavior (frame-dependent or system-clock-dependent).
    pub clock_step: ClockStep,
    /// Determines behavior at start/stop boundaries.
    pub clock_range: ClockRange,
    /// Whether tick can advance time.
    pub can_animate: bool,
    /// Whether tick should attempt to advance time.
    pub should_animate: bool,
    /// Last system time in seconds (for SystemClockMultiplier).
    #[allow(dead_code)]
    last_system_time_secs: f64,
}

/// Options for constructing a Clock.
/// Maps to CesiumJS Clock constructor options object.
#[derive(Debug, Clone, Default)]
pub struct ClockOptions {
    /// The start time of the clock.
    pub start_time: Option<JulianDate>,
    /// The stop time of the clock.
    pub stop_time: Option<JulianDate>,
    /// The current time.
    pub current_time: Option<JulianDate>,
    /// Determines how much time advances per tick.
    pub multiplier: Option<f64>,
    /// Determines tick behavior.
    pub clock_step: Option<ClockStep>,
    /// Determines behavior at start/stop boundaries.
    pub clock_range: Option<ClockRange>,
    /// Whether tick can advance time.
    pub can_animate: Option<bool>,
    /// Whether tick should attempt to advance time.
    pub should_animate: Option<bool>,
}

impl Clock {
    /// Creates a new Clock with the given parameters.
    pub fn new(
        start_time: JulianDate,
        stop_time: JulianDate,
        current_time: JulianDate,
    ) -> Self {
        Self {
            start_time,
            stop_time,
            current_time,
            multiplier: 1.0,
            clock_step: ClockStep::SystemClockMultiplier,
            clock_range: ClockRange::Unbounded,
            can_animate: true,
            should_animate: false,
            last_system_time_secs: Self::get_system_time_secs(),
        }
    }

    /// Creates a Clock from options, faithfully mirroring CesiumJS Clock constructor.
    /// Derivation rules:
    /// - currentTime: if not specified → startTime if set, else stopTime - 1 day, else now
    /// - startTime: if not specified → currentTime (as derived above)
    /// - stopTime: if not specified → startTime + 1 day
    pub fn from_options(options: &ClockOptions) -> Self {
        // Derive currentTime
        let current_time = if let Some(ct) = options.current_time {
            ct
        } else if let Some(st) = options.start_time {
            st
        } else if let Some(stop) = options.stop_time {
            stop.add_days(-1.0)
        } else {
            JulianDate::now()
        };

        // Derive startTime
        let start_time = options.start_time.unwrap_or(current_time);

        // Derive stopTime
        let stop_time = options.stop_time.unwrap_or_else(|| start_time.add_days(1.0));

        Self {
            start_time,
            stop_time,
            current_time,
            multiplier: options.multiplier.unwrap_or(1.0),
            clock_step: options.clock_step.unwrap_or(ClockStep::SystemClockMultiplier),
            clock_range: options.clock_range.unwrap_or(ClockRange::Unbounded),
            can_animate: options.can_animate.unwrap_or(true),
            should_animate: options.should_animate.unwrap_or(false),
            last_system_time_secs: Self::get_system_time_secs(),
        }
    }

    /// Creates a clock with default settings (current time = now).
    pub fn default_now() -> Self {
        Self::from_options(&ClockOptions::default())
    }

    /// Advances the clock from the current time.
    /// Maps to `Clock.tick()`
    ///
    /// `delta_secs` is the elapsed system time in seconds since last tick
    /// (provided by the caller for framework independence).
    pub fn tick(&mut self, delta_secs: f64) -> JulianDate {
        let mut current_time = self.current_time;

        if self.can_animate && self.should_animate {
            match self.clock_step {
                ClockStep::SystemClock => {
                    current_time = JulianDate::now();
                }
                ClockStep::TickDependent => {
                    current_time = current_time.add_seconds(self.multiplier);
                }
                ClockStep::SystemClockMultiplier => {
                    current_time = current_time.add_seconds(self.multiplier * delta_secs);
                }
            }

            // Apply clock range constraints
            match self.clock_range {
                ClockRange::Clamped => {
                    if current_time.less_than(&self.start_time) {
                        current_time = self.start_time;
                    } else if current_time.greater_than(&self.stop_time) {
                        current_time = self.stop_time;
                    }
                }
                ClockRange::LoopStop => {
                    if current_time.less_than(&self.start_time) {
                        current_time = self.start_time;
                    }
                    while current_time.greater_than(&self.stop_time) {
                        let overshoot = current_time.seconds_difference(&self.stop_time);
                        current_time = self.start_time.add_seconds(overshoot);
                    }
                }
                ClockRange::Unbounded => {}
            }
        }

        self.current_time = current_time;
        current_time
    }

    /// Gets the current system time in seconds (monotonic).
    fn get_system_time_secs() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::default_now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_unbounded() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        let mut clock = Clock::new(start, stop, start);
        clock.should_animate = true;
        clock.clock_step = ClockStep::TickDependent;
        clock.multiplier = 60.0; // 60 seconds per tick

        let result = clock.tick(0.016); // delta doesn't matter for TickDependent
        let expected = start.add_seconds(60.0);
        assert!(result.equals_epsilon(&expected, 1e-10));
    }

    #[test]
    fn test_tick_clamped() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 1, 0, 1, 0, 0.0); // 1 minute
        let mut clock = Clock::new(start, stop, start);
        clock.should_animate = true;
        clock.clock_step = ClockStep::TickDependent;
        clock.clock_range = ClockRange::Clamped;
        clock.multiplier = 120.0; // 2 minutes per tick (exceeds stop)

        let result = clock.tick(0.016);
        assert_eq!(result, stop); // Clamped to stop
    }

    #[test]
    fn test_tick_loop_stop() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 1, 1, 0, 0, 0.0); // 1 hour
        let current = JulianDate::from_date_components(2000, 1, 1, 0, 59, 0, 0.0);
        let mut clock = Clock::new(start, stop, current);
        clock.should_animate = true;
        clock.clock_step = ClockStep::TickDependent;
        clock.clock_range = ClockRange::LoopStop;
        clock.multiplier = 120.0; // 2 minutes per tick

        let result = clock.tick(0.016);
        // 59:00 + 2:00 = 61:00, which is 1:00 past stop (60:00)
        // Loops to start + 60 seconds = 00:01:00
        let expected = start.add_seconds(60.0);
        assert!(result.equals_epsilon(&expected, 1e-10));
    }

    #[test]
    fn test_tick_no_animate() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        let mut clock = Clock::new(start, stop, start);
        clock.should_animate = false;
        clock.clock_step = ClockStep::TickDependent;
        clock.multiplier = 60.0;

        let result = clock.tick(0.016);
        assert_eq!(result, start); // Should not advance
    }

    #[test]
    fn test_system_clock_multiplier() {
        let start = JulianDate::from_date_components(2000, 1, 1, 0, 0, 0, 0.0);
        let stop = JulianDate::from_date_components(2000, 1, 2, 0, 0, 0, 0.0);
        let mut clock = Clock::new(start, stop, start);
        clock.should_animate = true;
        clock.clock_step = ClockStep::SystemClockMultiplier;
        clock.multiplier = 2.0; // 2x speed

        let result = clock.tick(0.5); // 0.5 seconds elapsed
        let expected = start.add_seconds(1.0); // 0.5 * 2.0 = 1.0 second
        assert!(result.equals_epsilon(&expected, 1e-10));
    }
}
