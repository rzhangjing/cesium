//! Tests for remaining stub modules (batch A): terrain providers,
//! geocoder services, Google Earth Enterprise, and other small modules.

use cesium_core::arc_gis_tiled_elevation_terrain_provider::ArcGISTiledElevationTerrainProvider;
use cesium_core::bing_maps_geocoder_service::BingMapsGeocoderService;
use cesium_core::cartographic_geocoder_service::CartographicGeocoderService;
use cesium_core::cesium_terrain_provider::CesiumTerrainProvider;
use cesium_core::cesium3d_tiles_terrain_data::Cesium3DTilesTerrainData;
use cesium_core::cesium3d_tiles_terrain_geometry_processor::Cesium3DTilesTerrainGeometryProcessor;
use cesium_core::create_world_bathymetry_async::CreateWorldBathymetryAsync;
use cesium_core::create_world_terrain_async::CreateWorldTerrainAsync;
use cesium_core::custom_heightmap_terrain_provider::CustomHeightmapTerrainProvider;
use cesium_core::fullscreen::Fullscreen;
use cesium_core::geocoder_service::GeocoderService;
use cesium_core::google_earth_enterprise_metadata::GoogleEarthEnterpriseMetadata;
use cesium_core::google_earth_enterprise_terrain_data::GoogleEarthEnterpriseTerrainData;
use cesium_core::google_earth_enterprise_terrain_provider::GoogleEarthEnterpriseTerrainProvider;
use cesium_core::google_earth_enterprise_tile_information::GoogleEarthEnterpriseTileInformation;
use cesium_core::google_geocoder_service::GoogleGeocoderService;
use cesium_core::heightmap_terrain_data::HeightmapTerrainData;
use cesium_core::heightmap_tessellator::HeightmapTessellator;
use cesium_core::i_twin_platform::ITwinPlatform;
use cesium_core::ion_geocoder_service::IonGeocoderService;

// --- Terrain provider stubs ---
#[test]
fn arc_gis_tiled_elevation_terrain_provider_new() {
    let _ = ArcGISTiledElevationTerrainProvider::new();
    let _ = ArcGISTiledElevationTerrainProvider::default();
}

#[test]
fn cesium_terrain_provider_new() {
    let _ = CesiumTerrainProvider::new();
    let _ = CesiumTerrainProvider::default();
}

#[test]
fn cesium3d_tiles_terrain_data_new() {
    let _ = Cesium3DTilesTerrainData::new();
    let _ = Cesium3DTilesTerrainData::default();
}

#[test]
fn cesium3d_tiles_terrain_geometry_processor_new() {
    let _ = Cesium3DTilesTerrainGeometryProcessor::new();
    let _ = Cesium3DTilesTerrainGeometryProcessor::default();
}

#[test]
fn custom_heightmap_terrain_provider_new() {
    let _ = CustomHeightmapTerrainProvider::new();
    let _ = CustomHeightmapTerrainProvider::default();
}

#[test]
fn google_earth_enterprise_metadata_new() {
    let _ = GoogleEarthEnterpriseMetadata::new();
    let _ = GoogleEarthEnterpriseMetadata::default();
}

#[test]
fn google_earth_enterprise_terrain_data_new() {
    let _ = GoogleEarthEnterpriseTerrainData::new();
    let _ = GoogleEarthEnterpriseTerrainData::default();
}

#[test]
fn google_earth_enterprise_terrain_provider_new() {
    let _ = GoogleEarthEnterpriseTerrainProvider::new();
    let _ = GoogleEarthEnterpriseTerrainProvider::default();
}

#[test]
fn google_earth_enterprise_tile_information_new() {
    let _ = GoogleEarthEnterpriseTileInformation::new();
    let _ = GoogleEarthEnterpriseTileInformation::default();
}

#[test]
fn heightmap_terrain_data_new() {
    let _ = HeightmapTerrainData::new();
    let _ = HeightmapTerrainData::default();
}

#[test]
fn heightmap_tessellator_new() {
    let _ = HeightmapTessellator::new();
    let _ = HeightmapTessellator::default();
}

// --- Geocoder service stubs ---
#[test]
fn bing_maps_geocoder_service_new() {
    let _ = BingMapsGeocoderService::new();
    let _ = BingMapsGeocoderService::default();
}

#[test]
fn cartographic_geocoder_service_new() {
    let _ = CartographicGeocoderService::new();
    let _ = CartographicGeocoderService::default();
}

#[test]
fn geocoder_service_new() {
    let _ = GeocoderService::new();
    let _ = GeocoderService::default();
}

#[test]
fn google_geocoder_service_new() {
    let _ = GoogleGeocoderService::new();
    let _ = GoogleGeocoderService::default();
}

#[test]
fn ion_geocoder_service_new() {
    let _ = IonGeocoderService::new();
    let _ = IonGeocoderService::default();
}

// --- Other stubs ---
#[test]
fn create_world_bathymetry_async_new() {
    let _ = CreateWorldBathymetryAsync::new();
    let _ = CreateWorldBathymetryAsync::default();
}

#[test]
fn create_world_terrain_async_new() {
    let _ = CreateWorldTerrainAsync::new();
    let _ = CreateWorldTerrainAsync::default();
}

#[test]
fn fullscreen_new() {
    let _ = Fullscreen::new();
    let _ = Fullscreen::default();
}

#[test]
fn i_twin_platform_new() {
    let _ = ITwinPlatform::new();
    let _ = ITwinPlatform::default();
}
