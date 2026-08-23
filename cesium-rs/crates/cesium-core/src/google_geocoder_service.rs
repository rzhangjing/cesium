//! Ported from `packages/engine/Source/Core/GoogleGeocoderService.js`.

/// A geocoder service using Google.
pub struct GoogleGeocoderService {
    _private: (),
}

impl GoogleGeocoderService {
    /// Creates a new GoogleGeocoderService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GoogleGeocoderService {
    fn default() -> Self { Self::new() }
}
