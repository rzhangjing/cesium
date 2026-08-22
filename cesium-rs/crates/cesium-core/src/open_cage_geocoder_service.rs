//! Ported from `packages/engine/Source/Core/OpenCageGeocoderService.js`.
//!
//! Geocoder service using OpenCage.

/// Geocoder service using OpenCage geocoding API.
/// Skeleton: requires network I/O.
pub struct OpenCageGeocoderService {
    _api_key: String,
}

impl OpenCageGeocoderService {
    /// Creates a new OpenCage geocoder service.
    pub fn new(api_key: String) -> Self {
        Self { _api_key: api_key }
    }
}
