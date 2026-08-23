//! Ported from `packages/engine/Source/DataSources/KmlTour.js`.

/// Represents a KML tour containing a sequence of tour entries.
pub struct KmlTour {
    /// The name of the tour.
    pub name: String,
    /// The tour entries.
    pub entries: Vec<KmlTourEntry>,
}

/// A single entry in a KML tour.
pub enum KmlTourEntry {
    /// Fly to a location.
    FlyTo(crate::kml_tour_fly_to::KmlTourFlyTo),
    /// Wait for a duration.
    Wait(crate::kml_tour_wait::KmlTourWait),
}

impl KmlTour {
    /// Creates a new KML tour.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entries: Vec::new(),
        }
    }
}
