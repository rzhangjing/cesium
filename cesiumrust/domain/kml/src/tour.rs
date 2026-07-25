//! KML Tour support.
//!
//! Maps to CesiumJS:
//! - `DataSources/KmlTour.js`
//! - `DataSources/KmlTourFlyTo.js`
//! - `DataSources/KmlTourWait.js`

use glam::DVec3;

// ============================================================================
// KmlTourFlyTo
// ============================================================================

/// A fly-to entry in a KML tour playlist.
///
/// Maps to CesiumJS `DataSources/KmlTourFlyTo.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlTourFlyTo {
    /// Duration of the fly-to in seconds.
    pub duration: f64,
    /// Target position (longitude, latitude, altitude) in degrees/meters.
    pub position: DVec3,
    /// Heading in degrees.
    pub heading: Option<f64>,
    /// Tilt in degrees.
    pub tilt: Option<f64>,
    /// Range (distance from target) in meters.
    pub range: Option<f64>,
    /// Whether to use great circle path (vs. linear).
    pub fly_to_mode: FlyToMode,
}

/// Fly-to interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlyToMode {
    /// Smooth camera path.
    #[default]
    Smooth,
    /// Bounce effect.
    Bounce,
}

impl KmlTourFlyTo {
    /// Create a new fly-to entry.
    pub fn new(duration: f64, position: DVec3) -> Self {
        Self {
            duration,
            position,
            heading: None,
            tilt: None,
            range: None,
            fly_to_mode: FlyToMode::Smooth,
        }
    }

    /// Set the heading.
    pub fn with_heading(mut self, heading: f64) -> Self {
        self.heading = Some(heading);
        self
    }

    /// Set the tilt.
    pub fn with_tilt(mut self, tilt: f64) -> Self {
        self.tilt = Some(tilt);
        self
    }

    /// Set the range.
    pub fn with_range(mut self, range: f64) -> Self {
        self.range = Some(range);
        self
    }

    /// Set the fly-to mode.
    pub fn with_mode(mut self, mode: FlyToMode) -> Self {
        self.fly_to_mode = mode;
        self
    }
}

// ============================================================================
// KmlTourWait
// ============================================================================

/// A wait entry in a KML tour playlist.
///
/// Maps to CesiumJS `DataSources/KmlTourWait.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlTourWait {
    /// Duration to wait in seconds.
    pub duration: f64,
}

impl KmlTourWait {
    /// Create a new wait entry.
    pub fn new(duration: f64) -> Self {
        Self { duration }
    }
}

// ============================================================================
// KmlTourEntry
// ============================================================================

/// A playlist entry (either fly-to or wait).
#[derive(Debug, Clone, PartialEq)]
pub enum KmlTourEntry {
    /// Fly to a position.
    FlyTo(KmlTourFlyTo),
    /// Wait for a duration.
    Wait(KmlTourWait),
}

impl KmlTourEntry {
    /// Get the duration of this entry.
    pub fn duration(&self) -> f64 {
        match self {
            Self::FlyTo(f) => f.duration,
            Self::Wait(w) => w.duration,
        }
    }
}

// ============================================================================
// KmlTour
// ============================================================================

/// A KML tour with a playlist of entries.
///
/// Maps to CesiumJS `DataSources/KmlTour.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlTour {
    /// Tour ID.
    pub id: String,
    /// Tour name.
    pub name: String,
    /// Playlist of entries.
    pub playlist: Vec<KmlTourEntry>,
    /// Current playlist index.
    pub playlist_index: usize,
    /// Whether the tour is playing.
    pub is_playing: bool,
}

impl KmlTour {
    /// Create a new tour.
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            playlist: Vec::new(),
            playlist_index: 0,
            is_playing: false,
        }
    }

    /// Add a fly-to entry to the playlist.
    pub fn add_fly_to(&mut self, fly_to: KmlTourFlyTo) {
        self.playlist.push(KmlTourEntry::FlyTo(fly_to));
    }

    /// Add a wait entry to the playlist.
    pub fn add_wait(&mut self, wait: KmlTourWait) {
        self.playlist.push(KmlTourEntry::Wait(wait));
    }

    /// Add a generic entry to the playlist.
    pub fn add_entry(&mut self, entry: KmlTourEntry) {
        self.playlist.push(entry);
    }

    /// Get the total duration of the tour.
    pub fn total_duration(&self) -> f64 {
        self.playlist.iter().map(|e| e.duration()).sum()
    }

    /// Get the number of entries.
    pub fn entry_count(&self) -> usize {
        self.playlist.len()
    }

    /// Start playing the tour.
    pub fn play(&mut self) {
        self.is_playing = true;
        self.playlist_index = 0;
    }

    /// Stop the tour.
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.playlist_index = 0;
    }

    /// Advance to the next entry. Returns false if tour is complete.
    pub fn advance(&mut self) -> bool {
        if self.playlist_index < self.playlist.len() {
            self.playlist_index += 1;
        }
        if self.playlist_index >= self.playlist.len() {
            self.is_playing = false;
            return false;
        }
        true
    }

    /// Get the current entry.
    pub fn current_entry(&self) -> Option<&KmlTourEntry> {
        self.playlist.get(self.playlist_index)
    }

    /// Whether the tour is complete.
    pub fn is_complete(&self) -> bool {
        self.playlist_index >= self.playlist.len()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kml_tour_fly_to() {
        let fly_to = KmlTourFlyTo::new(5.0, DVec3::new(-122.0, 37.0, 1000.0))
            .with_heading(45.0)
            .with_tilt(60.0)
            .with_range(5000.0)
            .with_mode(FlyToMode::Bounce);

        assert_eq!(fly_to.duration, 5.0);
        assert_eq!(fly_to.heading, Some(45.0));
        assert_eq!(fly_to.tilt, Some(60.0));
        assert_eq!(fly_to.range, Some(5000.0));
        assert_eq!(fly_to.fly_to_mode, FlyToMode::Bounce);
    }

    #[test]
    fn test_kml_tour_wait() {
        let wait = KmlTourWait::new(2.5);
        assert_eq!(wait.duration, 2.5);
    }

    #[test]
    fn test_kml_tour_entry_duration() {
        let fly_to = KmlTourEntry::FlyTo(KmlTourFlyTo::new(3.0, DVec3::ZERO));
        let wait = KmlTourEntry::Wait(KmlTourWait::new(1.5));

        assert_eq!(fly_to.duration(), 3.0);
        assert_eq!(wait.duration(), 1.5);
    }

    #[test]
    fn test_kml_tour_playlist() {
        let mut tour = KmlTour::new("tour1", "City Tour");

        tour.add_fly_to(KmlTourFlyTo::new(5.0, DVec3::new(-122.0, 37.0, 0.0)));
        tour.add_wait(KmlTourWait::new(2.0));
        tour.add_fly_to(KmlTourFlyTo::new(4.0, DVec3::new(-121.0, 38.0, 0.0)));

        assert_eq!(tour.entry_count(), 3);
        assert_eq!(tour.total_duration(), 11.0);
    }

    #[test]
    fn test_kml_tour_playback() {
        let mut tour = KmlTour::new("tour1", "Test");
        tour.add_fly_to(KmlTourFlyTo::new(1.0, DVec3::ZERO));
        tour.add_wait(KmlTourWait::new(1.0));

        assert!(!tour.is_playing);
        assert_eq!(tour.playlist_index, 0);

        tour.play();
        assert!(tour.is_playing);
        assert_eq!(tour.playlist_index, 0);
        assert!(!tour.is_complete());

        // Advance through entries
        assert!(tour.advance());
        assert_eq!(tour.playlist_index, 1);
        assert!(!tour.is_complete());

        assert!(!tour.advance()); // Last entry
        assert!(tour.is_complete());
        assert!(!tour.is_playing);
    }

    #[test]
    fn test_kml_tour_stop() {
        let mut tour = KmlTour::new("tour1", "Test");
        tour.add_fly_to(KmlTourFlyTo::new(1.0, DVec3::ZERO));
        tour.add_wait(KmlTourWait::new(1.0));

        tour.play();
        tour.advance();
        assert_eq!(tour.playlist_index, 1);

        tour.stop();
        assert!(!tour.is_playing);
        assert_eq!(tour.playlist_index, 0);
    }

    #[test]
    fn test_kml_tour_current_entry() {
        let mut tour = KmlTour::new("tour1", "Test");
        tour.add_fly_to(KmlTourFlyTo::new(5.0, DVec3::new(1.0, 2.0, 3.0)));
        tour.add_wait(KmlTourWait::new(2.0));

        let entry = tour.current_entry().unwrap();
        assert_eq!(entry.duration(), 5.0);

        tour.advance();
        let entry = tour.current_entry().unwrap();
        assert_eq!(entry.duration(), 2.0);
    }
}
