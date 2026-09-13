//! Timeline widget model.
//!
//! Maps to CesiumJS `Timeline/Timeline.js`.

/// Timeline tic scales in seconds.
pub const TIMELINE_TIC_SCALES: &[f64] = &[
    0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.25, 0.5,
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0,
    60.0,      // 1 min
    120.0,     // 2 min
    300.0,     // 5 min
    600.0,     // 10 min
    900.0,     // 15 min
    1800.0,    // 30 min
    3600.0,    // 1 hr
    7200.0,    // 2 hr
    14400.0,   // 4 hr
    21600.0,   // 6 hr
    43200.0,   // 12 hr
    86400.0,   // 24 hr
    172800.0,  // 2 days
    345600.0,  // 4 days
    604800.0,  // 7 days
    1296000.0, // 15 days
    2592000.0, // 30 days
    5184000.0, // 60 days
    7776000.0, // 90 days
    15552000.0,  // 180 days
    31536000.0,  // 365 days
    63072000.0,  // 2 years
    126144000.0, // 4 years
    157680000.0, // 5 years
    315360000.0, // 10 years
    630720000.0, // 20 years
    1261440000.0, // 40 years
    1576800000.0, // 50 years
    3153600000.0, // 100 years
    6307200000.0, // 200 years
    12614400000.0, // 400 years
    15768000000.0, // 500 years
    31536000000.0, // 1000 years
];

/// A timeline tic scale with label formatting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineTicScale {
    /// The scale value in seconds.
    pub seconds: f64,
    /// Whether this is a major tic.
    pub is_major: bool,
}

impl TimelineTicScale {
    /// Get the appropriate tic scale for a given time span and pixel width.
    pub fn for_span_and_width(span_seconds: f64, width_pixels: f64, min_tic_spacing: f64) -> Self {
        let ideal_tic_seconds = span_seconds * min_tic_spacing / width_pixels;

        for &scale in TIMELINE_TIC_SCALES {
            if scale >= ideal_tic_seconds {
                let is_major = scale >= 3600.0; // Major tics at 1hr+
                return Self { seconds: scale, is_major };
            }
        }

        // Fallback to largest scale
        Self {
            seconds: *TIMELINE_TIC_SCALES.last().unwrap(),
            is_major: true,
        }
    }

    /// Format a time value for this tic scale.
    pub fn format_label(&self, time_seconds: f64) -> String {
        if self.seconds >= 86400.0 {
            // Days or longer - show date
            let days = (time_seconds / 86400.0).floor() as i64;
            format!("Day {}", days)
        } else if self.seconds >= 3600.0 {
            // Hours
            let hours = (time_seconds / 3600.0).floor() as i64;
            let minutes = ((time_seconds % 3600.0) / 60.0).floor() as i64;
            format!("{:02}:{:02}", hours, minutes)
        } else if self.seconds >= 60.0 {
            // Minutes
            let minutes = (time_seconds / 60.0).floor() as i64;
            let seconds = (time_seconds % 60.0).floor() as i64;
            format!("{:02}:{:02}", minutes, seconds)
        } else {
            // Seconds
            format!("{:.1}s", time_seconds)
        }
    }
}

/// A track on the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTrack {
    /// Track name/identifier.
    pub name: String,
    /// Start time in seconds since epoch.
    pub start_time: f64,
    /// End time in seconds since epoch.
    pub end_time: f64,
    /// Track color as RGBA [0, 1].
    pub color: [f64; 4],
    /// Height of the track in pixels.
    pub height: f64,
}

impl TimelineTrack {
    /// Create a new timeline track.
    pub fn new(name: impl Into<String>, start_time: f64, end_time: f64) -> Self {
        Self {
            name: name.into(),
            start_time,
            end_time,
            color: [0.5, 0.5, 1.0, 1.0],
            height: 10.0,
        }
    }

    /// Set the track color.
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// Set the track height.
    pub fn with_height(mut self, height: f64) -> Self {
        self.height = height;
        self
    }

    /// Check if a time is within this track.
    pub fn contains_time(&self, time: f64) -> bool {
        time >= self.start_time && time <= self.end_time
    }

    /// Get the duration of the track.
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }
}

/// A highlight range on the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineHighlightRange {
    /// Start time in seconds.
    pub start_time: f64,
    /// End time in seconds.
    pub end_time: f64,
    /// Highlight color as RGBA [0, 1].
    pub color: [f64; 4],
}

impl TimelineHighlightRange {
    /// Create a new highlight range.
    pub fn new(start_time: f64, end_time: f64) -> Self {
        Self {
            start_time,
            end_time,
            color: [1.0, 1.0, 0.0, 0.3],
        }
    }

    /// Set the highlight color.
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// Check if a time is within this range.
    pub fn contains_time(&self, time: f64) -> bool {
        time >= self.start_time && time <= self.end_time
    }

    /// Get the duration.
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }
}

/// Timeline widget model.
///
/// Displays and controls the current scene time with tracks and highlights.
#[derive(Debug, Clone)]
pub struct Timeline {
    /// Start time of the visible range (seconds since epoch).
    pub start_time: f64,
    /// End time of the visible range (seconds since epoch).
    pub end_time: f64,
    /// Current time (seconds since epoch).
    pub current_time: f64,
    /// Tracks on the timeline.
    pub tracks: Vec<TimelineTrack>,
    /// Highlight ranges.
    pub highlight_ranges: Vec<TimelineHighlightRange>,
    /// Whether the timeline is visible.
    pub show: bool,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            end_time: 86400.0, // 1 day
            current_time: 0.0,
            tracks: Vec::new(),
            highlight_ranges: Vec::new(),
            show: true,
        }
    }
}

impl Timeline {
    /// Create a new timeline with a time range.
    pub fn new(start_time: f64, end_time: f64) -> Self {
        Self {
            start_time,
            end_time,
            current_time: start_time,
            ..Default::default()
        }
    }

    /// Get the visible time span in seconds.
    pub fn span(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// Set the visible time range.
    pub fn set_range(&mut self, start_time: f64, end_time: f64) {
        self.start_time = start_time;
        self.end_time = end_time;
        self.current_time = self.current_time.clamp(start_time, end_time);
    }

    /// Set the current time.
    pub fn set_current_time(&mut self, time: f64) {
        self.current_time = time.clamp(self.start_time, self.end_time);
    }

    /// Convert a time to a normalized position [0, 1] on the timeline.
    pub fn time_to_position(&self, time: f64) -> f64 {
        let span = self.span();
        if span <= 0.0 {
            return 0.0;
        }
        ((time - self.start_time) / span).clamp(0.0, 1.0)
    }

    /// Convert a normalized position [0, 1] to a time.
    pub fn position_to_time(&self, position: f64) -> f64 {
        let clamped = position.clamp(0.0, 1.0);
        self.start_time + clamped * self.span()
    }

    /// Add a track.
    pub fn add_track(&mut self, track: TimelineTrack) {
        self.tracks.push(track);
    }

    /// Remove a track by name.
    pub fn remove_track(&mut self, name: &str) -> bool {
        let len_before = self.tracks.len();
        self.tracks.retain(|t| t.name != name);
        self.tracks.len() < len_before
    }

    /// Add a highlight range.
    pub fn add_highlight(&mut self, highlight: TimelineHighlightRange) {
        self.highlight_ranges.push(highlight);
    }

    /// Clear all highlights.
    pub fn clear_highlights(&mut self) {
        self.highlight_ranges.clear();
    }

    /// Zoom in by a factor (centered on current time).
    pub fn zoom_in(&mut self, factor: f64) {
        let new_span = self.span() / factor.max(1.01);
        let center = self.current_time;
        self.start_time = center - new_span / 2.0;
        self.end_time = center + new_span / 2.0;
    }

    /// Zoom out by a factor (centered on current time).
    pub fn zoom_out(&mut self, factor: f64) {
        let new_span = self.span() * factor.max(1.01);
        let center = self.current_time;
        self.start_time = center - new_span / 2.0;
        self.end_time = center + new_span / 2.0;
    }

    /// Pan by a fraction of the visible span.
    pub fn pan(&mut self, fraction: f64) {
        let delta = self.span() * fraction;
        self.start_time += delta;
        self.end_time += delta;
    }

    /// Get the appropriate tic scale for the current view.
    pub fn tic_scale(&self, width_pixels: f64) -> TimelineTicScale {
        TimelineTicScale::for_span_and_width(self.span(), width_pixels, 50.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_default() {
        let timeline = Timeline::default();
        assert_eq!(timeline.start_time, 0.0);
        assert_eq!(timeline.end_time, 86400.0);
        assert!(timeline.show);
    }

    #[test]
    fn test_timeline_new() {
        let timeline = Timeline::new(1000.0, 2000.0);
        assert_eq!(timeline.start_time, 1000.0);
        assert_eq!(timeline.end_time, 2000.0);
        assert_eq!(timeline.current_time, 1000.0);
    }

    #[test]
    fn test_timeline_span() {
        let timeline = Timeline::new(0.0, 3600.0);
        assert_eq!(timeline.span(), 3600.0);
    }

    #[test]
    fn test_timeline_time_to_position() {
        let timeline = Timeline::new(0.0, 100.0);
        assert!((timeline.time_to_position(0.0)).abs() < 1e-10);
        assert!((timeline.time_to_position(50.0) - 0.5).abs() < 1e-10);
        assert!((timeline.time_to_position(100.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_timeline_position_to_time() {
        let timeline = Timeline::new(0.0, 100.0);
        assert!((timeline.position_to_time(0.0)).abs() < 1e-10);
        assert!((timeline.position_to_time(0.5) - 50.0).abs() < 1e-10);
        assert!((timeline.position_to_time(1.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_timeline_set_current_time_clamped() {
        let mut timeline = Timeline::new(0.0, 100.0);
        timeline.set_current_time(150.0);
        assert_eq!(timeline.current_time, 100.0);

        timeline.set_current_time(-50.0);
        assert_eq!(timeline.current_time, 0.0);
    }

    #[test]
    fn test_timeline_tracks() {
        let mut timeline = Timeline::default();
        timeline.add_track(TimelineTrack::new("track1", 0.0, 100.0));
        timeline.add_track(TimelineTrack::new("track2", 50.0, 150.0));

        assert_eq!(timeline.tracks.len(), 2);
        assert!(timeline.remove_track("track1"));
        assert_eq!(timeline.tracks.len(), 1);
        assert!(!timeline.remove_track("nonexistent"));
    }

    #[test]
    fn test_timeline_track_contains() {
        let track = TimelineTrack::new("test", 10.0, 20.0);
        assert!(track.contains_time(15.0));
        assert!(!track.contains_time(5.0));
        assert!(!track.contains_time(25.0));
        assert_eq!(track.duration(), 10.0);
    }

    #[test]
    fn test_timeline_highlights() {
        let mut timeline = Timeline::default();
        timeline.add_highlight(TimelineHighlightRange::new(0.0, 100.0));
        assert_eq!(timeline.highlight_ranges.len(), 1);

        timeline.clear_highlights();
        assert!(timeline.highlight_ranges.is_empty());
    }

    #[test]
    fn test_timeline_zoom() {
        let mut timeline = Timeline::new(0.0, 100.0);
        timeline.current_time = 50.0;

        timeline.zoom_in(2.0);
        assert!(timeline.span() < 100.0);
        assert!((timeline.current_time - 50.0).abs() < 1e-10);

        timeline.zoom_out(2.0);
        assert!(timeline.span() > 50.0);
    }

    #[test]
    fn test_timeline_pan() {
        let mut timeline = Timeline::new(0.0, 100.0);
        timeline.pan(0.1);
        assert!((timeline.start_time - 10.0).abs() < 1e-10);
        assert!((timeline.end_time - 110.0).abs() < 1e-10);
    }

    #[test]
    fn test_tic_scale() {
        let scale = TimelineTicScale::for_span_and_width(3600.0, 1000.0, 50.0);
        assert!(scale.seconds >= 1.0);

        let scale2 = TimelineTicScale::for_span_and_width(86400.0, 1000.0, 50.0);
        assert!(scale2.seconds >= 60.0);
    }

    #[test]
    fn test_tic_scale_format() {
        let scale = TimelineTicScale { seconds: 3600.0, is_major: true };
        let label = scale.format_label(7200.0);
        assert!(label.contains("02"));

        let scale2 = TimelineTicScale { seconds: 60.0, is_major: false };
        let label2 = scale2.format_label(125.0);
        assert!(label2.contains("02"));
    }
}
