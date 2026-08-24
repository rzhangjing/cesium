//! Tests for remaining stub modules (batch A): terrain providers,
//! geocoder services, Google Earth Enterprise, and other small modules.

use cesium_core::bing_maps_geocoder_service::{
    BingMapsGeocoderService, BingMapsGeocoderServiceOptions,
};
use cesium_core::cartographic_geocoder_service::CartographicGeocoderService;
use cesium_core::cesium_terrain_provider::CesiumTerrainProvider;
use cesium_core::cesium3d_tiles_terrain_data::Cesium3DTilesTerrainData;
use cesium_core::cesium3d_tiles_terrain_geometry_processor::Cesium3DTilesTerrainGeometryProcessor;
use cesium_core::create_world_bathymetry_async::create_world_bathymetry_async;
use cesium_core::create_world_terrain_async::{create_world_terrain_async, CreateWorldTerrainOptions};
use cesium_core::fullscreen::Fullscreen;
use cesium_core::google_geocoder_service::{
    GoogleGeocoderService, GoogleGeocoderServiceOptions,
};
use cesium_core::heightmap_terrain_data::{
    HeightmapBuffer, HeightmapTerrainData, HeightmapTerrainDataOptions,
};
use cesium_core::heightmap_tessellator::HeightmapTessellator;
use cesium_core::i_twin_platform::ITwinPlatform;
use cesium_core::ion_geocoder_service::IonGeocoderService;

// --- Terrain provider stubs ---
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

// GoogleEarthEnterpriseMetadata / TerrainData / TerrainProvider were
// substantiated (P3): the old parameter-less stub tests were removed;
// behavior is covered by the GEE metadata/data/provider spec mirrors.

#[test]
fn heightmap_terrain_data_new() {
    // DEVIATION: `HeightmapTerrainData` is now substantiated and requires an
    // options object (mirrors the JS `options` parameter); the old
    // parameter-less stub constructor no longer exists.
    let _ = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![0.0; 4])),
        width: Some(2),
        height: Some(2),
        ..Default::default()
    });
}

#[test]
fn heightmap_tessellator_new() {
    let _ = HeightmapTessellator::new();
    let _ = HeightmapTessellator::default();
}

// --- Geocoder services (substantiated by Track A7; construction smoke) ---
#[test]
fn bing_maps_geocoder_service_new() {
    let _ = BingMapsGeocoderService::new(Some(BingMapsGeocoderServiceOptions {
        key: Some("key".to_string()),
        culture: None,
    }));
}

#[test]
fn cartographic_geocoder_service_new() {
    let _ = CartographicGeocoderService::new();
    let _ = CartographicGeocoderService::default();
}

// DEVIATION: `GeocoderService` is now a trait (interface); see
// `core_fidelity/geocoder_fidelity_spec.rs` for the behavior mirrors.

#[test]
fn google_geocoder_service_new() {
    let _ = GoogleGeocoderService::new(Some(GoogleGeocoderServiceOptions {
        key: Some("key".to_string()),
    }));
}

#[test]
fn ion_geocoder_service_new() {
    let _ = IonGeocoderService::new(None);
}

// --- Other stubs ---
#[test]
fn create_world_bathymetry_async_call() {
    // DEVIATION: stub returns None (Ion/Scene dependency deferred).
    assert!(create_world_bathymetry_async().is_none());
}

#[test]
fn create_world_terrain_async_call() {
    // DEVIATION: stub returns None (Ion/ResourceBackend deferred).
    assert!(create_world_terrain_async(None).is_none());
    assert!(create_world_terrain_async(Some(CreateWorldTerrainOptions::default())).is_none());
}

#[test]
fn fullscreen_probe() {
    // Track A6: `Fullscreen` is a static utility (no DOM in the headless
    // port, so the probe reports "not supported").
    let _ = Fullscreen::supports_fullscreen();
}

#[test]
fn i_twin_platform_new() {
    let _ = ITwinPlatform::new();
    let _ = ITwinPlatform::default();
}
