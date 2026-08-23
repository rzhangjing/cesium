//! Ported from `packages/engine/Source/Core/Cesium3DTilesTerrainProvider.js`.
//!
//! A terrain provider that accesses terrain data in 3D Tiles format.

use crate::ellipsoid::Ellipsoid;
use crate::event::Event;

/// A [`TerrainProvider`] that accesses terrain data in a 3D Tiles format.
///
/// Use `from_ion_asset_id` or `from_url` to construct. Do not call the constructor directly.
/// Mirrors CesiumJS `Cesium3DTilesTerrainProvider` (1053 lines).
pub struct Cesium3DTilesTerrainProvider {
    /// The ellipsoid used by this provider.
    pub ellipsoid: Ellipsoid,
    /// Whether to request vertex normals.
    pub request_vertex_normals: bool,
    /// Whether to request water masks.
    pub request_water_mask: bool,
    /// The credit for this provider.
    pub credit: Option<String>,
    /// Whether this provider is ready.
    pub ready: bool,
    /// Whether this provider has been destroyed.
    is_destroyed: bool,
    /// The error event.
    pub error_event: Event,
    /// The URL of the tileset.
    pub url: Option<String>,
}

impl Cesium3DTilesTerrainProvider {
    /// Creates a new Cesium3DTilesTerrainProvider.
    pub fn new() -> Self {
        Self {
            ellipsoid: Ellipsoid::WGS84,
            request_vertex_normals: false,
            request_water_mask: false,
            credit: None,
            ready: false,
            is_destroyed: false,
            error_event: Event::new(),
            url: None,
        }
    }

    /// Creates a provider from a Cesium ion asset ID.
    pub fn from_ion_asset_id(_asset_id: u64, _options: Option<Cesium3DTilesTerrainProviderOptions>) -> Self {
        // DEVIATION: Requires IonResource and async loading
        let mut provider = Self::new();
        provider.ready = false; // Would be set to true after async load
        provider
    }

    /// Creates a provider from a URL.
    pub fn from_url(_url: &str, _options: Option<Cesium3DTilesTerrainProviderOptions>) -> Self {
        // DEVIATION: Requires Resource and async loading
        let mut provider = Self::new();
        provider.url = Some(_url.to_string());
        provider.ready = false; // Would be set to true after async load
        provider
    }

    /// Returns whether this provider has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this provider.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }

    /// Returns the height of the terrain at the given cartographic position.
    pub fn get_height(&self, _cartographic: &crate::cartographic::Cartographic) -> f64 {
        // DEVIATION: Requires terrain tile lookup
        0.0
    }
}

/// Options for constructing a Cesium3DTilesTerrainProvider.
pub struct Cesium3DTilesTerrainProviderOptions {
    /// Whether to request vertex normals.
    pub request_vertex_normals: bool,
    /// Whether to request water masks.
    pub request_water_mask: bool,
    /// The ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// The credit.
    pub credit: Option<String>,
}

impl Default for Cesium3DTilesTerrainProvider {
    fn default() -> Self { Self::new() }
}
