//! Ported from `packages/engine/Source/Core/PeliasGeocoderService.js`.
//!
//! Geocoder service using Pelias.

/// Geocoder service using Pelias geocoding API.
/// Skeleton: requires network I/O.
pub struct PeliasGeocoderService {
    _url: String,
}

impl PeliasGeocoderService {
    /// Creates a new Pelias geocoder service.
    pub fn new(url: String) -> Self {
        Self { _url: url }
    }
}
