//! Ported from `packages/widgets/Source/Animation/ClockViewModel.js`.

use crate::command::Command;

/// The view model for the clock, used by the Animation widget.
pub struct ClockViewModel {
    /// The current time as Julian date.
    pub current_time: f64,
    /// The start time.
    pub start_time: f64,
    /// The stop time.
    pub stop_time: f64,
    /// The multiplier.
    pub multiplier: f64,
    /// Whether the clock is playing.
    pub should_animate: bool,
    /// The play command.
    pub play_command: Command,
    /// The pause command.
    pub pause_command: Command,
}

impl ClockViewModel {
    /// Creates a new clock view model.
    pub fn new() -> Self {
        Self {
            current_time: 0.0,
            start_time: 0.0,
            stop_time: 0.0,
            multiplier: 1.0,
            should_animate: false,
            play_command: Command::empty(),
            pause_command: Command::empty(),
        }
    }
}

impl Default for ClockViewModel {
    fn default() -> Self { Self::new() }
}
