//! Terrain provider extended spec tests.
//!
//! Maps to CesiumJS:
//! - Core/CesiumTerrainProviderSpec.js
//! - Core/HeightmapTerrainProviderSpec.js
//! - Core/VRTheWorldTerrainProviderSpec.js
//! - Core/ArcGISTerrainProviderSpec.js
//! - Core/GoogleEarthEnterpriseTerrainProviderSpec.js
//!
//! A-class tests: URL generation, layer.json parsing, availability, descriptors.

use cesium_provider::terrain_provider::{
    ArcGisTerrainProvider, AvailabilityStrategy, CesiumTerrainProvider, EllipsoidTerrainProvider,
    GoogleEarthEnterpriseTerrainProvider, HeightmapTerrainProvider, TerrainLayerConfig,
    TerrainProviderDescriptor, VrTheWorldTerrainProvider,
};

// === CesiumTerrainProvider ===

#[test]
fn cesium_provider_new_defaults() {
    let p = CesiumTerrainProvider::new("https://assets.cesium.com/terrain");
    assert_eq!(p.url, "https://assets.cesium.com/terrain");
    assert!(!p.request_vertex_normals);
    assert!(!p.request_water_mask);
    assert_eq!(p.availability, AvailabilityStrategy::All);
    assert!(p.credit.is_none());
}

#[test]
fn cesium_provider_tile_url_basic() {
    let p = CesiumTerrainProvider::new("https://terrain.example.com/tiles");
    assert_eq!(
        p.get_tile_url(5, 10, 15),
        "https://terrain.example.com/tiles/5/10/15.terrain"
    );
}

#[test]
fn cesium_provider_tile_url_trailing_slash() {
    let p = CesiumTerrainProvider::new("https://terrain.example.com/");
    assert_eq!(
        p.get_tile_url(0, 0, 0),
        "https://terrain.example.com/0/0/0.terrain"
    );
}

#[test]
fn cesium_provider_tile_url_with_normals() {
    let p = CesiumTerrainProvider::new("https://t.com").with_vertex_normals();
    let url = p.get_tile_url(3, 1, 2);
    assert!(url.contains("extensions=octvertexnormals"));
    assert!(url.starts_with("https://t.com/3/1/2.terrain?"));
}

#[test]
fn cesium_provider_tile_url_with_water_mask() {
    let p = CesiumTerrainProvider::new("https://t.com").with_water_mask();
    let url = p.get_tile_url(3, 1, 2);
    assert!(url.contains("extensions=watermask"));
}

#[test]
fn cesium_provider_tile_url_with_both_extensions() {
    let p = CesiumTerrainProvider::new("https://t.com")
        .with_vertex_normals()
        .with_water_mask();
    let url = p.get_tile_url(1, 0, 0);
    assert!(url.contains("octvertexnormals"));
    assert!(url.contains("watermask"));
}

#[test]
fn cesium_provider_layer_json_url() {
    let p = CesiumTerrainProvider::new("https://terrain.example.com/");
    assert_eq!(
        p.get_layer_json_url(),
        "https://terrain.example.com/layer.json"
    );
}

#[test]
fn cesium_provider_availability_all() {
    let p = CesiumTerrainProvider::new("https://t.com");
    assert!(p.is_available(0));
    assert!(p.is_available(22));
    assert!(p.is_available(100));
}

#[test]
fn cesium_provider_availability_tiling_scheme() {
    let mut p = CesiumTerrainProvider::new("https://t.com");
    p.availability = AvailabilityStrategy::TilingScheme {
        minimum_level: 2,
        maximum_level: 18,
    };
    assert!(!p.is_available(0));
    assert!(!p.is_available(1));
    assert!(p.is_available(2));
    assert!(p.is_available(10));
    assert!(p.is_available(18));
    assert!(!p.is_available(19));
}

// === EllipsoidTerrainProvider ===

#[test]
fn ellipsoid_provider_always_zero() {
    let p = EllipsoidTerrainProvider::new();
    assert_eq!(p.get_height(0.0, 0.0), 0.0);
    assert_eq!(p.get_height(1.5, -0.8), 0.0);
    assert_eq!(p.get_height(-180.0, 90.0), 0.0);
}

// === HeightmapTerrainProvider ===

#[test]
fn heightmap_provider_defaults() {
    let p = HeightmapTerrainProvider::new("https://hm.example.com");
    assert_eq!(p.width, 65);
    assert_eq!(p.height, 65);
    assert_eq!(p.file_extension, "terrain");
    assert_eq!(p.minimum_level, 0);
    assert_eq!(p.maximum_level, 25);
}

#[test]
fn heightmap_provider_custom_dimensions() {
    let p = HeightmapTerrainProvider::new("https://hm.com").with_dimensions(128, 128);
    assert_eq!(p.width, 128);
    assert_eq!(p.height, 128);
}

#[test]
fn heightmap_provider_tile_url() {
    let p = HeightmapTerrainProvider::new("https://hm.example.com");
    assert_eq!(
        p.get_tile_url(4, 8, 12),
        "https://hm.example.com/4/8/12.terrain"
    );
}

#[test]
fn heightmap_provider_availability() {
    let p = HeightmapTerrainProvider::new("https://hm.com");
    assert!(p.is_available(0));
    assert!(p.is_available(25));
    assert!(!p.is_available(26));
}

// === VrTheWorldTerrainProvider ===

#[test]
fn vrtheworld_provider_tile_url() {
    let p = VrTheWorldTerrainProvider::new("https://vrtheworld.example.com");
    assert_eq!(
        p.get_tile_url(2, 3, 4),
        "https://vrtheworld.example.com/2/3/4.tif"
    );
}

#[test]
fn vrtheworld_provider_trailing_slash() {
    let p = VrTheWorldTerrainProvider::new("https://vr.com/");
    assert_eq!(p.get_tile_url(0, 0, 0), "https://vr.com/0/0/0.tif");
}

// === ArcGISTerrainProvider ===

#[test]
fn arcgis_provider_defaults() {
    let p = ArcGisTerrainProvider::new("https://elevation.arcgis.com");
    assert_eq!(p.tile_width, 256);
    assert_eq!(p.tile_height, 256);
    assert_eq!(p.maximum_level, 23);
    assert!(p.use_https);
}

#[test]
fn arcgis_provider_tile_url() {
    let p = ArcGisTerrainProvider::new("https://elevation.arcgis.com");
    // ArcGIS uses /tile/{level}/{row}/{col} = /tile/{level}/{y}/{x}
    assert_eq!(
        p.get_tile_url(5, 10, 15),
        "https://elevation.arcgis.com/tile/5/15/10"
    );
}

#[test]
fn arcgis_provider_with_credit() {
    let p = ArcGisTerrainProvider::new("https://arcgis.com").with_credit("Esri");
    assert_eq!(p.credit, Some("Esri".to_string()));
}

// === GoogleEarthEnterpriseTerrainProvider ===

#[test]
fn gee_provider_new() {
    let p = GoogleEarthEnterpriseTerrainProvider::new("https://gee.example.com", "/dbRoot");
    assert_eq!(p.url, "https://gee.example.com");
    assert_eq!(p.path, "/dbRoot");
    assert_eq!(p.tile_width, 32);
    assert_eq!(p.tile_height, 32);
    assert_eq!(p.maximum_level, 23);
}

#[test]
fn gee_provider_tile_url() {
    let p = GoogleEarthEnterpriseTerrainProvider::new("https://gee.com/", "/db");
    let url = p.get_tile_url(3, 5, 7);
    assert!(url.contains("request=TerrainMaps"));
    assert!(url.contains("path=/db"));
    assert!(url.contains("x=5"));
    assert!(url.contains("y=7"));
    assert!(url.contains("z=3"));
}

#[test]
fn gee_provider_metadata_url() {
    let p = GoogleEarthEnterpriseTerrainProvider::new("https://gee.com", "/db");
    let url = p.get_metadata_url();
    assert!(url.contains("request=DatabaseMetadata"));
    assert!(url.contains("path=/db"));
}

#[test]
fn gee_provider_with_credit() {
    let p = GoogleEarthEnterpriseTerrainProvider::new("https://gee.com", "/db")
        .with_credit("Google");
    assert_eq!(p.credit, Some("Google".to_string()));
}

// === TerrainLayerConfig ===

#[test]
fn layer_config_parse_full() {
    let json = r#"{
        "tilejson": "2.1.0",
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
fn layer_config_parse_minimal() {
    let json = r#"{"format": "heightmap-1.0", "maxLevel": 15}"#;
    let config = TerrainLayerConfig::from_json(json).unwrap();
    assert_eq!(config.format, "heightmap-1.0");
    assert_eq!(config.max_level, 15);
    assert!(!config.has_vertex_normals);
    assert!(!config.has_water_mask);
}

#[test]
fn layer_config_no_extensions() {
    let json = r#"{"format": "quantized-mesh-1.0", "minLevel": 3, "maxLevel": 18, "projection": "EPSG:3857"}"#;
    let config = TerrainLayerConfig::from_json(json).unwrap();
    assert_eq!(config.min_level, 3);
    assert_eq!(config.projection, "EPSG:3857");
    assert!(!config.has_vertex_normals);
    assert!(!config.has_water_mask);
}

// === TerrainProviderDescriptor ===

#[test]
fn descriptor_cesium() {
    let provider = CesiumTerrainProvider::new("https://t.com")
        .with_vertex_normals()
        .with_water_mask();
    let desc = TerrainProviderDescriptor::cesium(provider, 18);

    assert!(desc.has_vertex_normals);
    assert!(desc.has_water_mask);
    assert_eq!(desc.maximum_level, 18);
    assert!(desc.is_available(10));
    assert!(desc.is_available(18));
    assert!(!desc.is_available(19));
}

#[test]
fn descriptor_ellipsoid() {
    let desc = TerrainProviderDescriptor::ellipsoid();
    assert!(!desc.has_vertex_normals);
    assert!(!desc.has_water_mask);
    assert_eq!(desc.maximum_level, 0);
    assert!(desc.get_tile_url(0, 0, 0).is_none());
}

#[test]
fn descriptor_heightmap() {
    let provider = HeightmapTerrainProvider::new("https://hm.com");
    let desc = TerrainProviderDescriptor::heightmap(provider);
    assert_eq!(desc.maximum_level, 25);
    let url = desc.get_tile_url(3, 1, 2).unwrap();
    assert!(url.contains("hm.com"));
    assert!(url.contains("3/1/2"));
}

#[test]
fn descriptor_get_tile_url_cesium() {
    let provider = CesiumTerrainProvider::new("https://terrain.example.com");
    let desc = TerrainProviderDescriptor::cesium(provider, 22);
    let url = desc.get_tile_url(5, 10, 15).unwrap();
    assert_eq!(url, "https://terrain.example.com/5/10/15.terrain");
}
