//! Mirror of `packages/engine/Specs/Core/ArcGISTiledElevationTerrainProviderSpec.js`
//! (406 lines).
//!
//! The JS spec patches `loadWithXhr` to answer three kinds of requests:
//! metadata (`f=pjson`), availability (`/tilemap/`) and tiles (`/tile/`).
//! The Rust port registers the same payloads on [`MockResourceBackend`] at
//! the exact derived URLs. The metadata fixture is reproduced verbatim
//! except the 17 `lods` entries, whose `resolution`/`scale` values are not
//! read by the provider (only `lods.length` matters) and are generated.
//! The LERC tile body (`Data/Images/Red16x16.png` in JS) is replaced with
//! synthetic bytes (module DEVIATION: tiles are fetched as byte buffers).
//!
//! # Skipped JS tests (DEVIATION)
//! - "fromUrl resolves with url promise" / "fromUrl rejects if url rejects":
//!   Rust callers resolve the url before the call (module DEVIATION 1).
//! - "returns undefined if too many requests are already in progress":
//!   RequestScheduler throttling is not modeled (module DEVIATION 2).

use std::collections::HashMap;

use serde_json::{json, Value};

use cesium_core::arc_gis_tiled_elevation_terrain_provider::{
    ArcGISTiledElevationTerrainProvider, ArcGISTiledElevationTerrainProviderOptions,
};
use cesium_core::math::CesiumMath;
use cesium_core::resource::{DerivedResourceOptions, MockResourceBackend, Resource};
use cesium_core::terrain_provider::TerrainProvider;

const BASE_URL: &str = "made/up/url";

// ── Fixture helpers ─────────────────────────────────────────────────────

fn base_metadata() -> Value {
    let lods: Vec<Value> = (0..=16)
        .map(|level| {
            json!({
                "level": level,
                "resolution": 156543.03392800014 / 2f64.powi(level),
                "scale": 5.91657527591555e8 / 2f64.powi(level),
            })
        })
        .collect();

    json!({
        "currentVersion": 10.3,
        "serviceDescription": "Test",
        "name": "Test",
        "description": "Test",
        "extent": {
            "xmin": -2.0037507842788246e7,
            "ymin": -2.0037508659999996e7,
            "xmax": 2.0037509157211754e7,
            "ymax": 2.0037508340000004e7,
            "spatialReference": {
                "wkid": 102100,
                "latestWkid": 3857,
            },
        },
        "bandCount": 1,
        "copyrightText":
            "Source: USGS, NGA, NASA, CGIAR, GEBCO,N Robinson,NCEAS,NLS,OS,NMA,Geodatastyrelsen and the GIS User Community",
        "minValues": [-450],
        "maxValues": [8700],
        "capabilities": "Image,Tilemap,Mensuration",
        "tileInfo": {
            "rows": 256,
            "cols": 256,
            "format": "LERC",
            "lods": lods,
        },
        "spatialReference": {
            "wkid": 3857,
            "latestWkid": 3857,
        },
    })
}

/// The metadata request URL exactly as the provider derives it
/// (`appendForwardSlash` + `f=pjson`).
fn metadata_url() -> String {
    let mut resource = Resource::new(BASE_URL.to_string());
    resource.append_forward_slash();
    let mut query = HashMap::new();
    query.insert("f".to_string(), "pjson".to_string());
    resource
        .get_derived_resource_with_options(DerivedResourceOptions {
            query_parameters: Some(&query),
            ..Default::default()
        })
        .url()
}

/// A derived sub-resource URL (tile / tilemap requests).
fn derived_url(path: &str) -> String {
    let mut resource = Resource::new(BASE_URL.to_string());
    resource.append_forward_slash();
    resource
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(path),
            ..Default::default()
        })
        .url()
}

fn register_metadata(backend: &mut MockResourceBackend, metadata: &Value) {
    backend.register_json_response(&metadata_url(), &metadata.to_string());
}

/// Availability fixture: 128×128 entries, all available.
fn availability_fixture() -> Value {
    json!({ "data": vec![1i64; 128 * 128] })
}

/// Just return any old tile body (JS loads `Data/Images/Red16x16.png`).
fn make_tile_bytes() -> Vec<u8> {
    vec![0u8; 16]
}

async fn make_provider(
    backend: &MockResourceBackend,
) -> ArcGISTiledElevationTerrainProvider {
    ArcGISTiledElevationTerrainProvider::from_url(Some(BASE_URL), None, backend)
        .await
        .unwrap()
}

// ── Interface / construction ────────────────────────────────────────────

#[test]
fn conforms_to_terrain_provider_interface() {
    fn assert_terrain_provider(_: &dyn TerrainProvider) {}
    let _f: fn(&ArcGISTiledElevationTerrainProvider) = |p| assert_terrain_provider(p);
}

#[tokio::test]
#[should_panic(expected = "url is required, actual value was undefined")]
async fn from_url_throws_without_url() {
    let backend = MockResourceBackend::new();
    let _ = ArcGISTiledElevationTerrainProvider::from_url(None, None, &backend).await;
}

#[tokio::test]
async fn from_url_resolves_to_new_provider() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let provider = make_provider(&backend).await;
    let _ = &provider as &ArcGISTiledElevationTerrainProvider;
}

#[tokio::test]
async fn from_url_with_resource_resolves_to_new_provider() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let resource = Resource::new(BASE_URL.to_string());
    let provider =
        ArcGISTiledElevationTerrainProvider::from_resource(resource, None, &backend)
            .await
            .unwrap();
    let _ = &provider as &ArcGISTiledElevationTerrainProvider;
}

#[tokio::test]
async fn has_error_event() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let provider = make_provider(&backend).await;
    assert!(std::ptr::eq(provider.error_event(), provider.error_event()));
}

#[tokio::test]
async fn returns_reasonable_geometric_error_for_various_levels() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

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
async fn logo_is_undefined_if_credit_is_not_provided() {
    let mut backend = MockResourceBackend::new();
    let mut metadata = base_metadata();
    // JS: `delete metadata.copyrightText;`
    metadata.as_object_mut().unwrap().remove("copyrightText");
    register_metadata(&mut backend, &metadata);

    let provider = make_provider(&backend).await;
    assert!(provider.credit().is_none());
}

#[tokio::test]
async fn logo_is_defined_if_credit_is_provided() {
    // JS passes a `credit` option but the JS implementation derives the
    // credit from the metadata `copyrightText` (see module DEVIATION 4);
    // the Rust port mirrors that behavior.
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let provider = ArcGISTiledElevationTerrainProvider::from_url(
        Some(BASE_URL),
        Some(ArcGISTiledElevationTerrainProviderOptions::default()),
        &backend,
    )
    .await
    .unwrap();
    assert!(provider.credit().is_some());
}

#[tokio::test]
async fn does_not_have_a_water_mask() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let provider = make_provider(&backend).await;
    assert!(!provider.has_water_mask());
}

#[tokio::test]
async fn detects_web_mercator_tiling_scheme() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let provider = make_provider(&backend).await;

    // JS: toBeInstanceOf(WebMercatorTilingScheme) → behavioral assertion:
    // WebMercator has 1×1 level-zero tiles (Geographic would be 2×1).
    assert_eq!(provider.tiling_scheme().get_number_of_x_tiles_at_level(0), 1);
    assert_eq!(provider.tiling_scheme().get_number_of_y_tiles_at_level(0), 1);
}

#[tokio::test]
async fn detects_geographic_tiling_scheme() {
    let mut backend = MockResourceBackend::new();
    let mut metadata = base_metadata();
    metadata["spatialReference"]["latestWkid"] = json!(4326);
    register_metadata(&mut backend, &metadata);

    let provider = make_provider(&backend).await;

    // JS: toBeInstanceOf(GeographicTilingScheme) → behavioral assertion:
    // Geographic has 2×1 level-zero tiles.
    assert_eq!(provider.tiling_scheme().get_number_of_x_tiles_at_level(0), 2);
    assert_eq!(provider.tiling_scheme().get_number_of_y_tiles_at_level(0), 1);
}

#[tokio::test]
async fn from_url_throws_if_the_srs_is_not_supported() {
    let mut backend = MockResourceBackend::new();
    let mut metadata = base_metadata();
    metadata["spatialReference"]["latestWkid"] = json!(1234);
    register_metadata(&mut backend, &metadata);

    let error = ArcGISTiledElevationTerrainProvider::from_url(Some(BASE_URL), None, &backend)
        .await
        .err()
        .expect("expected error");
    assert_eq!(error.message, "Invalid spatial reference");
}

#[tokio::test]
async fn from_url_throws_if_tile_info_missing() {
    let mut backend = MockResourceBackend::new();
    let mut metadata = base_metadata();
    // JS: `delete metadata.tileInfo;`
    metadata.as_object_mut().unwrap().remove("tileInfo");
    register_metadata(&mut backend, &metadata);

    let error = ArcGISTiledElevationTerrainProvider::from_url(Some(BASE_URL), None, &backend)
        .await
        .err()
        .expect("expected error");
    assert_eq!(error.message, "tileInfo is required");
}

#[tokio::test]
async fn checks_availability_if_tile_map_capability_exists() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let provider = make_provider(&backend).await;

    assert!(provider.has_availability());
    assert!(provider.availability().is_some());
    // `_tilesAvailabilityLoaded` is internal; its presence is implied by
    // `has_availability` (built together in `parse_metadata_success`).
}

#[tokio::test]
async fn does_not_check_availability_if_tile_map_capability_is_missing() {
    let mut backend = MockResourceBackend::new();
    let mut metadata = base_metadata();
    metadata["capabilities"] = json!("Image,Mensuration");
    register_metadata(&mut backend, &metadata);

    let provider = make_provider(&backend).await;

    assert!(!provider.has_availability());
    assert!(provider.availability().is_none());
}

// ── requestTileGeometry ─────────────────────────────────────────────────

#[tokio::test]
async fn request_tile_geometry_provides_heightmap_terrain_data() {
    let mut backend = MockResourceBackend::new();
    register_metadata(&mut backend, &base_metadata());

    let mut terrain_provider = make_provider(&backend).await;

    // requestTileGeometry(0, 0, 0) needs the child availability of level 1
    // first (the JS mock answers every `/tilemap/` request with the
    // availability fixture).
    backend.register_json_response(
        &derived_url("tilemap/1/0/0/2/2"),
        &availability_fixture().to_string(),
    );
    backend.register_response(&derived_url("tile/0/0/0"), make_tile_bytes());

    let loaded_data = terrain_provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap();
    assert!(loaded_data.is_some());
}
