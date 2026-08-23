//! Ported from `packages/engine/Source/Core/BingMapsGeocoderService.js`.

/// A geocoder service using Bing Maps.
pub struct BingMapsGeocoderService {
    _private: (),
}

impl BingMapsGeocoderService {
    /// Creates a new BingMapsGeocoderService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BingMapsGeocoderService {
    fn default() -> Self { Self::new() }
}
