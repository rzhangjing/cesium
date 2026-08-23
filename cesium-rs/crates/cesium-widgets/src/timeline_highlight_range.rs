//! Ported from `packages/widgets/Source/Timeline/TimelineHighlightRange.js`.

/// Represents a highlighted time range on the timeline.
pub struct TimelineHighlightRange {
    /// The start time.
    pub start: f64,
    /// The stop time.
    pub stop: f64,
    /// The color (RGBA).
    pub color: (f64, f64, f64, f64),
}

impl TimelineHighlightRange {
    /// Creates a new timeline highlight range.
    pub fn new(start: f64, stop: f64) -> Self {
        Self {
            start,
            stop,
            color: (0.0, 1.0, 0.0, 0.25),
        }
    }
}
