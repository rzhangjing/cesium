//! Ported from `packages/engine/Source/DataSources/KmlTourWait.js`.

/// A KML tour entry that pauses for a specified duration.
pub struct KmlTourWait {
    /// The duration to wait in seconds.
    pub duration: f64,
}

impl KmlTourWait {
    /// Creates a new wait tour entry.
    pub fn new(duration: f64) -> Self {
        Self { duration }
    }
}
