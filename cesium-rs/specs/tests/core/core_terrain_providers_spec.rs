//! Tests for EllipsoidTerrainProvider, Cesium3DTilesTerrainProvider.

use cesium_core::cesium3d_tiles_terrain_provider::{
    Cesium3DTilesTerrainProvider, Cesium3DTilesTerrainProviderOptions,
};
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoid_terrain_provider::EllipsoidTerrainProvider;

// --- EllipsoidTerrainProvider ---
#[test]
fn ellipsoid_terrain_provider_default() {
    let provider = EllipsoidTerrainProvider::new(None, None);
    let _ = provider;
}

#[test]
fn ellipsoid_terrain_provider_with_ellipsoid() {
    let provider = EllipsoidTerrainProvider::new(None, Some(Ellipsoid::WGS84.clone()));
    let _ = provider;
}

#[test]
fn ellipsoid_terrain_provider_geometric_error_decreases_with_level() {
    let provider = EllipsoidTerrainProvider::new(None, None);
    let err0 = provider.get_level_maximum_geometric_error(0);
    let err1 = provider.get_level_maximum_geometric_error(1);
    let err2 = provider.get_level_maximum_geometric_error(2);
    assert!(err0 > 0.0);
    assert!(err1 < err0);
    assert!(err2 < err1);
}

#[test]
fn ellipsoid_terrain_provider_geometric_error_halves() {
    let provider = EllipsoidTerrainProvider::new(None, None);
    let err0 = provider.get_level_maximum_geometric_error(0);
    let err1 = provider.get_level_maximum_geometric_error(1);
    // Each level halves the error
    assert!((err1 - err0 / 2.0).abs() < err0 * 0.001);
}

// --- Cesium3DTilesTerrainProvider ---
#[test]
fn cesium3d_tiles_provider_new() {
    let provider = Cesium3DTilesTerrainProvider::new();
    assert!(!provider.ready);
    assert!(!provider.is_destroyed());
    assert!(provider.url.is_none());
    assert!(!provider.request_vertex_normals);
    assert!(!provider.request_water_mask);
}

#[test]
fn cesium3d_tiles_provider_default() {
    let provider = Cesium3DTilesTerrainProvider::default();
    assert!(!provider.ready);
    assert_eq!(provider.ellipsoid, Ellipsoid::WGS84);
}

#[test]
fn cesium3d_tiles_provider_from_url() {
    let provider = Cesium3DTilesTerrainProvider::from_url(
        "https://example.com/tileset.json",
        None,
    );
    assert_eq!(provider.url.as_deref(), Some("https://example.com/tileset.json"));
    assert!(!provider.ready);
}

#[test]
fn cesium3d_tiles_provider_from_ion_asset_id() {
    let provider = Cesium3DTilesTerrainProvider::from_ion_asset_id(12345, None);
    assert!(!provider.ready);
}

#[test]
fn cesium3d_tiles_provider_destroy() {
    let mut provider = Cesium3DTilesTerrainProvider::new();
    assert!(!provider.is_destroyed());
    provider.destroy();
    assert!(provider.is_destroyed());
}

#[test]
fn cesium3d_tiles_provider_get_height_returns_zero() {
    let provider = Cesium3DTilesTerrainProvider::new();
    let cart = Cartographic::from_radians_new(0.0, 0.0, None);
    assert_eq!(provider.get_height(&cart), 0.0);
}

#[test]
fn cesium3d_tiles_provider_options() {
    let opts = Cesium3DTilesTerrainProviderOptions {
        request_vertex_normals: true,
        request_water_mask: true,
        ellipsoid: Ellipsoid::WGS84.clone(),
        credit: Some("test".to_string()),
    };
    let provider = Cesium3DTilesTerrainProvider::from_url("https://example.com", Some(opts));
    assert!(!provider.ready);
}
