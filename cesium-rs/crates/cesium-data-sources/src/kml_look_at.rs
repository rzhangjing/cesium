//! Ported from `packages/engine/Source/DataSources/KmlLookAt.js`.

/// Represents a LookAt viewpoint in a KML tour.
pub struct KmlLookAt {
    /// The longitude of the look-at point.
    pub longitude: f64,
    /// The latitude of the look-at point.
    pub latitude: f64,
    /// The altitude of the look-at point.
    pub altitude: f64,
    /// The heading in degrees.
    pub heading: f64,
    /// The tilt in degrees.
    pub tilt: f64,
    /// The range (distance from camera to point).
    pub range: f64,
}

impl KmlLookAt {
    /// Creates a new KML LookAt.
    pub fn new() -> Self {
        Self {
            longitude: 0.0, latitude: 0.0, altitude: 0.0,
            heading: 0.0, tilt: 0.0, range: 1000.0,
        }
    }
}

impl Default for KmlLookAt {
    fn default() -> Self { Self::new() }
}
