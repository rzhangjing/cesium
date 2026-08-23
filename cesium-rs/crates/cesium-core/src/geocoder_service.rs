//! Ported from `packages/engine/Source/Core/GeocoderService.js`.

/// Interface for geocoder services.
pub struct GeocoderService {
    _private: (),
}

impl GeocoderService {
    /// Creates a new GeocoderService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GeocoderService {
    fn default() -> Self { Self::new() }
}
