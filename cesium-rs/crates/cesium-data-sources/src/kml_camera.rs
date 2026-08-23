//! Ported from `packages/engine/Source/DataSources/KmlCamera.js`.

/// Represents a camera definition in a KML tour.
pub struct KmlCamera {
    /// The longitude in degrees.
    pub longitude: f64,
    /// The latitude in degrees.
    pub latitude: f64,
    /// The altitude in meters.
    pub altitude: f64,
    /// The heading in degrees.
    pub heading: f64,
    /// The tilt in degrees.
    pub tilt: f64,
    /// The roll in degrees.
    pub roll: f64,
}

impl KmlCamera {
    /// Creates a new KML camera.
    pub fn new() -> Self {
        Self {
            longitude: 0.0, latitude: 0.0, altitude: 0.0,
            heading: 0.0, tilt: 0.0, roll: 0.0,
        }
    }
}

impl Default for KmlCamera {
    fn default() -> Self { Self::new() }
}
