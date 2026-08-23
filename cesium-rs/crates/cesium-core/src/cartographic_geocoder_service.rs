//! Ported from `packages/engine/Source/Core/CartographicGeocoderService.js`.

/// A geocoder service for cartographic coordinates.
pub struct CartographicGeocoderService {
    _private: (),
}

impl CartographicGeocoderService {
    /// Creates a new CartographicGeocoderService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CartographicGeocoderService {
    fn default() -> Self { Self::new() }
}
