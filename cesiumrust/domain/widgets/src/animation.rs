//! Animation widget view model.
//!
//! Maps to CesiumJS `Animation/AnimationViewModel.js`.

/// Shuttle ring angle constants.
pub const REALTIME_SHUTTLE_RING_ANGLE: f64 = 15.0;
pub const MAX_SHUTTLE_RING_ANGLE: f64 = 105.0;

/// Default shuttle ring ticks (speed multipliers).
pub const DEFAULT_SHUTTLE_RING_TICKS: &[f64] = &[
    -1000.0, -100.0, -50.0, -25.0, -10.0, -5.0, -2.0, -1.0,
    1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 1000.0,
];

/// Month names for date display.
pub const MONTH_NAMES: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Shuttle ring angle ↔ multiplier conversion.
///
/// Maps to CesiumJS AnimationViewModel angle/multiplier functions.
#[derive(Debug, Clone)]
pub struct ShuttleRing {
    /// The shuttle ring tick values.
    pub ticks: Vec<f64>,
}

impl Default for ShuttleRing {
    fn default() -> Self {
        Self {
            ticks: DEFAULT_SHUTTLE_RING_TICKS.to_vec(),
        }
    }
}

impl ShuttleRing {
    /// Create with custom ticks.
    pub fn with_ticks(ticks: Vec<f64>) -> Self {
        let mut sorted = ticks;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self { ticks: sorted }
    }

    /// Convert a shuttle ring angle to a speed multiplier.
    ///
    /// Angle range: [-MAX_SHUTTLE_RING_ANGLE, MAX_SHUTTLE_RING_ANGLE]
    /// - Angles in [-15, 15] map linearly to [-1, 1]
    /// - Angles outside use logarithmic scale
    pub fn angle_to_multiplier(&self, angle: f64) -> f64 {
        if angle.abs() <= REALTIME_SHUTTLE_RING_ANGLE {
            return angle / REALTIME_SHUTTLE_RING_ANGLE;
        }

        let minp = REALTIME_SHUTTLE_RING_ANGLE;
        let maxp = MAX_SHUTTLE_RING_ANGLE;
        let minv = 0.0_f64;

        if angle > 0.0 {
            let maxv = self.ticks.last().copied().unwrap_or(1000.0).ln();
            let scale = (maxv - minv) / (maxp - minp);
            (minv + scale * (angle - minp)).exp()
        } else {
            let maxv = (-self.ticks.first().copied().unwrap_or(-1000.0)).ln();
            let scale = (maxv - minv) / (maxp - minp);
            -((minv + scale * (angle.abs() - minp)).exp())
        }
    }

    /// Convert a speed multiplier to a shuttle ring angle.
    pub fn multiplier_to_angle(&self, multiplier: f64, is_system_clock: bool) -> f64 {
        if is_system_clock {
            return REALTIME_SHUTTLE_RING_ANGLE;
        }

        if multiplier.abs() <= 1.0 {
            return multiplier * REALTIME_SHUTTLE_RING_ANGLE;
        }

        let fastest = self.ticks.last().copied().unwrap_or(1000.0);
        let clamped = multiplier.clamp(-fastest, fastest);

        let minp = REALTIME_SHUTTLE_RING_ANGLE;
        let maxp = MAX_SHUTTLE_RING_ANGLE;
        let minv = 0.0_f64;

        if clamped > 0.0 {
            let maxv = fastest.ln();
            let scale = (maxv - minv) / (maxp - minp);
            (clamped.ln() - minv) / scale + minp
        } else {
            let maxv = (-self.ticks.first().copied().unwrap_or(-1000.0)).ln();
            let scale = (maxv - minv) / (maxp - minp);
            -((clamped.abs().ln() - minv) / scale + minp)
        }
    }

    /// Get the typical multiplier index for a given multiplier.
    pub fn get_typical_multiplier_index(&self, multiplier: f64) -> usize {
        match self.ticks.binary_search_by(|t| {
            t.partial_cmp(&multiplier).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            Ok(idx) => idx,
            Err(idx) => idx,
        }
    }
}

/// Animation widget view model.
///
/// Controls time playback with play/pause, speed multiplier, and shuttle ring.
#[derive(Debug, Clone)]
pub struct AnimationViewModel {
    /// Whether animation is playing.
    pub is_playing: bool,
    /// Current speed multiplier (1.0 = real-time).
    pub multiplier: f64,
    /// Current shuttle ring angle in degrees.
    pub shuttle_ring_angle: f64,
    /// Whether the clock is in system clock mode.
    pub is_system_clock: bool,
    /// Current time as seconds since J2000 epoch.
    pub current_time: f64,
    /// The shuttle ring converter.
    pub shuttle_ring: ShuttleRing,
}

impl Default for AnimationViewModel {
    fn default() -> Self {
        Self {
            is_playing: false,
            multiplier: 1.0,
            shuttle_ring_angle: REALTIME_SHUTTLE_RING_ANGLE,
            is_system_clock: false,
            current_time: 0.0,
            shuttle_ring: ShuttleRing::default(),
        }
    }
}

impl AnimationViewModel {
    /// Create a new animation view model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle play/pause.
    pub fn toggle_play(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Play the animation.
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Pause the animation.
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Play in reverse.
    pub fn play_reverse(&mut self) {
        self.is_playing = true;
        if self.multiplier > 0.0 {
            self.multiplier = -self.multiplier;
        }
    }

    /// Play forward.
    pub fn play_forward(&mut self) {
        self.is_playing = true;
        if self.multiplier < 0.0 {
            self.multiplier = -self.multiplier;
        }
    }

    /// Set the speed multiplier.
    pub fn set_multiplier(&mut self, multiplier: f64) {
        self.multiplier = multiplier;
        self.shuttle_ring_angle = self.shuttle_ring.multiplier_to_angle(multiplier, self.is_system_clock);
    }

    /// Set the shuttle ring angle.
    pub fn set_shuttle_ring_angle(&mut self, angle: f64) {
        let clamped = angle.clamp(-MAX_SHUTTLE_RING_ANGLE, MAX_SHUTTLE_RING_ANGLE);
        self.shuttle_ring_angle = clamped;
        self.multiplier = self.shuttle_ring.angle_to_multiplier(clamped);
    }

    /// Set system clock mode.
    pub fn set_system_clock(&mut self, enabled: bool) {
        self.is_system_clock = enabled;
        if enabled {
            self.shuttle_ring_angle = REALTIME_SHUTTLE_RING_ANGLE;
        }
    }

    /// Update the current time.
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }

    /// Format the current time as a date string.
    pub fn format_date(&self) -> String {
        // Simplified: convert seconds since J2000 to a date string
        // J2000 epoch is 2000-01-01 12:00:00 TT
        let j2000_unix = 946728000.0; // Unix timestamp of J2000
        let unix_time = self.current_time + j2000_unix;
        let days = (unix_time / 86400.0).floor() as i64;

        // Simple date calculation (approximate)
        let years_since_1970 = days / 365;
        let year = 1970 + years_since_1970;
        let day_of_year = days % 365;
        let month = (day_of_year / 30).clamp(0, 11) as usize;
        let day = (day_of_year % 30) + 1;

        format!("{} {}, {}", MONTH_NAMES[month], day, year)
    }

    /// Format the current time as a time string.
    pub fn format_time(&self) -> String {
        let j2000_unix = 946728000.0;
        let unix_time = self.current_time + j2000_unix;
        let seconds_in_day = unix_time % 86400.0;
        let hours = (seconds_in_day / 3600.0).floor() as i32;
        let minutes = ((seconds_in_day % 3600.0) / 60.0).floor() as i32;
        let seconds = (seconds_in_day % 60.0).floor() as i32;

        format!("{:02}:{:02}:{:02} UTC", hours, minutes, seconds)
    }

    /// Get the multiplier display string.
    pub fn multiplier_string(&self) -> String {
        if self.multiplier == 1.0 {
            "1x".to_string()
        } else if self.multiplier == -1.0 {
            "-1x".to_string()
        } else if self.multiplier.abs() < 1.0 {
            format!("{:.2}x", self.multiplier)
        } else {
            format!("{:.0}x", self.multiplier)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shuttle_ring_default() {
        let ring = ShuttleRing::default();
        assert_eq!(ring.ticks.len(), 16);
        assert!(ring.ticks[0] < 0.0);
        assert!(ring.ticks[15] > 0.0);
    }

    #[test]
    fn test_shuttle_ring_angle_to_multiplier_linear() {
        let ring = ShuttleRing::default();
        // In linear range [-15, 15]
        assert!((ring.angle_to_multiplier(0.0)).abs() < 1e-10);
        assert!((ring.angle_to_multiplier(15.0) - 1.0).abs() < 1e-10);
        assert!((ring.angle_to_multiplier(-15.0) - (-1.0)).abs() < 1e-10);
        assert!((ring.angle_to_multiplier(7.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring_angle_to_multiplier_log() {
        let ring = ShuttleRing::default();
        // At max angle, should be near max tick
        let max_mult = ring.angle_to_multiplier(MAX_SHUTTLE_RING_ANGLE);
        assert!(max_mult > 100.0);

        let min_mult = ring.angle_to_multiplier(-MAX_SHUTTLE_RING_ANGLE);
        assert!(min_mult < -100.0);
    }

    #[test]
    fn test_shuttle_ring_multiplier_to_angle() {
        let ring = ShuttleRing::default();
        // Multiplier 1.0 should give angle 15
        assert!((ring.multiplier_to_angle(1.0, false) - 15.0).abs() < 1e-10);
        assert!((ring.multiplier_to_angle(-1.0, false) - (-15.0)).abs() < 1e-10);
        assert!((ring.multiplier_to_angle(0.5, false) - 7.5).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring_system_clock() {
        let ring = ShuttleRing::default();
        // System clock always returns realtime angle
        assert!((ring.multiplier_to_angle(100.0, true) - REALTIME_SHUTTLE_RING_ANGLE).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring_roundtrip() {
        let ring = ShuttleRing::default();
        for angle in [-100.0, -50.0, -15.0, 0.0, 15.0, 50.0, 100.0] {
            let mult = ring.angle_to_multiplier(angle);
            let angle_back = ring.multiplier_to_angle(mult, false);
            assert!((angle - angle_back).abs() < 0.1, "angle {} -> mult {} -> angle {}", angle, mult, angle_back);
        }
    }

    #[test]
    fn test_animation_view_model_default() {
        let vm = AnimationViewModel::new();
        assert!(!vm.is_playing);
        assert_eq!(vm.multiplier, 1.0);
        assert!(!vm.is_system_clock);
    }

    #[test]
    fn test_animation_toggle_play() {
        let mut vm = AnimationViewModel::new();
        assert!(!vm.is_playing);
        vm.toggle_play();
        assert!(vm.is_playing);
        vm.toggle_play();
        assert!(!vm.is_playing);
    }

    #[test]
    fn test_animation_play_reverse() {
        let mut vm = AnimationViewModel::new();
        vm.multiplier = 5.0;
        vm.play_reverse();
        assert!(vm.is_playing);
        assert!(vm.multiplier < 0.0);
    }

    #[test]
    fn test_animation_play_forward() {
        let mut vm = AnimationViewModel::new();
        vm.multiplier = -5.0;
        vm.play_forward();
        assert!(vm.is_playing);
        assert!(vm.multiplier > 0.0);
    }

    #[test]
    fn test_animation_set_multiplier() {
        let mut vm = AnimationViewModel::new();
        vm.set_multiplier(10.0);
        assert_eq!(vm.multiplier, 10.0);
        assert!(vm.shuttle_ring_angle > REALTIME_SHUTTLE_RING_ANGLE);
    }

    #[test]
    fn test_animation_set_shuttle_ring_angle() {
        let mut vm = AnimationViewModel::new();
        vm.set_shuttle_ring_angle(50.0);
        assert_eq!(vm.shuttle_ring_angle, 50.0);
        assert!(vm.multiplier > 1.0);
    }

    #[test]
    fn test_animation_multiplier_string() {
        let mut vm = AnimationViewModel::new();
        assert_eq!(vm.multiplier_string(), "1x");
        vm.multiplier = -1.0;
        assert_eq!(vm.multiplier_string(), "-1x");
        vm.multiplier = 10.0;
        assert_eq!(vm.multiplier_string(), "10x");
        vm.multiplier = 0.5;
        assert_eq!(vm.multiplier_string(), "0.50x");
    }

    #[test]
    fn test_animation_format_time() {
        let mut vm = AnimationViewModel::new();
        vm.current_time = 0.0; // J2000 epoch = 2000-01-01 12:00:00
        let time_str = vm.format_time();
        assert!(time_str.contains("UTC"));
    }

    #[test]
    fn test_typical_multiplier_index() {
        let ring = ShuttleRing::default();
        let idx = ring.get_typical_multiplier_index(1.0);
        assert!(idx < ring.ticks.len());
    }
}
