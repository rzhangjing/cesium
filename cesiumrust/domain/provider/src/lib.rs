//! cesium-provider: Imagery and terrain providers for tile-based services.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/UrlTemplateImageryProvider.js` → imagery_provider
//! - `Scene/WebMapTileServiceImageryProvider.js` → imagery_provider
//! - `Scene/WebMapServiceImageryProvider.js` → imagery_provider
//! - `Scene/TileMapServiceImageryProvider.js` → imagery_provider
//! - `Core/CesiumTerrainProvider.js` → terrain_provider
//! - `Core/EllipsoidTerrainProvider.js` → terrain_provider

pub mod imagery_provider;
pub mod terrain_provider;

pub use imagery_provider::{
    BingMapStyle, BingMapsImageryProvider, OpenStreetMapImageryProvider,
    SubdomainStrategy, TileCoord, TmsImageryProvider, UrlTemplateImageryProvider,
    WmsImageryProvider, WmtsImageryProvider,
};
pub use terrain_provider::{
    AvailabilityStrategy, CesiumTerrainProvider, EllipsoidTerrainProvider,
    HeightmapTerrainProvider, TerrainLayerConfig, VrTheWorldTerrainProvider,
};
