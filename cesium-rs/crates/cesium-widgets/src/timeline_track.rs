//! Ported from `packages/widgets/Source/Timeline/TimelineTrack.js`.

/// Represents a track on the timeline for displaying entity data.
pub struct TimelineTrack {
    /// The name of the track.
    pub name: String,
    /// The start time.
    pub start: f64,
    /// The stop time.
    pub stop: f64,
}

impl TimelineTrack {
    /// Creates a new timeline track.
    pub fn new(name: &str, start: f64, stop: f64) -> Self {
        Self {
            name: name.to_string(),
            start,
            stop,
        }
    }
}
