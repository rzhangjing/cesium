//! Ported from `packages/engine/Source/Core/PeliasGeocoderService.js`.

/// A geocoder service using Pelias.
pub struct PeliasGeocoderService {
    _private: (),
}

impl PeliasGeocoderService {
    /// Creates a new PeliasGeocoderService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PeliasGeocoderService {
    fn default() -> Self { Self::new() }
}
