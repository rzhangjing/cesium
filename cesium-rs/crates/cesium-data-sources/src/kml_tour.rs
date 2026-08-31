//! Ported from `packages/engine/Source/DataSources/KmlTour.js`.

use crate::kml_camera::KmlCamera;
use crate::kml_look_at::KmlLookAt;
use crate::kml_tour_fly_to::KmlTourFlyTo;
use crate::kml_tour_wait::KmlTourWait;

/// The camera view targeted by a tour entry (mirror of the JS
/// `lookAt || camera` union).
#[derive(Clone, Debug)]
pub enum KmlTourView {
    /// A `<LookAt>` view.
    LookAt(KmlLookAt),
    /// A `<Camera>` view.
    Camera(KmlCamera),
}

/// A single entry in a KML tour playlist.
#[derive(Clone, Debug)]
pub enum KmlTourEntry {
    /// Fly to a location.
    FlyTo(KmlTourFlyTo),
    /// Wait for a duration.
    Wait(KmlTourWait),
}

/// Represents a KML tour containing a sequence of playlist entries
/// (mirror of `KmlTour`).
///
/// DEVIATION (playback): CesiumJS drives the scene camera through the
/// playlist with `play`/`stop`; this port only materializes the parsed
/// playlist value model.
#[derive(Clone, Debug)]
pub struct KmlTour {
    /// The name of the tour.
    pub name: Option<String>,
    /// The id of the tour.
    pub id: Option<String>,
    /// The playlist entries.
    pub playlist: Vec<KmlTourEntry>,
}

impl KmlTour {
    /// Creates a new KML tour.
    pub fn new(name: Option<String>, id: Option<String>) -> Self {
        Self {
            name,
            id,
            playlist: Vec::new(),
        }
    }

    /// Appends an entry to the playlist (mirror of `addPlaylistEntry`).
    pub fn add_playlist_entry(&mut self, entry: KmlTourEntry) {
        self.playlist.push(entry);
    }
}
