//! cesium-provider: Imagery and terrain providers for tile-based services.
//!
//! STATUS (P2 code-health audit, 2026-09-27): implemented and covered by the
//! `cesium-specs` suite, but **not wired into any production runtime path** — no
//! adapter or application crate depends on it. Retained as a CesiumJS
//! feature-parity domain model reserved for future adapter bridging; do NOT read
//! it as a shipped capability. See docs/ARCHITECTURE.md "Test-only domain crates".
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
//! - `Core/GeographicTilingScheme.js` → tiling_scheme
//! - `Core/WebMercatorTilingScheme.js` → tiling_scheme
//! - `Core/TileAvailability.js` → tiling_scheme

pub mod imagery_provider;
pub mod terrain_provider;
pub mod tiling_scheme;
pub mod google_earth_enterprise;

pub use imagery_provider::{
    ArcGisMapServerImageryProvider, BingMapStyle, BingMapsImageryProvider,
    GoogleEarthEnterpriseImageryProvider, GoogleEarthEnterpriseMapsProvider,
    ImageryProviderDescriptor, ImageryProviderKind, IonImageryProvider,
    MapboxImageryProvider, MapboxStyleImageryProvider, OpenStreetMapImageryProvider,
    SingleTileImageryProvider, SubdomainStrategy, TileCoordinatesImageryProvider,
    TileCoord, TimeDynamicImagery, TmsImageryProvider, UrlTemplateImageryProvider,
    WmsGetFeatureInfo, WmsImageryProvider, WmtsImageryProvider,
};
pub use terrain_provider::{
    ArcGisTerrainProvider, AvailabilityStrategy, CesiumTerrainProvider, EllipsoidTerrainProvider,
    GoogleEarthEnterpriseTerrainProvider, HeightmapSampleParams, HeightmapTerrainProvider,
    QuantizedSampleParams, SampledHeight, TerrainLayerConfig, TerrainProviderDescriptor,
    TerrainProviderKind, VrTheWorldTerrainProvider, sample_height_bilinear, sample_height_quantized,
};
pub use tiling_scheme::{
    GeographicTilingScheme, TileAvailability, TilingScheme, WebMercatorTilingScheme,
};
