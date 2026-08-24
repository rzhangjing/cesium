//! Mirror of `packages/engine/Specs/Core/GoogleEarthEnterpriseTerrainProviderSpec.js`
//! (389 lines).
//!
//! The JS `installMockGetQuadTreePacket` prototype spy is reproduced by
//! inserting the same four `GoogleEarthEnterpriseTileInformation` children
//! (bits `0xff`, `ancestorHasTerrain = true`) directly into the metadata
//! tile info (the JS `fromUrl` call in the spec is fully intercepted, so no
//! HTTP happens there; the Rust mirror constructs the metadata object
//! directly with the default decode key, matching the JS dbRoot-fallback
//! outcome).
//!
//! # Skipped JS tests (DEVIATION)
//! - "returns undefined if too many requests are already in progress":
//!   RequestScheduler throttling is not modeled (module DEVIATION 1).

use cesium_core::credit::Credit;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::google_earth_enterprise_metadata::{
    default_key, GoogleEarthEnterpriseMetadata,
};
use cesium_core::google_earth_enterprise_terrain_provider::{
    GoogleEarthEnterpriseTerrainProvider, GoogleEarthEnterpriseTerrainProviderOptions,
    GoogleEarthEnterpriseTerrainTileData, TerrainState,
};
use cesium_core::google_earth_enterprise_tile_information::GoogleEarthEnterpriseTileInformation;
use cesium_core::math::CesiumMath;
use cesium_core::resource::{DerivedResourceOptions, MockResourceBackend, Resource};
use cesium_core::terrain_provider::TerrainProvider;
use cesium_core::tiling_scheme::TilingScheme;

const BASE_URL: &str = "made/up/url";

const GEE_TERRAIN_FIXTURE: &[u8] =
    include_bytes!("../../Data/GoogleEarthEnterprise/gee.terrain");

/// Mirrors `installMockGetQuadTreePacket` + the intercepted
/// `GoogleEarthEnterpriseMetadata.fromUrl("made/up/url")`.
fn mock_metadata() -> GoogleEarthEnterpriseMetadata {
    let mut resource = Resource::new(BASE_URL.to_string());
    resource.append_forward_slash();
    let mut metadata = GoogleEarthEnterpriseMetadata::new(resource);
    // JS: requestDbRoot fails in the spec environment -> default key.
    metadata.key = Some(default_key());
    install_mock_get_quad_tree_packet(&metadata, "");
    metadata
}

fn install_mock_get_quad_tree_packet(metadata: &GoogleEarthEnterpriseMetadata, quad_key: &str) {
    let mut tile_info = metadata.tile_info.borrow_mut();
    for i in 0..4 {
        let mut t = GoogleEarthEnterpriseTileInformation::new(0xff, 1, 1, 1, 0, 0);
        t.ancestor_has_terrain = true;
        tile_info.insert(format!("{quad_key}{i}"), Some(t));
    }
}

/// The terrain tile URL built by the provider for a quad key
/// (`flatfile?f1c-0{quadKey}-t.{version}`).
fn terrain_url(quad_key: &str, version: i32) -> String {
    let mut resource = Resource::new(BASE_URL.to_string());
    resource.append_forward_slash();
    resource
        .clone_resource()
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(&format!("flatfile?f1c-0{quad_key}-t.{version}")),
            ..Default::default()
        })
        .url()
}

#[test]
fn conforms_to_terrain_provider_interface() {
    fn assert_terrain_provider(_: &dyn TerrainProvider) {}
    // Type-level assertion (JS `toConformToInterface`).
    let _f: fn(&GoogleEarthEnterpriseTerrainProvider) = |p| assert_terrain_provider(p);
}

#[test]
#[should_panic(expected = "metadata is required, actual value was undefined")]
fn from_metadata_throws_without_metadata() {
    let _ = GoogleEarthEnterpriseTerrainProvider::from_metadata(None, None);
}

#[tokio::test]
async fn from_metadata_throws_if_there_isnt_terrain() {
    let mut metadata = mock_metadata();
    metadata.terrain_present = false;

    let result = GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(metadata), None);
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("fromMetadata must fail without terrain"),
    };
    assert_eq!(error.message, "The server made/up/url/ doesn't have terrain");
}

#[test]
fn uses_geographic_tiling_scheme_by_default() {
    let provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();

    // Type-level: the tiling scheme is a GeographicTilingScheme.
    let _scheme: &cesium_core::geographic_tiling_scheme::GeographicTilingScheme =
        provider.tiling_scheme();
}

#[test]
fn can_use_a_custom_ellipsoid() {
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let provider = GoogleEarthEnterpriseTerrainProvider::from_metadata(
        Some(mock_metadata()),
        Some(GoogleEarthEnterpriseTerrainProviderOptions {
            ellipsoid: Some(ellipsoid),
            ..Default::default()
        }),
    )
    .unwrap();

    assert_eq!(*provider.tiling_scheme().ellipsoid(), ellipsoid);
}

#[test]
fn has_error_event() {
    let provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();
    let event = provider.error_event();
    assert!(std::ptr::eq(event, provider.error_event()));
}

#[test]
fn returns_reasonable_geometric_error_for_various_levels() {
    let provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();

    assert!(provider.get_level_maximum_geometric_error(0) > 0.0);
    assert!(CesiumMath::equals_epsilon(
        provider.get_level_maximum_geometric_error(0),
        provider.get_level_maximum_geometric_error(1) * 2.0,
        Some(CesiumMath::EPSILON10),
        None
    ));
    assert!(CesiumMath::equals_epsilon(
        provider.get_level_maximum_geometric_error(1),
        provider.get_level_maximum_geometric_error(2) * 2.0,
        Some(CesiumMath::EPSILON10),
        None
    ));
}

#[test]
fn credit_is_none_if_credit_is_not_provided() {
    let provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();

    assert!(provider.credit().is_none());
}

#[test]
fn credit_is_defined_if_credit_is_provided() {
    // DEVIATION: JS accepts a string and wraps it in a Credit; the Rust
    // options take the Credit directly.
    let provider = GoogleEarthEnterpriseTerrainProvider::from_metadata(
        Some(mock_metadata()),
        Some(GoogleEarthEnterpriseTerrainProviderOptions {
            credit: Some(Credit::new(
                "thanks to our awesome made up contributors!",
                false,
            )),
            ..Default::default()
        }),
    )
    .unwrap();

    assert!(provider.credit().is_some());
}

#[test]
fn has_water_mask_is_false() {
    let provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();

    assert!(!provider.has_water_mask());
}

#[test]
fn has_vertex_normals_is_false() {
    let provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();

    assert!(!provider.has_vertex_normals());
}

// ── requestTileGeometry ─────────────────────────────────────────────────

/// Mirrors `waitForTile(0, 0, 0, f)`: the tile must report available and
/// `requestTileGeometry` must yield the expected data.
async fn wait_for_tile(backend: &MockResourceBackend) -> GoogleEarthEnterpriseTerrainTileData {
    let mut provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();

    // JS polls getTileDataAvailable until truthy; with the mock metadata it
    // is immediately available.
    assert_eq!(provider.get_tile_data_available(0, 0, 0), Some(true));

    provider
        .request_tile_geometry(0, 0, 0, backend)
        .await
        .expect("requestTileGeometry must not fail")
        .expect("requestTileGeometry must return a tile")
}

#[tokio::test]
async fn request_tile_geometry_provides_google_earth_enterprise_terrain_data() {
    let mut backend = MockResourceBackend::new();
    backend.register_response(&terrain_url("3", 1), GEE_TERRAIN_FIXTURE.to_vec());

    let loaded = wait_for_tile(&backend).await;
    assert!(matches!(
        loaded,
        GoogleEarthEnterpriseTerrainTileData::Google(_)
    ));
}

#[tokio::test]
async fn request_tile_geometry_provides_google_earth_enterprise_terrain_data_with_from_metadata()
 {
    let mut backend = MockResourceBackend::new();
    backend.register_response(&terrain_url("3", 1), GEE_TERRAIN_FIXTURE.to_vec());

    let mut provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(mock_metadata()), None).unwrap();

    assert_eq!(provider.get_tile_data_available(0, 0, 0), Some(true));

    let loaded = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .expect("requestTileGeometry must not fail")
        .expect("requestTileGeometry must return a tile");

    assert!(matches!(
        loaded,
        GoogleEarthEnterpriseTerrainTileData::Google(_)
    ));
}

#[tokio::test]
async fn supports_get_tile_data_available() {
    let backend = MockResourceBackend::new();

    let metadata = mock_metadata();
    // Remove the terrain bit from the 0,1,0 tile (quadKey "0").
    {
        let mut tile_info = metadata.tile_info.borrow_mut();
        let info = tile_info.get_mut("0").and_then(|i| i.as_mut()).unwrap();
        info.set_bits(0x7f);
        info.terrain_state = Some(TerrainState::NONE);
        info.ancestor_has_terrain = true;
    }

    let provider =
        GoogleEarthEnterpriseTerrainProvider::from_metadata(Some(metadata), None).unwrap();

    assert_eq!(provider.get_tile_data_available(0, 0, 0), Some(true));
    assert_eq!(provider.get_tile_data_available(0, 1, 0), Some(false));
    assert_eq!(provider.get_tile_data_available(1, 0, 0), Some(true));
    assert_eq!(provider.get_tile_data_available(1, 1, 0), Some(true));
    assert_eq!(provider.get_tile_data_available(0, 0, 2), Some(false));

    // The trait-level accessor reports the same answers.
    let provider_ref: &dyn TerrainProvider = &provider;
    assert_eq!(provider_ref.get_tile_data_available(0, 0, 0), Some(true));
    let _ = &backend;
}
