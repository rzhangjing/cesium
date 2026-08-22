//! Ported from `packages/engine/Source/Core/Clock.js`.
//!
//! A simple clock for keeping track of simulated time.

use crate::clock_range::ClockRange;
use crate::clock_step::ClockStep;
use crate::event::Event;
use crate::get_timestamp::get_timestamp;
use crate::julian_date::JulianDate;

/// A simple clock for keeping track of simulated time.
pub struct Clock {
    /// The start time of the clock.
    pub start_time: JulianDate,
    /// The stop time of the clock.
    pub stop_time: JulianDate,
    /// Determines how the clock should behave when startTime or stopTime is reached.
    pub clock_range: ClockRange,
    /// Indicates whether `tick` can advance time.
    pub can_animate: bool,
    /// An event fired when `tick` is called.
    pub on_tick: Event<()>,
    /// An event fired when `stop_time` is reached.
    pub on_stop: Event<()>,

    current_time: JulianDate,
    multiplier: f64,
    clock_step: ClockStep,
    should_animate: bool,
    last_system_time: f64,
}

impl Clock {
    /// Creates a new Clock.
    pub fn new(
        start_time: Option<JulianDate>,
        stop_time: Option<JulianDate>,
        current_time: Option<JulianDate>,
        multiplier: Option<f64>,
        clock_step: Option<ClockStep>,
        clock_range: Option<ClockRange>,
        can_animate: Option<bool>,
        should_animate: Option<bool>,
    ) -> Self {
        let current_time = if let Some(ct) = current_time {
            ct
        } else if let Some(st) = &start_time {
            st.clone()
        } else if let Some(spt) = &stop_time {
            JulianDate::add_days(spt, -1.0)
        } else {
            JulianDate::now()
        };

        let start_time = start_time.unwrap_or_else(|| current_time.clone());
        let stop_time = stop_time.unwrap_or_else(|| JulianDate::add_days(&start_time, 1.0));

        let mut clock = Self {
            start_time,
            stop_time,
            clock_range: clock_range.unwrap_or(ClockRange::Unbounded),
            can_animate: can_animate.unwrap_or(true),
            on_tick: Event::new(),
            on_stop: Event::new(),
            current_time,
            multiplier: multiplier.unwrap_or(1.0),
            clock_step: clock_step.unwrap_or(ClockStep::SystemClockMultiplier),
            should_animate: should_animate.unwrap_or(false),
            last_system_time: get_timestamp(),
        };

        // Apply setters to ensure side effects (like clock_step switching)
        if let Some(m) = multiplier {
            clock.set_multiplier(m);
        }
        if let Some(sa) = should_animate {
            clock.set_should_animate(sa);
        }
        if let Some(cs) = clock_step {
            clock.set_clock_step(cs);
        }

        clock
    }

    /// Gets the current time.
    pub fn current_time(&self) -> &JulianDate {
        &self.current_time
    }

    /// Sets the current time. Changes clock_step from SystemClock to SystemClockMultiplier.
    pub fn set_current_time(&mut self, value: JulianDate) {
        if JulianDate::equals(&self.current_time, &value) {
            return;
        }
        if self.clock_step == ClockStep::SystemClock {
            self.clock_step = ClockStep::SystemClockMultiplier;
        }
        self.current_time = value;
    }

    /// Gets the multiplier.
    pub fn get_multiplier(&self) -> f64 {
        self.multiplier
    }

    /// Sets the multiplier. Changes clock_step from SystemClock to SystemClockMultiplier.
    pub fn set_multiplier(&mut self, value: f64) {
        if self.multiplier == value {
            return;
        }
        if self.clock_step == ClockStep::SystemClock {
            self.clock_step = ClockStep::SystemClockMultiplier;
        }
        self.multiplier = value;
    }

    /// Gets the clock step.
    pub fn get_clock_step(&self) -> ClockStep {
        self.clock_step
    }

    /// Sets the clock step. If SystemClock, sets multiplier=1.0, should_animate=true, current_time=now.
    pub fn set_clock_step(&mut self, value: ClockStep) {
        if value == ClockStep::SystemClock {
            self.multiplier = 1.0;
            self.should_animate = true;
            self.current_time = JulianDate::now();
        }
        self.clock_step = value;
    }

    /// Gets whether the clock should animate.
    pub fn get_should_animate(&self) -> bool {
        self.should_animate
    }

    /// Sets whether the clock should animate. Changes clock_step from SystemClock to SystemClockMultiplier.
    pub fn set_should_animate(&mut self, value: bool) {
        if self.should_animate == value {
            return;
        }
        if self.clock_step == ClockStep::SystemClock {
            self.clock_step = ClockStep::SystemClockMultiplier;
        }
        self.should_animate = value;
    }

    /// Advances the clock based on the current configuration.
    /// Returns the new current time.
    pub fn tick(&mut self) -> JulianDate {
        let current_system_time = get_timestamp();
        let mut current_time = self.current_time.clone();

        if self.can_animate && self.should_animate {
            match self.clock_step {
                ClockStep::SystemClock => {
                    current_time = JulianDate::now();
                }
                ClockStep::TickDependent => {
                    current_time = JulianDate::add_seconds(&current_time, self.multiplier);
                    current_time = self.apply_clock_range(current_time);
                }
                ClockStep::SystemClockMultiplier => {
                    let milliseconds = current_system_time - self.last_system_time;
                    current_time = JulianDate::add_seconds(
                        &current_time,
                        self.multiplier * (milliseconds / 1000.0),
                    );
                    current_time = self.apply_clock_range(current_time);
                }
            }
        }

        self.current_time = current_time;
        self.last_system_time = current_system_time;
        self.on_tick.raise_event(&());
        self.current_time.clone()
    }

    fn apply_clock_range(&mut self, mut current_time: JulianDate) -> JulianDate {
        match self.clock_range {
            ClockRange::Clamped => {
                if JulianDate::less_than(&current_time, &self.start_time) {
                    current_time = self.start_time.clone();
                } else if JulianDate::greater_than(&current_time, &self.stop_time) {
                    current_time = self.stop_time.clone();
                    self.on_stop.raise_event(&());
                }
            }
            ClockRange::LoopStop => {
                if JulianDate::less_than(&current_time, &self.start_time) {
                    current_time = self.start_time.clone();
                }
                while JulianDate::greater_than(&current_time, &self.stop_time) {
                    let diff = JulianDate::seconds_difference(&current_time, &self.stop_time);
                    current_time = JulianDate::add_seconds(&self.start_time, diff);
                    self.on_stop.raise_event(&());
                }
            }
            ClockRange::Unbounded => {}
        }
        current_time
    }
}
