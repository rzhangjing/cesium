//! Timeline and animation controller.
//!
//! Maps to CesiumJS:
//! - `Widgets/Timeline/Timeline.js`
//! - `Widgets/Animation/AnimationViewModel.js`
//! - `Scene/Clock.js` (extended)

use cesium_time::clock::Clock;
use cesium_time::julian_date::JulianDate;

/// Timeline configuration.
#[derive(Debug, Clone)]
pub struct TimelineConfig {
    /// Start time of the timeline.
    pub start_time: JulianDate,
    /// End time of the timeline.
    pub end_time: JulianDate,
    /// Whether the timeline is visible.
    pub visible: bool,
    /// Zoom level (seconds per pixel).
    pub seconds_per_pixel: f64,
}

impl TimelineConfig {
    /// Creates a new timeline config.
    pub fn new(start_time: JulianDate, end_time: JulianDate) -> Self {
        let duration = end_time.seconds_difference(&start_time);
        Self {
            start_time,
            end_time,
            visible: true,
            seconds_per_pixel: duration / 1000.0, // Default: 1000px wide
        }
    }

    /// Returns the total duration in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.end_time.seconds_difference(&self.start_time)
    }
}

/// Timeline state for UI rendering.
#[derive(Debug, Clone)]
pub struct TimelineState {
    /// Current time position.
    pub current_time: JulianDate,
    /// Visible start time.
    pub visible_start: JulianDate,
    /// Visible end time.
    pub visible_end: JulianDate,
    /// Whether the playhead is at the start.
    pub at_start: bool,
    /// Whether the playhead is at the end.
    pub at_end: bool,
}

/// Animation controller for playback.
///
/// Maps to CesiumJS `Widgets/Animation/AnimationViewModel.js`
#[derive(Debug, Clone)]
pub struct AnimationController {
    /// The underlying clock.
    pub clock: Clock,
    /// Playback speed multiplier.
    pub speed_multiplier: f64,
    /// Whether playback is paused.
    pub paused: bool,
    /// Whether to loop playback.
    pub looping: bool,
    /// Shuttle ring angle (-1.0 to 1.0).
    pub shuttle_ring_angle: f64,
}

impl AnimationController {
    /// Creates a new animation controller.
    pub fn new(clock: Clock) -> Self {
        Self {
            clock,
            speed_multiplier: 1.0,
            paused: true,
            looping: true,
            shuttle_ring_angle: 0.0,
        }
    }

    /// Plays the animation forward.
    pub fn play(&mut self) {
        self.paused = false;
        self.clock.multiplier = self.speed_multiplier.abs();
    }

    /// Plays the animation in reverse.
    pub fn play_reverse(&mut self) {
        self.paused = false;
        self.speed_multiplier = -self.speed_multiplier.abs();
        self.clock.multiplier = self.speed_multiplier;
    }

    /// Pauses the animation.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Stops and resets to start.
    pub fn stop(&mut self) {
        self.paused = true;
        self.clock.current_time = self.clock.start_time;
    }

    /// Sets the speed multiplier.
    pub fn set_speed(&mut self, multiplier: f64) {
        self.speed_multiplier = multiplier;
        if !self.paused {
            self.clock.multiplier = if self.clock.multiplier >= 0.0 {
                multiplier.abs()
            } else {
                -multiplier.abs()
            };
        }
    }

    /// Sets the shuttle ring angle (-1.0 to 1.0).
    pub fn set_shuttle_ring(&mut self, angle: f64) {
        self.shuttle_ring_angle = angle.clamp(-1.0, 1.0);
        // Map angle to speed: 0 = paused, ±1 = max speed
        let max_speed = 1000.0; // 1000x real-time at full deflection
        self.speed_multiplier = self.shuttle_ring_angle * max_speed;
        if self.shuttle_ring_angle.abs() < 0.01 {
            self.paused = true;
        } else {
            self.paused = false;
            self.clock.multiplier = self.speed_multiplier;
        }
    }

    /// Advances the animation by delta seconds.
    pub fn tick(&mut self, delta_secs: f64) -> JulianDate {
        if self.paused {
            return self.clock.current_time;
        }

        let effective_delta = delta_secs * self.speed_multiplier;
        let new_time = self.clock.current_time.add_seconds(effective_delta);

        // Handle looping
        if self.looping {
            let duration = self.clock.stop_time.seconds_difference(&self.clock.start_time);
            if duration > 0.0 {
                let elapsed = new_time.seconds_difference(&self.clock.start_time);
                let wrapped = elapsed.rem_euclid(duration);
                self.clock.current_time = self.clock.start_time.add_seconds(wrapped);
            } else {
                self.clock.current_time = new_time;
            }
        } else {
            // Clamp to range
            if new_time.less_than(&self.clock.start_time) {
                self.clock.current_time = self.clock.start_time;
                self.paused = true;
            } else if self.clock.stop_time.less_than(&new_time) {
                self.clock.current_time = self.clock.stop_time;
                self.paused = true;
            } else {
                self.clock.current_time = new_time;
            }
        }

        self.clock.current_time
    }

    /// Seeks to a specific time.
    pub fn seek(&mut self, time: JulianDate) {
        self.clock.current_time = time;
    }

    /// Seeks to a fraction of the timeline (0.0 to 1.0).
    pub fn seek_fraction(&mut self, fraction: f64) {
        let duration = self.clock.stop_time.seconds_difference(&self.clock.start_time);
        let offset = duration * fraction.clamp(0.0, 1.0);
        self.clock.current_time = self.clock.start_time.add_seconds(offset);
    }

    /// Returns the current progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        let duration = self.clock.stop_time.seconds_difference(&self.clock.start_time);
        if duration <= 0.0 {
            return 0.0;
        }
        let elapsed = self.clock.current_time.seconds_difference(&self.clock.start_time);
        (elapsed / duration).clamp(0.0, 1.0)
    }

    /// Returns true if playing forward.
    pub fn is_playing_forward(&self) -> bool {
        !self.paused && self.speed_multiplier > 0.0
    }

    /// Returns true if playing in reverse.
    pub fn is_playing_reverse(&self) -> bool {
        !self.paused && self.speed_multiplier < 0.0
    }
}

/// Speed presets for the animation controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpeedPreset {
    /// 1x real-time.
    RealTime,
    /// 2x speed.
    Fast2x,
    /// 5x speed.
    Fast5x,
    /// 10x speed.
    Fast10x,
    /// 60x speed (1 minute per second).
    Fast60x,
    /// 3600x speed (1 hour per second).
    Fast3600x,
    /// 86400x speed (1 day per second).
    Fast86400x,
}

impl SpeedPreset {
    /// Returns the multiplier for this preset.
    pub fn multiplier(&self) -> f64 {
        match self {
            Self::RealTime => 1.0,
            Self::Fast2x => 2.0,
            Self::Fast5x => 5.0,
            Self::Fast10x => 10.0,
            Self::Fast60x => 60.0,
            Self::Fast3600x => 3600.0,
            Self::Fast86400x => 86400.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_clock() -> Clock {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(3600.0); // 1 hour
        Clock::new(start, stop, start)
    }

    #[test]
    fn test_timeline_config() {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let end = start.add_seconds(7200.0);
        let config = TimelineConfig::new(start, end);

        assert!((config.duration_seconds() - 7200.0).abs() < 1e-10);
        assert!(config.visible);
    }

    #[test]
    fn test_animation_controller_creation() {
        let clock = create_test_clock();
        let controller = AnimationController::new(clock);

        assert!(controller.paused);
        assert_eq!(controller.speed_multiplier, 1.0);
        assert!(controller.looping);
    }

    #[test]
    fn test_play_pause() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.play();
        assert!(!controller.paused);
        assert!(controller.is_playing_forward());

        controller.pause();
        assert!(controller.paused);
    }

    #[test]
    fn test_play_reverse() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.play_reverse();
        assert!(!controller.paused);
        assert!(controller.is_playing_reverse());
    }

    #[test]
    fn test_tick_advances_time() {
        let clock = create_test_clock();
        let start = clock.current_time;
        let mut controller = AnimationController::new(clock);
        controller.play();

        let new_time = controller.tick(1.0); // 1 second at 1x
        let elapsed = new_time.seconds_difference(&start);
        assert!((elapsed - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_tick_with_speed() {
        let clock = create_test_clock();
        let start = clock.current_time;
        let mut controller = AnimationController::new(clock);
        controller.set_speed(60.0);
        controller.play();

        let new_time = controller.tick(1.0); // 1 second at 60x
        let elapsed = new_time.seconds_difference(&start);
        assert!((elapsed - 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_tick_paused() {
        let clock = create_test_clock();
        let start = clock.current_time;
        let mut controller = AnimationController::new(clock);
        // paused by default

        let new_time = controller.tick(1.0);
        assert_eq!(new_time, start);
    }

    #[test]
    fn test_loop_wrapping() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);
        controller.looping = true;
        controller.play();

        // Advance past the end (3600 + 100 seconds)
        let new_time = controller.tick(3700.0);
        let elapsed = new_time.seconds_difference(&start);
        assert!((elapsed - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_no_loop_clamp() {
        let clock = create_test_clock();
        let stop = clock.stop_time;
        let mut controller = AnimationController::new(clock);
        controller.looping = false;
        controller.play();

        // Advance past the end
        let new_time = controller.tick(3700.0);
        assert_eq!(new_time, stop);
        assert!(controller.paused); // Auto-paused at end
    }

    #[test]
    fn test_seek() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);

        let target = start.add_seconds(1800.0);
        controller.seek(target);
        assert_eq!(controller.clock.current_time, target);
    }

    #[test]
    fn test_seek_fraction() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);

        controller.seek_fraction(0.5);
        let elapsed = controller.clock.current_time.seconds_difference(&start);
        assert!((elapsed - 1800.0).abs() < 1e-10);
    }

    #[test]
    fn test_progress() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.seek_fraction(0.25);
        assert!((controller.progress() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.set_shuttle_ring(0.5);
        assert!(!controller.paused);
        assert!(controller.speed_multiplier > 0.0);

        controller.set_shuttle_ring(0.0);
        assert!(controller.paused);
    }

    #[test]
    fn test_stop() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);
        controller.play();
        controller.tick(100.0);

        controller.stop();
        assert!(controller.paused);
        assert_eq!(controller.clock.current_time, start);
    }

    #[test]
    fn test_speed_presets() {
        assert_eq!(SpeedPreset::RealTime.multiplier(), 1.0);
        assert_eq!(SpeedPreset::Fast60x.multiplier(), 60.0);
        assert_eq!(SpeedPreset::Fast86400x.multiplier(), 86400.0);
    }
}
