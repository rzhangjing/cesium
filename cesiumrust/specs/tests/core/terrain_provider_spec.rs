//! Core/CesiumTerrainProviderSpec.js, EllipsoidTerrainProviderSpec.js,
//! HeightmapTerrainProviderSpec.js, VRTheWorldTerrainProviderSpec.js
//! → Rust integration tests

use cesium_provider::terrain_provider::{
    AvailabilityStrategy, CesiumTerrainProvider, EllipsoidTerrainProvider,
    HeightmapTerrainProvider, TerrainLayerConfig, VrTheWorldTerrainProvider,
};
use cesium_specs::assert_approx;
use cesium_specs::epsilon;

// === CesiumTerrainProvider ===

#[test]
fn test_cesium_terrain_provider_new() {
    let provider = CesiumTerrainProvider::new("https://assets.cesium.com/terrain");
    assert_eq!(provider.url, "https://assets.cesium.com/terrain");
    assert!(!provider.request_vertex_normals);
    assert!(!provider.request_water_mask);
}

#[test]
fn test_cesium_terrain_provider_with_vertex_normals() {
    let provider = CesiumTerrainProvider::new("https://example.com/terrain")
        .with_vertex_normals();
    assert!(provider.request_vertex_normals);
}

#[test]
fn test_cesium_terrain_provider_with_water_mask() {
    let provider = CesiumTerrainProvider::new("https://example.com/terrain")
        .with_water_mask();
    assert!(provider.request_water_mask);
}

#[test]
fn test_cesium_terrain_provider_tile_url() {
    let provider = CesiumTerrainProvider::new("https://example.com/terrain");
    let url = provider.get_tile_url(5, 10, 15);
    assert_eq!(url, "https://example.com/terrain/5/10/15.terrain");
}

#[test]
fn test_cesium_terrain_provider_tile_url_with_normals() {
    let provider = CesiumTerrainProvider::new("https://example.com/terrain")
        .with_vertex_normals();
    let url = provider.get_tile_url(3, 2, 1);
    assert!(url.contains("extensions=octvertexnormals"));
}

#[test]
fn test_cesium_terrain_provider_layer_json_url() {
    let provider = CesiumTerrainProvider::new("https://example.com/terrain");
    let url = provider.get_layer_json_url();
    assert_eq!(url, "https://example.com/terrain/layer.json");
}

#[test]
fn test_cesium_terrain_provider_availability_all() {
    let provider = CesiumTerrainProvider::new("https://example.com/terrain");
    assert!(provider.is_available(0));
    assert!(provider.is_available(10));
    assert!(provider.is_available(25));
}

#[test]
fn test_cesium_terrain_provider_availability_tiling_scheme() {
    let mut provider = CesiumTerrainProvider::new("https://example.com/terrain");
    provider.availability = AvailabilityStrategy::TilingScheme {
        minimum_level: 2,
        maximum_level: 18,
    };
    assert!(!provider.is_available(0));
    assert!(!provider.is_available(1));
    assert!(provider.is_available(2));
    assert!(provider.is_available(10));
    assert!(provider.is_available(18));
    assert!(!provider.is_available(19));
}

// === EllipsoidTerrainProvider ===

#[test]
fn test_ellipsoid_terrain_provider_new() {
    let provider = EllipsoidTerrainProvider::new();
    // Should always return height 0
    assert_approx!(provider.get_height(0.0, 0.0), 0.0, epsilon::EPSILON15);
    assert_approx!(provider.get_height(1.0, 0.5), 0.0, epsilon::EPSILON15);
}

// === HeightmapTerrainProvider ===

#[test]
fn test_heightmap_terrain_provider_new() {
    let provider = HeightmapTerrainProvider::new("https://example.com/heightmap");
    assert_eq!(provider.url, "https://example.com/heightmap");
    assert_eq!(provider.width, 65);
    assert_eq!(provider.height, 65);
    assert_eq!(provider.minimum_level, 0);
    assert_eq!(provider.maximum_level, 25);
}

#[test]
fn test_heightmap_terrain_provider_with_dimensions() {
    let provider = HeightmapTerrainProvider::new("https://example.com/heightmap")
        .with_dimensions(128, 128);
    assert_eq!(provider.width, 128);
    assert_eq!(provider.height, 128);
}

#[test]
fn test_heightmap_terrain_provider_tile_url() {
    let provider = HeightmapTerrainProvider::new("https://example.com/heightmap");
    let url = provider.get_tile_url(3, 4, 5);
    assert_eq!(url, "https://example.com/heightmap/3/4/5.terrain");
}

#[test]
fn test_heightmap_terrain_provider_availability() {
    let provider = HeightmapTerrainProvider::new("https://example.com/heightmap");
    assert!(provider.is_available(0));
    assert!(provider.is_available(12));
    assert!(provider.is_available(25));
    assert!(!provider.is_available(26));
}

// === VrTheWorldTerrainProvider ===

#[test]
fn test_vr_the_world_terrain_provider_new() {
    let provider = VrTheWorldTerrainProvider::new("https://example.com/vrtheworld");
    assert_eq!(provider.url, "https://example.com/vrtheworld");
}

#[test]
fn test_vr_the_world_terrain_provider_tile_url() {
    let provider = VrTheWorldTerrainProvider::new("https://example.com/vrtheworld");
    let url = provider.get_tile_url(2, 3, 4);
    assert_eq!(url, "https://example.com/vrtheworld/2/3/4.tif");
}

// === TerrainLayerConfig ===

#[test]
fn test_terrain_layer_config_from_json() {
    let json = r#"{
        "format": "quantized-mesh-1.0",
        "minLevel": 0,
        "maxLevel": 22,
        "projection": "EPSG:4326",
        "extensions": ["octvertexnormals", "watermask"]
    }"#;
    let config = TerrainLayerConfig::from_json(json).unwrap();
    assert_eq!(config.format, "quantized-mesh-1.0");
    assert_eq!(config.min_level, 0);
    assert_eq!(config.max_level, 22);
    assert_eq!(config.projection, "EPSG:4326");
    assert!(config.has_vertex_normals);
    assert!(config.has_water_mask);
}

#[test]
fn test_terrain_layer_config_minimal() {
    let json = r#"{ "format": "heightmap-1.0" }"#;
    let config = TerrainLayerConfig::from_json(json).unwrap();
    assert_eq!(config.format, "heightmap-1.0");
    assert!(!config.has_vertex_normals);
    assert!(!config.has_water_mask);
}
