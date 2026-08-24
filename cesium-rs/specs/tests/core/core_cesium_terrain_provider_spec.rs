//! Mirror of `packages/engine/Specs/Core/CesiumTerrainProviderSpec.js` (633 lines).
//!
//! All `Data/CesiumTerrainTileJson/*` fixtures are reproduced verbatim as
//! mock `layer.json` documents registered on [`MockResourceBackend`] under
//! `https://terrain.test/<fixture>/layer.json`; quantized-mesh / heightmap
//! tile bodies are synthesized by [`build_quantized_mesh_tile`] /
//! [`make_heightmap_tile`] following the quantized-mesh 1.0 binary layout.
//!
//! # Skipped JS tests (DEVIATION)
//! - "fromUrl resolves with url promise" / "fromUrl rejects if url rejects":
//!   Rust callers resolve the url before the call (module DEVIATION 1).
//! - "supports scheme-less template URLs in layer.json resolved with absolute
//!   URL": depends on `document.baseURI` via `getAbsoluteUri` (no DOM).
//! - "returns undefined if too many requests are already in progress":
//!   RequestScheduler throttling is not modeled (module DEVIATION 2).

use std::collections::HashMap;

use serde_json::{json, Value};

use cesium_core::cesium_terrain_provider::{
    get_availability_tile, AvailabilityTile, CesiumTerrainProvider, CesiumTerrainProviderOptions,
    TerrainTileData,
};
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ion_resource::{IonEndpoint, IonResource};
use cesium_core::resource::{DerivedResourceOptions, MockResourceBackend, Resource};
use cesium_core::terrain_provider::TerrainProvider;

const BASE: &str = "https://terrain.test";

// ── Fixture helpers ─────────────────────────────────────────────────────

fn register_fixture(backend: &mut MockResourceBackend, path: &str, json: &Value) {
    backend.register_json_response(
        &format!("{BASE}/{path}/layer.json"),
        &json.to_string(),
    );
}

/// The `available` array shared by most quantized-mesh fixtures
/// (level 0 full, level 1 full).
fn qm_available_2_levels() -> Value {
    json!([
        [{"startX": 0, "startY": 0, "endX": 1, "endY": 0}],
        [{"startX": 0, "startY": 0, "endX": 3, "endY": 1}]
    ])
}

fn heightmap_layer_json() -> Value {
    json!({
        "tilejson": "2.1.0",
        "format": "heightmap-1.0",
        "version": "1.0.0",
        "scheme": "tms",
        "tiles": ["11_3027_1342.terrain?v={version}&x={x}&y={y}&z={z}"]
    })
}

fn standard_qm_layer_json(template: &str, extensions: &[&str]) -> Value {
    let mut value = json!({
        "tilejson": "2.1.0",
        "format": "quantized-mesh-1.0",
        "version": "1.0.0",
        "scheme": "tms",
        "tiles": [template],
        "available": qm_available_2_levels(),
    });
    if !extensions.is_empty() {
        value["extensions"] = Value::Array(
            extensions.iter().map(|e| Value::String((*e).to_string())).collect(),
        );
    }
    value
}

/// A heightmap-1.0 tile: 65×65 u16 heights (little endian) + 1 child-tile-mask
/// byte + 1 water-mask byte (mirrors the 11_3027_1342.terrain layout).
fn make_heightmap_tile(first_height: u16, child_tile_mask: u8) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(65 * 65 * 2 + 2);
    buffer.extend_from_slice(&first_height.to_le_bytes());
    for _ in 1..(65 * 65) {
        buffer.extend_from_slice(&0u16.to_le_bytes());
    }
    buffer.push(child_tile_mask);
    buffer.push(255);
    buffer
}

/// Options for [`build_quantized_mesh_tile`].
#[derive(Default)]
struct QmTileOptions {
    vertex_count: usize,
    /// Append the oct-encoded vertex normals extension (id 1).
    oct_normals: bool,
    /// The layer advertises the legacy `vertexnormals` extension: extension
    /// lengths are written big-endian.
    legacy_normals: bool,
    /// Append the water mask extension (id 2).
    water_mask: bool,
    /// Append the metadata extension (id 4) carrying this JSON.
    metadata_json: Option<String>,
    /// Append an unknown extension (id 200, zero length).
    unknown_ext: bool,
}

/// Builds a quantized-mesh 1.0 tile buffer: `vertex_count` zig-zag encoded
/// vertices, one high-water-mark encoded triangle ([0,0,0] codes), one index
/// per edge, then the requested extensions.
fn build_quantized_mesh_tile(options: &QmTileOptions) -> Vec<u8> {
    fn f64le(w: &mut Vec<u8>, v: f64) {
        w.extend_from_slice(&v.to_le_bytes());
    }
    fn f32le(w: &mut Vec<u8>, v: f32) {
        w.extend_from_slice(&v.to_le_bytes());
    }
    fn u32le(w: &mut Vec<u8>, v: u32) {
        w.extend_from_slice(&v.to_le_bytes());
    }
    fn u16le(w: &mut Vec<u8>, v: u16) {
        w.extend_from_slice(&v.to_le_bytes());
    }
    fn write_index(w: &mut Vec<u8>, bytes_per_index: usize, v: u32) {
        if bytes_per_index == 4 {
            w.extend_from_slice(&v.to_le_bytes());
        } else {
            w.extend_from_slice(&(v as u16).to_le_bytes());
        }
    }
    fn write_ext_len(w: &mut Vec<u8>, little_endian: bool, length: usize) {
        if little_endian {
            w.extend_from_slice(&(length as u32).to_le_bytes());
        } else {
            w.extend_from_slice(&(length as u32).to_be_bytes());
        }
    }

    let n = options.vertex_count.max(3);
    let mut w: Vec<u8> = Vec::new();

    f64le(&mut w, 1.0);
    f64le(&mut w, 2.0);
    f64le(&mut w, 3.0); // center
    f32le(&mut w, 10.0);
    f32le(&mut w, 20.0); // minimumHeight / maximumHeight
    f64le(&mut w, 1.0);
    f64le(&mut w, 2.0);
    f64le(&mut w, 3.0);
    f64le(&mut w, 100.0); // boundingSphere
    f64le(&mut w, 1.0);
    f64le(&mut w, 2.0);
    f64le(&mut w, 3.0); // horizonOcclusionPoint
    u32le(&mut w, n as u32); // vertexCount

    // Encoded vertices: u / v / height runs of zig-zag deltas. Small meshes
    // decode to u = 10·i, v = 5·i, h = 1000·i; large meshes use zeros so the
    // values stay inside the u16 range.
    if n <= 3 {
        for encoded in [0u16, 200, 200] {
            u16le(&mut w, encoded); // u
        }
        for encoded in [0u16, 100, 100] {
            u16le(&mut w, encoded); // v
        }
        for encoded in [0u16, 2000, 2000] {
            u16le(&mut w, encoded); // height
        }
    } else {
        for _ in 0..(n * 3) {
            u16le(&mut w, 0);
        }
    }

    let bytes_per_index = if n > 64 * 1024 { 4usize } else { 2 };
    if w.len() % bytes_per_index != 0 {
        let padding = bytes_per_index - (w.len() % bytes_per_index);
        for _ in 0..padding {
            w.push(0);
        }
    }

    u32le(&mut w, 1); // triangleCount
    for _ in 0..3 {
        write_index(&mut w, bytes_per_index, 0); // codes [0,0,0] -> [0, 1, 2]
    }
    for edge_index in [0u32, 0, 1, 2] {
        // west / south / east / north: one index each
        u32le(&mut w, 1);
        write_index(&mut w, bytes_per_index, edge_index);
    }

    let little_endian_extension_size = !options.legacy_normals;

    if options.oct_normals {
        w.push(1); // QuantizedMeshExtensionIds.OCT_VERTEX_NORMALS
        write_ext_len(&mut w, little_endian_extension_size, 2 * n);
        for _ in 0..(2 * n) {
            w.push(0xAB);
        }
    }
    if options.water_mask {
        w.push(2); // QuantizedMeshExtensionIds.WATER_MASK
        write_ext_len(&mut w, little_endian_extension_size, 4);
        w.extend_from_slice(&[255, 255, 0, 0]);
    }
    if let Some(metadata) = &options.metadata_json {
        w.push(4); // QuantizedMeshExtensionIds.METADATA
        write_ext_len(&mut w, little_endian_extension_size, 4 + metadata.len());
        w.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
        w.extend_from_slice(metadata.as_bytes());
    }
    if options.unknown_ext {
        w.push(200);
        write_ext_len(&mut w, little_endian_extension_size, 0);
    }

    w
}

/// Mirrors the URL construction of `request_tile_geometry_from_layer` so the
/// exact fetch URL can be registered on the mock backend (and asserted on,
/// like the JS spec asserts `request.url`).
fn expected_tile_url(
    provider: &CesiumTerrainProvider,
    layer_index: usize,
    x: i32,
    y: i32,
    level: i32,
    query: Option<&HashMap<String, String>>,
) -> String {
    let layer = &provider.layers()[layer_index];
    let terrain_y = if provider.scheme() == Some("slippyMap") {
        y
    } else {
        let y_tiles = provider
            .tiling_scheme()
            .get_number_of_y_tiles_at_level(level);
        y_tiles - y - 1
    };
    let templates = &layer.tile_url_templates;
    let template = templates
        [((x + terrain_y + level).rem_euclid(templates.len() as i32)) as usize]
        .clone();
    let mut template_values = HashMap::new();
    if let Some(version) = &layer.version {
        template_values.insert("version".to_string(), version.clone());
    }
    template_values.insert("z".to_string(), level.to_string());
    template_values.insert("x".to_string(), x.to_string());
    template_values.insert("y".to_string(), terrain_y.to_string());
    layer
        .resource
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(&template),
            template_values: Some(&template_values),
            query_parameters: query,
            ..Default::default()
        })
        .url()
}

fn register_qm_tile(
    backend: &mut MockResourceBackend,
    provider: &CesiumTerrainProvider,
    x: i32,
    y: i32,
    level: i32,
    query: Option<&HashMap<String, String>>,
    body: Vec<u8>,
) -> String {
    let url = expected_tile_url(provider, 0, x, y, level, query);
    backend.register_response(&url, body);
    url
}

const METADATA_AVAILABLE_LEVEL_1: &str =
    r#"{"available":[[{"startX":0,"startY":1,"endX":0,"endY":1}]]}"#;

// ── Interface / construction ────────────────────────────────────────────

/// Mirrors "conforms to TerrainProvider interface".
#[test]
fn conforms_to_terrain_provider_interface() {
    fn assert_terrain_provider<T: TerrainProvider>(_: &T) {}
    let provider = CesiumTerrainProvider::new();
    assert_terrain_provider(&provider);
}

/// Mirrors "fromIonAssetId throws without assetId".
#[tokio::test]
#[should_panic(expected = "assetId is required, actual value was undefined")]
async fn from_ion_asset_id_throws_without_asset_id() {
    let backend = MockResourceBackend::new();
    let _ = CesiumTerrainProvider::from_ion_asset_id(None, None, &backend).await;
}

/// Mirrors "fromUrl throws without url".
#[tokio::test]
#[should_panic(expected = "url is required, actual value was undefined")]
async fn from_url_throws_without_url() {
    let backend = MockResourceBackend::new();
    let _ = CesiumTerrainProvider::from_url(None, None, &backend).await;
}

/// Mirrors "fromUrl resolves to created CesiumTerrainProvider".
#[tokio::test]
async fn from_url_resolves_to_created_provider() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        None,
        &backend,
    )
    .await
    .unwrap();
    assert_eq!(provider.layers().len(), 1);
}

/// Mirrors "fromUrl resolves with Resource".
#[tokio::test]
async fn from_url_resolves_with_resource() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let resource = Resource::new(format!("{BASE}/Heightmap"));
    let provider = CesiumTerrainProvider::from_resource(resource, None, &backend)
        .await
        .unwrap();
    assert_eq!(provider.layers().len(), 1);
}

/// Mirrors "uses geographic tiling scheme by default" (geographic schemes
/// have 2 level-zero tiles in x).
#[tokio::test]
async fn uses_geographic_tiling_scheme_by_default() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        None,
        &backend,
    )
    .await
    .unwrap();
    assert_eq!(provider.tiling_scheme().get_number_of_x_tiles_at_level(0), 2);
    assert_eq!(provider.tiling_scheme().get_number_of_y_tiles_at_level(0), 1);
}

/// Mirrors "can use a custom ellipsoid".
#[tokio::test]
async fn can_use_a_custom_ellipsoid() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let ellipsoid = Ellipsoid::new(1.0, 2.0, 3.0);
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        Some(CesiumTerrainProviderOptions {
            ellipsoid: Some(ellipsoid),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    assert_eq!(*provider.tiling_scheme().ellipsoid(), ellipsoid);
}

/// Mirrors "has error event".
#[tokio::test]
async fn has_error_event() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        None,
        &backend,
    )
    .await
    .unwrap();
    let _ = provider.error_event();
}

/// Mirrors "returns reasonable geometric error for various levels".
#[tokio::test]
async fn returns_reasonable_geometric_error_for_various_levels() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMesh",
        &standard_qm_layer_json("tile.terrain?v={version}&x={x}&y={y}&z={z}", &[]),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMesh")),
        None,
        &backend,
    )
    .await
    .unwrap();
    let err0 = provider.get_level_maximum_geometric_error(0);
    let err1 = provider.get_level_maximum_geometric_error(1);
    let err2 = provider.get_level_maximum_geometric_error(2);
    assert!(err0 > 0.0);
    assert!((err0 - err1 * 2.0).abs() < 1.0e-10);
    assert!((err1 - err2 * 2.0).abs() < 1.0e-10);
}

/// Mirrors "credit is undefined if credit option is not provided".
#[tokio::test]
async fn credit_is_none_if_credit_option_is_not_provided() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        None,
        &backend,
    )
    .await
    .unwrap();
    assert!(provider.credit().is_none());
}

/// Mirrors "credit is defined if credit option is provided".
#[tokio::test]
async fn credit_is_defined_if_credit_option_is_provided() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        Some(CesiumTerrainProviderOptions {
            credit: Some("thanks to our awesome made up contributors!".to_string()),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    assert!(provider.credit().is_some());
}

/// Mirrors "has a water mask" (heightmap-1.0 implies a water mask and forces
/// requestWaterMask to true in TerrainProviderBuilder).
#[tokio::test]
async fn has_a_water_mask() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        None,
        &backend,
    )
    .await
    .unwrap();
    assert!(provider.has_water_mask());
}

/// Mirrors "has vertex normals".
#[tokio::test]
async fn has_vertex_normals() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithOctVertexNormals",
        &standard_qm_layer_json(
            "tile.octvertexnormals.terrain?v={version}&x={x}&y={y}&z={z}",
            &["octvertexnormals"],
        ),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithOctVertexNormals")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    assert!(provider.request_vertex_normals());
    assert!(provider.has_vertex_normals());
}

/// Mirrors "does not request vertex normals".
#[tokio::test]
async fn does_not_request_vertex_normals() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithOctVertexNormals",
        &standard_qm_layer_json(
            "tile.octvertexnormals.terrain?v={version}&x={x}&y={y}&z={z}",
            &["octvertexnormals"],
        ),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithOctVertexNormals")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(false),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    assert!(!provider.request_vertex_normals());
    assert!(!provider.has_vertex_normals());
}

/// Mirrors "requests parent layer json".
#[tokio::test]
async fn requests_parent_layer_json() {
    let mut backend = MockResourceBackend::new();

    // Parent fixture: Data/CesiumTerrainTileJson/QuantizedMeshWithParentUrl
    register_fixture(
        &mut backend,
        "QuantizedMeshWithParentUrl",
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This amazing data is courtesy The Amazing Data Source!",
            "tiles": ["tile.terrain?v={version}&x={x}&y={y}&z={z}"],
            "available": qm_available_2_levels(),
        }),
    );
    // Child fixture: .../QuantizedMeshWithParentUrl/ChildTileset
    register_fixture(
        &mut backend,
        "QuantizedMeshWithParentUrl/ChildTileset",
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This is a child tileset!",
            "tiles": ["tile.terrain?v={version}&x={x}&y={y}&z={z}"],
            "extensions": ["watermask"],
            "available": [
                [{"startX": 0, "startY": 0, "endX": 1, "endY": 0}],
                [{"startX": 0, "startY": 0, "endX": 2, "endY": 1}]
            ],
            "parentUrl": "../",
        }),
    );
    // The parent layer.json is fetched through the literal
    // `getDerivedResource("../")` concatenation (child + "../" + layer.json).
    backend.register_json_response(
        &format!("{BASE}/QuantizedMeshWithParentUrl/ChildTileset/../layer.json"),
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This amazing data is courtesy The Amazing Data Source!",
            "tiles": ["tile.terrain?v={version}&x={x}&y={y}&z={z}"],
            "available": qm_available_2_levels(),
        })
        .to_string(),
    );

    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithParentUrl/ChildTileset")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            request_water_mask: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();

    assert_eq!(
        provider.tile_credits()[0].html(),
        "This is a child tileset! This amazing data is courtesy The Amazing Data Source!"
    );
    assert!(provider.request_vertex_normals());
    assert!(provider.request_water_mask());
    assert!(!provider.has_vertex_normals()); // Neither tileset has them
    assert!(provider.has_water_mask()); // The child tileset has them
    let availability = provider.availability().unwrap();
    assert!(availability.is_tile_available(1, 2, 1)); // Both have this
    assert!(availability.is_tile_available(1, 3, 1)); // Parent has this, but child doesn't
    assert!(!availability.is_tile_available(2, 0, 0)); // Neither has this

    let layers = provider.layers();
    assert_eq!(layers.len(), 2);
    assert!(!layers[0].has_vertex_normals);
    assert!(layers[0].has_water_mask);
    let layer0 = layers[0].availability.as_ref().unwrap();
    assert!(layer0.is_tile_available(1, 2, 1));
    assert!(!layer0.is_tile_available(1, 3, 1));
    assert!(!layer0.is_tile_available(2, 0, 0));
    assert!(!layers[1].has_vertex_normals);
    assert!(!layers[1].has_water_mask);
    let layer1 = layers[1].availability.as_ref().unwrap();
    assert!(layer1.is_tile_available(1, 2, 1));
    assert!(layer1.is_tile_available(1, 3, 1));
    assert!(!layer1.is_tile_available(2, 0, 0));
}

/// Mirrors "fromUrl throws if layer.json does not specify a format".
#[tokio::test]
async fn from_url_throws_if_layer_json_does_not_specify_a_format() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "NoFormat",
        &json!({
            "tilejson": "2.1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "tiles": ["{z}/{x}/{y}.terrain?v={version}"]
        }),
    );
    let Err(error) =
        CesiumTerrainProvider::from_url(Some(&format!("{BASE}/NoFormat")), None, &backend).await
    else {
        panic!("expected error");
    };
    assert_eq!(
        error.message,
        "The tile format is not specified in the layer.json file."
    );
}

/// Mirrors "fromUrl throws if layer.json specifies an unknown format".
#[tokio::test]
async fn from_url_throws_if_layer_json_specifies_an_unknown_format() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "InvalidFormat",
        &json!({
            "tilejson": "2.1.0",
            "format": "awesometron-9000.0",
            "version": "1.0.0",
            "scheme": "tms",
            "tiles": ["{z}/{x}/{y}.terrain?v={version}"]
        }),
    );
    let Err(error) =
        CesiumTerrainProvider::from_url(Some(&format!("{BASE}/InvalidFormat")), None, &backend)
            .await
    else {
        panic!("expected error");
    };
    assert_eq!(
        error.message,
        "The tile format \"awesometron-9000.0\" is invalid or not supported."
    );
}

/// Mirrors "fromUrl throws if layer.json does not specify quantized-mesh 1.x
/// format".
#[tokio::test]
async fn from_url_throws_if_layer_json_specifies_quantized_mesh_2_0() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMesh2.0",
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-2.0",
            "version": "1.0.0",
            "scheme": "tms",
            "tiles": ["{z}/{x}/{y}.terrain?v={version}"],
            "available": qm_available_2_levels(),
        }),
    );
    let Err(error) =
        CesiumTerrainProvider::from_url(Some(&format!("{BASE}/QuantizedMesh2.0")), None, &backend)
            .await
    else {
        panic!("expected error");
    };
    assert_eq!(
        error.message,
        "The tile format \"quantized-mesh-2.0\" is invalid or not supported."
    );
}

/// Mirrors "fromUrl supports quantized-mesh1.x minor versions".
#[tokio::test]
async fn from_url_supports_quantized_mesh_1_x_minor_versions() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMesh1.1",
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.1",
            "version": "1.0.0",
            "scheme": "tms",
            "tiles": ["{z}/{x}/{y}.terrain?v={version}"],
            "available": qm_available_2_levels(),
        }),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMesh1.1")),
        None,
        &backend,
    )
    .await;
    assert!(provider.is_ok());
}

/// Mirrors "fromUrl throws if layer.json does not specify a tiles property".
#[tokio::test]
async fn from_url_throws_if_layer_json_does_not_specify_tiles() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "NoTiles",
        &json!({
            "tilejson": "2.1.0",
            "format": "heightmap-1.0",
            "version": "1.0.0",
            "scheme": "tms"
        }),
    );
    let Err(error) =
        CesiumTerrainProvider::from_url(Some(&format!("{BASE}/NoTiles")), None, &backend).await
    else {
        panic!("expected error");
    };
    assert_eq!(
        error.message,
        "The layer.json file does not specify any tile URL templates."
    );
}

/// Mirrors "fromUrl throws if layer.json tiles property is an empty array".
#[tokio::test]
async fn from_url_throws_if_layer_json_tiles_is_empty() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "EmptyTilesArray",
        &json!({
            "tilejson": "2.1.0",
            "format": "heightmap-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "tiles": []
        }),
    );
    let Err(error) =
        CesiumTerrainProvider::from_url(Some(&format!("{BASE}/EmptyTilesArray")), None, &backend)
            .await
    else {
        panic!("expected error");
    };
    assert_eq!(
        error.message,
        "The layer.json file does not specify any tile URL templates."
    );
}

/// Mirrors "fromUrl uses attribution specified in layer json".
#[tokio::test]
async fn from_url_uses_attribution_specified_in_layer_json() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "HeightmapWithAttribution",
        &json!({
            "tilejson": "2.1.0",
            "format": "heightmap-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This amazing data is courtesy The Amazing Data Source!",
            "tiles": ["{z}/{x}/{y}.terrain?v={version}"]
        }),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/HeightmapWithAttribution")),
        None,
        &backend,
    )
    .await
    .unwrap();
    assert_eq!(
        provider.tile_credits()[0].html(),
        "This amazing data is courtesy The Amazing Data Source!"
    );
}

/// Mirrors "formUrl does not add blank attribution if layer.json does not
/// have one".
#[tokio::test]
async fn from_url_does_not_add_blank_attribution() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithWaterMask",
        &standard_qm_layer_json(
            "tile.watermask.terrain?v={version}&x={x}&y={y}&z={z}",
            &["watermask"],
        ),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithWaterMask")),
        Some(CesiumTerrainProviderOptions {
            request_water_mask: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    assert!(provider.tile_credits().is_empty());
}

// ── _getAvailabilityTile ────────────────────────────────────────────────

/// Mirrors "The undefined availability tile is returned at level 0".
#[test]
fn availability_tile_is_none_at_level_0() {
    assert_eq!(get_availability_tile(10, 0, 0, 0), None);
    assert_eq!(get_availability_tile(10, 1, 0, 0), None);
}

/// Mirrors "The correct availability tile is computed in first level".
#[test]
fn correct_availability_tile_computed_in_first_level() {
    assert_eq!(
        get_availability_tile(10, 1, 1, 1),
        Some(AvailabilityTile { level: 0, x: 0, y: 0 })
    );
    assert_eq!(
        get_availability_tile(10, 4, 2, 2),
        Some(AvailabilityTile { level: 0, x: 1, y: 0 })
    );
    assert_eq!(
        get_availability_tile(10, 80, 50, 10),
        Some(AvailabilityTile { level: 0, x: 0, y: 0 })
    );
}

/// Mirrors "The correct availability tile is computed in second level".
#[test]
fn correct_availability_tile_computed_in_second_level() {
    let expected = AvailabilityTile {
        level: 10,
        x: 80,
        y: 50,
    };
    let mut xs = [expected.x, expected.x];
    let mut ys = [expected.y, expected.y];
    // Compute level 20 tiles by always taking SW or NE child
    for _ in 0..10 {
        xs[0] *= 2;
        ys[0] *= 2;
        xs[1] = xs[1] * 2 + 1;
        ys[1] = ys[1] * 2 + 1;
    }
    assert_eq!(get_availability_tile(10, xs[0], ys[0], 20), Some(expected));
    assert_eq!(get_availability_tile(10, xs[1], ys[1], 20), Some(expected));
}

// ── requestTileGeometry ─────────────────────────────────────────────────

/// Mirrors "uses multiple urls specified in layer json".
#[tokio::test]
async fn uses_multiple_urls_specified_in_layer_json() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "MultipleUrls",
        &json!({
            "tilejson": "2.1.0",
            "format": "heightmap-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "tiles": [
                "../Heightmap/11_3027_1342.terrain?v={version}&x={x}&y={y}&z={z}&foo=0",
                "../Heightmap/11_3027_1342.terrain?v={version}&x={x}&y={y}&z={z}&foo=1",
                "../Heightmap/11_3027_1342.terrain?v={version}&x={x}&y={y}&z={z}&foo=2",
                "../Heightmap/11_3027_1342.terrain?v={version}&x={x}&y={y}&z={z}&foo=3"
            ]
        }),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/MultipleUrls")),
        None,
        &backend,
    )
    .await
    .unwrap();

    // Register a distinct heightmap body behind each template URL and check
    // that the requested URL selects the expected template (JS asserts
    // `request.url` contains foo=N).
    for (x, y, level, foo) in [(0, 0, 0, 0u16), (1, 0, 0, 1), (1, -1, 0, 2), (1, 0, 1, 3)] {
        let url = expected_tile_url(&provider, 0, x, y, level, None);
        assert!(
            url.contains(&format!("foo={foo}")),
            "expected foo={foo} in {url}"
        );
        backend.register_response(&url, make_heightmap_tile(foo, 15));
    }

    for (x, y, level, foo) in [(0, 0, 0, 0u16), (1, 0, 0, 1), (1, -1, 0, 2), (1, 0, 1, 3)] {
        let data = provider
            .request_tile_geometry(x, y, level, &backend)
            .await
            .unwrap()
            .unwrap();
        let TerrainTileData::Heightmap(heightmap) = data else {
            panic!("expected HeightmapTerrainData");
        };
        assert_eq!(heightmap.buffer().unwrap().get(0), f64::from(foo));
    }
}

/// Mirrors "provides HeightmapTerrainData".
#[tokio::test]
async fn provides_heightmap_terrain_data() {
    let mut backend = MockResourceBackend::new();
    register_fixture(&mut backend, "Heightmap", &heightmap_layer_json());
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap")),
        None,
        &backend,
    )
    .await
    .unwrap();
    let url = expected_tile_url(&provider, 0, 0, 0, 0, None);
    backend.register_response(&url, make_heightmap_tile(1234, 15));
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::Heightmap(heightmap) = data else {
        panic!("expected HeightmapTerrainData");
    };
    assert_eq!(heightmap.width(), 65);
    assert_eq!(heightmap.child_tile_mask(), 15);
}

/// Mirrors "provides QuantizedMeshTerrainData".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMesh",
        &standard_qm_layer_json("tile.terrain?v={version}&x={x}&y={y}&z={z}", &[]),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMesh")),
        None,
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert_eq!(mesh.minimum_height(), 10.0);
    assert_eq!(mesh.maximum_height(), 20.0);
    // High-water codes [0, 0, 0] decode to [0, 1, 2].
    assert_eq!(mesh.indices().unwrap(), &vec![0u32, 1, 2]);
}

/// Mirrors "provides QuantizedMeshTerrainData with 32bit indices".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_32bit_indices() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWith32BitIndices",
        &standard_qm_layer_json(
            "tile.32bitIndices.terrain?v={version}&x={x}&y={y}&z={z}",
            &[],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWith32BitIndices")),
        None,
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 65537, // > 64k vertices -> 32-bit indices
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    // JS checks `data._indices.BYTES_PER_ELEMENT === 4`; the Rust port reads
    // 4-byte indices once vertexCount > 64k.
    assert_eq!(mesh.indices().unwrap(), &vec![0u32, 1, 2]);
    assert_eq!(mesh.quantized_vertices().unwrap().len(), 65537 * 3);
}

/// Mirrors "provides QuantizedMeshTerrainData with VertexNormals" (legacy
/// big-endian extension length).
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_vertex_normals() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithVertexNormals",
        &standard_qm_layer_json(
            "tile.vertexnormals.terrain?v={version}&x={x}&y={y}&z={z}",
            &["vertexnormals"],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithVertexNormals")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            oct_normals: true,
            legacy_normals: true,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert!(mesh.encoded_normals().is_some());
}

/// Mirrors "provides QuantizedMeshTerrainData with WaterMask".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_water_mask() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithWaterMask",
        &standard_qm_layer_json(
            "tile.watermask.terrain?v={version}&x={x}&y={y}&z={z}",
            &["watermask"],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithWaterMask")),
        Some(CesiumTerrainProviderOptions {
            request_water_mask: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            water_mask: true,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert!(mesh.water_mask().is_some());
}

/// Mirrors "provides QuantizedMeshTerrainData with VertexNormals and
/// WaterMask".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_vertex_normals_and_water_mask() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithOctVertexNormalsAndWaterMask",
        &standard_qm_layer_json(
            "tile.octvertexnormals.watermask.terrain?v={version}&x={x}&y={y}&z={z}",
            &["octvertexnormals", "watermask"],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithOctVertexNormalsAndWaterMask")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            request_water_mask: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            oct_normals: true,
            water_mask: true,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert!(mesh.encoded_normals().is_some());
    assert!(mesh.water_mask().is_some());
}

/// Mirrors "provides QuantizedMeshTerrainData with OctVertexNormals".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_oct_vertex_normals() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithOctVertexNormals",
        &standard_qm_layer_json(
            "tile.octvertexnormals.terrain?v={version}&x={x}&y={y}&z={z}",
            &["octvertexnormals"],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithOctVertexNormals")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            oct_normals: true,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert!(mesh.encoded_normals().is_some());
}

/// Mirrors "provides QuantizedMeshTerrainData with VertexNormals and unknown
/// extensions".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_vertex_normals_and_unknown_ext() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithVertexNormalsAndUnknownExt",
        &standard_qm_layer_json(
            "tile.vertexnormals.unknownext.terrain?v={version}&x={x}&y={y}&z={z}",
            &["vertexnormals"],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithVertexNormalsAndUnknownExt")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            oct_normals: true,
            legacy_normals: true,
            unknown_ext: true,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert!(mesh.encoded_normals().is_some());
}

/// Mirrors "provides QuantizedMeshTerrainData with OctVertexNormals and
/// unknown extensions".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_oct_vertex_normals_and_unknown_ext() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithOctVertexNormalsAndUnknownExt",
        &standard_qm_layer_json(
            "tile.octvertexnormals.unknownext.terrain?v={version}&x={x}&y={y}&z={z}",
            &["octvertexnormals"],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithOctVertexNormalsAndUnknownExt")),
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            oct_normals: true,
            unknown_ext: true,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert!(mesh.encoded_normals().is_some());
}

/// Mirrors "provides QuantizedMeshTerrainData with unknown extension".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_unknown_ext() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithUnknownExt",
        &standard_qm_layer_json(
            "tile.unknownext.terrain?v={version}&x={x}&y={y}&z={z}",
            &[],
        ),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithUnknownExt")),
        None,
        &backend,
    )
    .await
    .unwrap();
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            unknown_ext: true,
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(data, TerrainTileData::QuantizedMesh(_)));
}

/// Mirrors "provides QuantizedMeshTerrainData with Metadata availability".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_metadata_availability() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithMetadataAvailability",
        &json!({
            "format": "quantized-mesh-1.0",
            "version": "1.33.0",
            "scheme": "tms",
            "attribution": "",
            "bounds": [-180, -90, 180, 90],
            "tiles": ["tile.metadataavailability.terrain?v={version}&x={x}&y={y}&z={z}"],
            "extensions": ["bvh", "metadata", "octvertexnormals"],
            "metadataAvailability": 10,
            "minzoom": 0,
            "maxzoom": 8,
            "projection": "EPSG:4326",
            "bvhlevels": 6
        }),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithMetadataAvailability")),
        None,
        &backend,
    )
    .await
    .unwrap();

    assert!(provider.has_metadata());
    assert_eq!(provider.layers()[0].availability_levels, Some(10));
    let availability = provider.availability().unwrap();
    assert!(availability.is_tile_available(0, 0, 0));
    assert!(availability.is_tile_available(0, 1, 0));
    assert!(!availability.is_tile_available(1, 0, 0));

    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            // y-flip: startY/endY 1 at level 1 (yTiles 2) -> tms y 0.
            metadata_json: Some(METADATA_AVAILABLE_LEVEL_1.to_string()),
            ..Default::default()
        }),
    );
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(data, TerrainTileData::QuantizedMesh(_)));
    assert!(provider.availability().unwrap().is_tile_available(1, 0, 0));
}

/// Mirrors "provides QuantizedMeshTerrainData with multiple layers and with
/// Metadata availability".
#[tokio::test]
async fn provides_quantized_mesh_terrain_data_with_multiple_layers_metadata_availability() {
    let mut backend = MockResourceBackend::new();
    // Parent fixture
    register_fixture(
        &mut backend,
        "QuantizedMeshWithParentUrlMetadataAvailability",
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This amazing data is courtesy The Amazing Data Source!",
            "tiles": ["tile.metadataavailability.terrain?v={version}&x={x}&y={y}&z={z}"],
            "extensions": ["watermask", "metadata", "octvertexnormals"],
            "metadataAvailability": 10,
            "minzoom": 0,
            "maxzoom": 13,
            "available": qm_available_2_levels(),
        }),
    );
    // Child fixture
    register_fixture(
        &mut backend,
        "QuantizedMeshWithParentUrlMetadataAvailability/ChildTileset",
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This is a child tileset!",
            "tiles": ["../tile.metadataavailability.terrain?v={version}&x={x}&y={y}&z={z}"],
            "extensions": ["watermask", "metadata", "octvertexnormals"],
            "metadataAvailability": 10,
            "minzoom": 0,
            "maxzoom": 13,
            "available": [
                [{"startX": 0, "startY": 0, "endX": 1, "endY": 0}],
                [{"startX": 0, "startY": 0, "endX": 2, "endY": 1}]
            ],
            "parentUrl": "../",
        }),
    );
    backend.register_json_response(
        &format!(
            "{BASE}/QuantizedMeshWithParentUrlMetadataAvailability/ChildTileset/../layer.json"
        ),
        &json!({
            "tilejson": "2.1.0",
            "format": "quantized-mesh-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This amazing data is courtesy The Amazing Data Source!",
            "tiles": ["tile.metadataavailability.terrain?v={version}&x={x}&y={y}&z={z}"],
            "extensions": ["watermask", "metadata", "octvertexnormals"],
            "metadataAvailability": 10,
            "minzoom": 0,
            "maxzoom": 13,
            "available": qm_available_2_levels(),
        })
        .to_string(),
    );

    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!(
            "{BASE}/QuantizedMeshWithParentUrlMetadataAvailability/ChildTileset"
        )),
        None,
        &backend,
    )
    .await
    .unwrap();

    assert!(provider.has_metadata());
    let layers = provider.layers();
    assert_eq!(layers.len(), 2);

    assert!(!provider.availability().unwrap().is_tile_available(1, 0, 0));

    // requestTileGeometry(0, 0, 1) must first load the availability tile
    // (0, 0, 0) of the top layer (JS checkLayer/availabilityPromiseCache;
    // the Rust port awaits it inline) whose metadata extension grants the
    // level-1 availability.
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            metadata_json: Some(METADATA_AVAILABLE_LEVEL_1.to_string()),
            ..Default::default()
        }),
    );
    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        1,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            metadata_json: Some(METADATA_AVAILABLE_LEVEL_1.to_string()),
            ..Default::default()
        }),
    );

    let data = provider
        .request_tile_geometry(0, 0, 1, &backend)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(data, TerrainTileData::QuantizedMesh(_)));
    assert!(provider.availability().unwrap().is_tile_available(1, 0, 0));
}

/// Mirrors "supports getTileDataAvailable()".
#[tokio::test]
async fn supports_get_tile_data_available() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMesh",
        &standard_qm_layer_json("tile.terrain?v={version}&x={x}&y={y}&z={z}", &[]),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMesh")),
        None,
        &backend,
    )
    .await
    .unwrap();
    assert_eq!(provider.get_tile_data_available(0, 0, 0), Some(true));
    assert_eq!(provider.get_tile_data_available(0, 0, 2), Some(false));
}

/// Mirrors "getTileDataAvailable() converts xyz to tms".
#[tokio::test]
async fn get_tile_data_available_converts_xyz_to_tms() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "HeightmapWithPartialAvailability",
        &json!({
            "tilejson": "2.1.0",
            "format": "heightmap-1.0",
            "version": "1.0.0",
            "scheme": "tms",
            "attribution": "This amazing data is courtesy The Amazing Data Source!",
            "tiles": ["{z}/{x}/{y}.terrain?v={version}"],
            "available": [
                [{"startX": 0, "startY": 0, "endX": 1, "endY": 0}],
                [{"startX": 0, "startY": 0, "endX": 3, "endY": 1}],
                [{"startX": 1, "startY": 0, "endX": 1, "endY": 0}]
            ]
        }),
    );
    let provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/HeightmapWithPartialAvailability")),
        None,
        &backend,
    )
    .await
    .unwrap();
    assert_eq!(provider.get_tile_data_available(1, 3, 2), Some(true));
    assert_eq!(provider.get_tile_data_available(1, 0, 2), Some(false));
}

/// Mirrors "getTileDataAvailable() with Metadata availability".
#[tokio::test]
async fn get_tile_data_available_with_metadata_availability() {
    let mut backend = MockResourceBackend::new();
    register_fixture(
        &mut backend,
        "QuantizedMeshWithMetadataAvailability",
        &json!({
            "format": "quantized-mesh-1.0",
            "version": "1.33.0",
            "scheme": "tms",
            "attribution": "",
            "bounds": [-180, -90, 180, 90],
            "tiles": ["tile.metadataavailability.terrain?v={version}&x={x}&y={y}&z={z}"],
            "extensions": ["bvh", "metadata", "octvertexnormals"],
            "metadataAvailability": 10,
            "minzoom": 0,
            "maxzoom": 8,
            "projection": "EPSG:4326",
            "bvhlevels": 6
        }),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/QuantizedMeshWithMetadataAvailability")),
        None,
        &backend,
    )
    .await
    .unwrap();

    assert_eq!(provider.get_tile_data_available(0, 0, 0), Some(true));
    assert_eq!(provider.get_tile_data_available(0, 0, 1), None);

    register_qm_tile(
        &mut backend,
        &provider,
        0,
        0,
        0,
        None,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            metadata_json: Some(METADATA_AVAILABLE_LEVEL_1.to_string()),
            ..Default::default()
        }),
    );
    let _ = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(provider.get_tile_data_available(0, 0, 1), Some(true));
}

/// Mirrors "supports a query string in the base URL".
#[tokio::test]
async fn supports_a_query_string_in_the_base_url() {
    let mut backend = MockResourceBackend::new();
    // The base URL query is inherited by the derived layer.json resource.
    backend.register_json_response(
        &format!("{BASE}/Heightmap/layer.json?foo=bar"),
        &heightmap_layer_json().to_string(),
    );
    let mut provider = CesiumTerrainProvider::from_url(
        Some(&format!("{BASE}/Heightmap?foo=bar")),
        None,
        &backend,
    )
    .await
    .unwrap();

    let url = expected_tile_url(&provider, 0, 0, 0, 0, None);
    assert!(url.contains("foo=bar"), "expected foo=bar in {url}");
    backend.register_response(&url, make_heightmap_tile(7, 15));
    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(data, TerrainTileData::Heightmap(_)));
}

/// Mirrors "Uses query parameter extensions for ion resource".
#[tokio::test]
async fn uses_query_parameter_extensions_for_ion_resource() {
    let mut backend = MockResourceBackend::new();
    let endpoint_url = format!("{BASE}/QuantizedMeshWithOctVertexNormalsAndWaterMask");
    backend.register_json_response(
        &format!("{endpoint_url}/layer.json"),
        &standard_qm_layer_json(
            "tile.octvertexnormals.watermask.terrain?v={version}&x={x}&y={y}&z={z}",
            &["octvertexnormals", "watermask"],
        )
        .to_string(),
    );

    let ion = IonResource::from_endpoint(IonEndpoint {
        url: endpoint_url.clone(),
        external_type: None,
        access_token: Some("not_really_a_refresh_token".to_string()),
        options_url: None,
    })
    .unwrap();
    let mut provider = CesiumTerrainProvider::from_ion_resource(
        ion,
        Some(CesiumTerrainProviderOptions {
            request_vertex_normals: Some(true),
            request_water_mask: Some(true),
            ..Default::default()
        }),
        &backend,
    )
    .await
    .unwrap();

    // ion negotiates extensions through a query parameter instead of the
    // Accept header.
    let mut query = HashMap::new();
    query.insert(
        "extensions".to_string(),
        "octvertexnormals-watermask".to_string(),
    );
    let url = expected_tile_url(&provider, 0, 0, 0, 0, Some(&query));
    assert!(
        url.contains("extensions=octvertexnormals-watermask"),
        "expected extensions query in {url}"
    );
    backend.register_response(
        &url,
        build_quantized_mesh_tile(&QmTileOptions {
            vertex_count: 3,
            oct_normals: true,
            water_mask: true,
            ..Default::default()
        }),
    );

    let data = provider
        .request_tile_geometry(0, 0, 0, &backend)
        .await
        .unwrap()
        .unwrap();
    let TerrainTileData::QuantizedMesh(mesh) = data else {
        panic!("expected QuantizedMeshTerrainData");
    };
    assert!(mesh.encoded_normals().is_some());
}
