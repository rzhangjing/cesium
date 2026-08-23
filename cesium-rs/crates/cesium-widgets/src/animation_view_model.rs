//! Ported from `packages/widgets/Source/Animation/AnimationViewModel.js`.
//!
//! The view model for the Animation widget.

use crate::command::Command;

/// The view model for the Animation widget.
///
/// In CesiumJS, AnimationViewModel.js is ~400 lines. It controls:
/// - Play/pause toggle
/// - Shuttle ring (speed control with discrete "tick" modes)
/// - Time display formatting
/// - Real-time vs. simulated time
///
/// The shuttle ring has 11 positions:
/// - Positions 0-5: negative speeds (reverse playback)
/// - Position 6: paused
/// - Positions 7-10: positive speeds (forward playback)
pub struct AnimationViewModel {
    /// Whether the animation is playing.
    is_playing: bool,
    /// The multiplier (speed) of animation.
    multiplier: f64,
    /// The shuttle ring angle (0-360, maps to speed).
    shuttle_ring_angle: f64,
    /// Whether the shuttle ring is in "tick" mode (discrete speeds).
    shuttle_ring_ticks: bool,
    /// The tooltip for the play button.
    play_tooltip: String,
    /// The tooltip for the pause button.
    pause_tooltip: String,
    /// The play command.
    pub play_command: Command,
    /// The pause command.
    pub pause_command: Command,
    is_destroyed: bool,
}

/// Shuttle ring angle to multiplier mapping.
///
/// In CesiumJS, the shuttle ring maps angles to specific multipliers:
/// - 0°: paused (multiplier = 0)
/// - 5°: 0.1x
/// - 10°: 0.25x
/// - 15°: 0.5x
/// - 20°: 1x
/// - 25°: 2x
/// - 30°: 5x
/// - 35°: 10x
/// - 40°: 50x
/// - 45°: 100x
/// - 50°: 1000x
const SHUTTLE_ANGLES: &[f64] = &[
    0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0,
];

const SHUTTLE_MULTIPLIERS: &[f64] = &[
    0.0, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 50.0, 100.0, 1000.0,
];

impl AnimationViewModel {
    /// Creates a new animation view model.
    pub fn new() -> Self {
        Self {
            is_playing: false,
            multiplier: 1.0,
            shuttle_ring_angle: 20.0, // 1x speed
            shuttle_ring_ticks: false,
            play_tooltip: String::from("Play"),
            pause_tooltip: String::from("Pause"),
            play_command: Command::empty(),
            pause_command: Command::empty(),
            is_destroyed: false,
        }
    }

    /// Returns whether the animation is playing.
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Toggles play/pause.
    pub fn toggle_play(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Returns the animation speed multiplier.
    pub fn multiplier(&self) -> f64 {
        self.multiplier
    }

    /// Sets the animation speed multiplier.
    pub fn set_multiplier(&mut self, multiplier: f64) {
        self.multiplier = multiplier;
    }

    /// Returns the shuttle ring angle.
    pub fn shuttle_ring_angle(&self) -> f64 {
        self.shuttle_ring_angle
    }

    /// Sets the shuttle ring angle and updates the multiplier accordingly.
    pub fn set_shuttle_ring_angle(&mut self, angle: f64) {
        self.shuttle_ring_angle = angle.clamp(0.0, 50.0);
        // Find the closest shuttle position
        let mut closest_idx = 0;
        let mut closest_dist = f64::MAX;
        for (i, &a) in SHUTTLE_ANGLES.iter().enumerate() {
            let dist = (a - self.shuttle_ring_angle).abs();
            if dist < closest_dist {
                closest_dist = dist;
                closest_idx = i;
            }
        }
        self.multiplier = SHUTTLE_MULTIPLIERS[closest_idx];
    }

    /// Returns whether the shuttle ring uses discrete ticks.
    pub fn shuttle_ring_ticks(&self) -> bool {
        self.shuttle_ring_ticks
    }

    /// Sets whether the shuttle ring uses discrete ticks.
    pub fn set_shuttle_ring_ticks(&mut self, ticks: bool) {
        self.shuttle_ring_ticks = ticks;
    }

    /// Returns whether this view model has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this view model.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for AnimationViewModel {
    fn default() -> Self {
        Self::new()
    }
}
