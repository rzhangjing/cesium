//! Ported from `packages/engine/Source/Core/ClockRange.js`.

/// Constants used by `Clock::tick` to determine behavior
/// when `start_time` or `stop_time` is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ClockRange {
    /// `tick` will always advance the clock in its current direction.
    Unbounded = 0,
    /// When `start_time` or `stop_time` is reached, `tick` will not advance
    /// `current_time` any further.
    Clamped = 1,
    /// When `stop_time` is reached, `tick` will advance `current_time` to the
    /// opposite end of the interval.
    LoopStop = 2,
}
