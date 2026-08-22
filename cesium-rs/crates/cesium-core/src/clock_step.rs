//! Ported from `packages/engine/Source/Core/ClockStep.js`.

/// Constants to determine how much time advances with each call to `Clock::tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ClockStep {
    /// Advances the current time by a fixed step (multiplier seconds).
    TickDependent = 0,
    /// Advances the current time by system elapsed time × multiplier.
    SystemClockMultiplier = 1,
    /// Sets the clock to the current system time, ignoring all other settings.
    SystemClock = 2,
}
