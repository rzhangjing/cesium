//! Ported from `packages/engine/Source/Core/GoogleGeocoderService.js`.
//!
//! Geocoder service using Google Maps.

/// Geocoder service using Google Maps API.
/// Skeleton: requires network I/O.
pub struct GoogleGeocoderService {
    _key: String,
}

impl GoogleGeocoderService {
    /// Creates a new Google geocoder service.
    pub fn new(key: String) -> Self {
        Self { _key: key }
    }
}
