//! Ported from `packages/engine/Source/Core/IonGeocoderService.js`.

/// A geocoder service using Cesium ion.
pub struct IonGeocoderService {
    _private: (),
}

impl IonGeocoderService {
    /// Creates a new IonGeocoderService.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for IonGeocoderService {
    fn default() -> Self { Self::new() }
}
