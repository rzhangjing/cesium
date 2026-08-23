//! Ported from `packages/widgets/Source/Geocoder/Geocoder.js`.
//!
//! A widget for searching and flying to locations.

/// A widget for searching and flying to locations.
pub struct Geocoder {
    is_destroyed: bool,
}

impl Geocoder {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Geocoder {
    fn default() -> Self { Self::new() }
}
