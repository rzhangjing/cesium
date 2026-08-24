//! Mirror of `packages/engine/Specs/Core/VRTheWorldTerrainProviderSpec.js`
//! (252 lines).
//!
//! The TileMap XML fixture is reproduced verbatim and registered on
//! [`MockResourceBackend`] under `made/up/url` (the JS spec patches
//! `loadWithXhr` to return it for every non-image URL). The tile image
//! (`Data/Images/Red16x16.png` in JS) is replaced with synthetic 32×32×4
//! RGBA bytes since the Rust port fetches the tile body as bytes
//! (module DEVIATION 3).
//!
//! # Skipped JS tests (DEVIATION)
//! - "returns undefined if too many requests are already in progress":
//!   RequestScheduler throttling is not modeled (module DEVIATION 3).

use std::collections::HashMap;

use cesium_core::math::CesiumMath;
use cesium_core::resource::{DerivedResourceOptions, MockResourceBackend, Resource};
use cesium_core::terrain_provider::TerrainProvider;
use cesium_core::vr_the_world_terrain_provider::{
    VRTheWorldTerrainProvider, VRTheWorldTerrainProviderOptions,
};

const BASE_URL: &str = "made/up/url";

fn xml_fixture(srs: &str) -> String {
    format!(
        r#"<TileMap version="1.0.0" tilemapservice="http://www.vr-theworld.com/vr-theworld/tiles/1.0.0/"><!--  Additional data: tms_type is default  --><Title>Hawaii World elev</Title><Abstract>layer to make cesium work right</Abstract><SRS>{srs}</SRS><BoundingBox minx="-180.000000" miny="-90.000000" maxx="180.000000" maxy="90.000000"/><Origin x="-180.000000" y="-90.000000"/><TileFormat width="32" height="32" mime-type="image/tif" extension="tif"/><TileSets><TileSet href="http://www.vr-theworld.com/vr-theworld/tiles/1.0.0/73/0" units-per-pixel="5.62500000000000000000" order="0"/><TileSet href="http://www.vr-theworld.com/vr-theworld/tiles/1.0.0/73/1" units-per-pixel="2.81250000000000000000" order="1"/><TileSet href="http://www.vr-theworld.com/vr-theworld/tiles/1.0.0/73/2" units-per-pixel="1.40625000000000000000" order="2"/><TileSet href="http://www.vr-theworld.com/vr-theworld/tiles/1.0.0/73/3" units-per-pixel="0.70312500000000000000" order="3"/></TileSets><DataExtents><DataExtent minx="-180.000000" miny="-90.000000" maxx="180.000000" maxy="90.000000" minlevel="0" maxlevel="9"/><DataExtent minx="24.999584" miny="-0.000417" maxx="30.000417" maxy="5.000417" minlevel="0" maxlevel="13"/></DataExtents></TileMap>"#
    )
}

fn register_metadata(backend: &mut MockResourceBackend) {
    backend.register_response(BASE_URL, xml_fixture("EPSG:4326").into_bytes());
}

/// Computes the tile URL exactly like the provider implementation
/// (`getDerivedResource({url: "level/x/y.tif", queryParameters: {cesium: true}})`).
fn expected_tile_url(x: i32, y: i32, level: i32) -> String {
    let y_tiles = 1 << level;
    let mut query = HashMap::new();
    query.insert("cesium".to_string(), "true".to_string());
    Resource::new(BASE_URL.to_string())
        .clone_resource()
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(&format!("{level}/{x}/{}.tif", y_tiles - y - 1)),
            query_parameters: Some(&query),
            ..Default::default()
        })
        .url()
}

/// Just return any old image: 32×32 RGBA pixels (TileFormat is 32×32).
fn make_tile_bytes() -> Vec<u8> {
    vec![0u8; 32 * 32 * 4]
}

async fn make_provider(backend: &MockResourceBackend) -> VRTheWorldTerrainProvider {
    VRTheWorldTerrainProvider::from_url(Some(BASE_URL), None, backend)
        .await
        .unwrap()
}

// ── Interface / construction ────────────────────────────────────────────

#[test]
fn conforms_to_terrain_provider_interface() {
    fn assert_terrain_provider(_: &dyn TerrainProvider) {}
    // Type-level assertion: construction below proves the trait is
    // implemented (JS `toConformToInterface`).
    let _f: fn(&VRTheWorldTerrainProvider) = |p| assert_terrain_provider(p);
}

#[tokio::test]
#[should_panic(expected = "url is required, actual value was undefined")]
async fn from_url_rejects_without_url() {
    let backend = MockResourceBackend::new();
    let _ = VRTheWorldTerrainProvider::from_url(None, None, &backend).await;
}

#[tokio::test]
async fn from_url_resolves_to_new_provider() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let provider = make_provider(&backend).await;
    let _ = &provider as &VRTheWorldTerrainProvider;
}

#[tokio::test]
async fn from_url_with_resource_resolves_to_new_provider() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let resource = Resource::new(BASE_URL.to_string());
    let provider = VRTheWorldTerrainProvider::from_resource(resource, None, &backend)
        .await
        .unwrap();
    let _ = &provider as &VRTheWorldTerrainProvider;
}

#[tokio::test]
async fn has_error_event() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let provider = make_provider(&backend).await;
    // JS: expect(provider.errorEvent).toBeDefined()
    assert!(std::ptr::eq(provider.error_event(), provider.error_event()));
}

#[tokio::test]
async fn returns_reasonable_geometric_error_for_various_levels() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let provider = make_provider(&backend).await;

    assert!(provider.get_level_maximum_geometric_error(0) > 0.0);
    assert!(CesiumMath::equals_epsilon(
        provider.get_level_maximum_geometric_error(0),
        provider.get_level_maximum_geometric_error(1) * 2.0,
        Some(CesiumMath::EPSILON10),
        None,
    ));
    assert!(CesiumMath::equals_epsilon(
        provider.get_level_maximum_geometric_error(1),
        provider.get_level_maximum_geometric_error(2) * 2.0,
        Some(CesiumMath::EPSILON10),
        None,
    ));
}

#[tokio::test]
async fn credit_is_undefined_if_credit_option_is_not_provided() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let provider = make_provider(&backend).await;
    assert!(provider.credit().is_none());
}

#[tokio::test]
async fn credit_is_defined_if_credit_option_is_provided() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let provider = VRTheWorldTerrainProvider::from_url(
        Some(BASE_URL),
        Some(VRTheWorldTerrainProviderOptions {
            credit: Some("thanks to our awesome made up contributors!".to_string()),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    assert!(provider.credit().is_some());
}

#[tokio::test]
async fn does_not_have_a_water_mask() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let provider = make_provider(&backend).await;
    assert!(!provider.has_water_mask());
}

#[tokio::test]
async fn from_url_throws_if_the_srs_is_not_supported() {
    let mut backend = MockResourceBackend::new();
    backend.register_response(BASE_URL, xml_fixture("EPSG:foo").into_bytes());

    let error = VRTheWorldTerrainProvider::from_url(Some(BASE_URL), None, &backend)
        .await
        .err()
        .expect("expected error");
    assert_eq!(
        error.message,
        "An error occurred while accessing made/up/url: SRS EPSG:foo is not supported"
    );
}

// ── requestTileGeometry ─────────────────────────────────────────────────

#[tokio::test]
async fn request_tile_geometry_provides_heightmap_terrain_data() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend);

    let terrain_provider = make_provider(&backend).await;

    // JS: expect(terrainProvider.tilingScheme).toBeInstanceOf(
    // GeographicTilingScheme) → behavioral assertion: geographic scheme
    // has 2×1 tiles at level 0 (WebMercator would be 1×1).
    assert_eq!(
        terrain_provider.tiling_scheme().get_number_of_x_tiles_at_level(0),
        2
    );
    assert_eq!(
        terrain_provider.tiling_scheme().get_number_of_y_tiles_at_level(0),
        1
    );

    let url = expected_tile_url(0, 0, 0);
    // JS: expect(request.url.indexOf(".tif?cesium=true")).toBeGreaterThanOrEqual(0)
    assert!(
        url.contains(".tif?cesium=true"),
        "expected .tif?cesium=true in {url}"
    );
    backend.register_response(&url, make_tile_bytes());

    let loaded_data = terrain_provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap();
    assert!(loaded_data.is_some());
}
