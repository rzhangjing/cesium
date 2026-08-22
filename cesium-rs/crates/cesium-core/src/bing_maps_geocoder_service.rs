//! Ported from `packages/engine/Source/Core/BingMapsGeocoderService.js`.
//!
//! Geocoder service using Bing Maps.

/// Geocoder service using Bing Maps API.
/// Skeleton: requires network I/O.
pub struct BingMapsGeocoderService {
    _key: String,
}

impl BingMapsGeocoderService {
    /// Creates a new Bing Maps geocoder service.
    pub fn new(key: String) -> Self {
        Self { _key: key }
    }
}
