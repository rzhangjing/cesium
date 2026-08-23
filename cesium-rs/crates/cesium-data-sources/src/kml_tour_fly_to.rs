//! Ported from `packages/engine/Source/DataSources/KmlTourFlyTo.js`.

/// A KML tour entry that flies the camera to a specified viewpoint.
pub struct KmlTourFlyTo {
    /// The duration in seconds.
    pub duration: f64,
    /// The camera mode (flyTo or set).
    pub mode: String,
}

impl KmlTourFlyTo {
    /// Creates a new fly-to tour entry.
    pub fn new(duration: f64) -> Self {
        Self {
            duration,
            mode: String::from("flyTo"),
        }
    }
}
