//! Ported from `packages/engine/Source/Core/OpenCageGeocoderService.js`.

/// A geocoder service using OpenCage.
pub struct OpenCageGeocoderService {
    _private: (),
}

impl OpenCageGeocoderService {
    /// Creates a new OpenCageGeocoderService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for OpenCageGeocoderService {
    fn default() -> Self { Self::new() }
}
