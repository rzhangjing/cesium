//! Ported from `packages/engine/Source/DataSources/KmlTourWait.js`.

/// A KML tour entry that pauses for a specified duration
/// (mirror of `KmlTourWait`).
///
/// DEVIATION (playback): the JS `play`/`stop` timer against the scene
/// clock is not materialized; only the parsed value model is kept.
#[derive(Clone, Debug)]
pub struct KmlTourWait {
    /// The duration to wait in seconds.
    pub duration: Option<f64>,
}

impl KmlTourWait {
    /// Creates a new wait tour entry.
    pub fn new(duration: Option<f64>) -> Self {
        Self { duration }
    }
}
