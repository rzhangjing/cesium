//! Mirror of `packages/engine/Specs/Scene/GltfJsonLoaderSpec.js` (CPU-side
//! subset) plus the `parseGlb.js` behaviours exercised through it.
//!
//! One `#[test]` per mirrored `it(...)` from the CesiumJS spec. Network
//! based cases (`_fetchGltf` / external buffer fetch) are not mirrored: the
//! Rust port exposes the same processing steps as synchronous functions on
//! in-memory bytes (see DEVIATION note in `gltf_json_loader.rs`).

use cesium_scene::gltf_json_loader::{GltfJsonLoader, GltfJsonLoaderState};
use cesium_scene::gltf_pipeline::parse_glb::parse_glb;

/// Rust analogue of `generateJsonBuffer(gltf)` in GltfJsonLoaderSpec.js.
fn generate_json_buffer(gltf: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(gltf).unwrap()
}

/// Rust analogue of `createGlb2(gltf)` in GltfJsonLoaderSpec.js: builds a
/// version 2 GLB container with a JSON chunk and an optional BIN chunk.
fn create_glb2(gltf: &serde_json::Value, binary: Option<&[u8]>) -> Vec<u8> {
    let mut json_chunk = serde_json::to_vec(gltf).unwrap();
    while json_chunk.len() % 4 != 0 {
        json_chunk.push(b' ');
    }

    let mut bin_chunk: Vec<u8> = Vec::new();
    if let Some(binary) = binary {
        bin_chunk.extend_from_slice(binary);
        while bin_chunk.len() % 4 != 0 {
            bin_chunk.push(0);
        }
    }

    let total_length =
        12 + 8 + json_chunk.len() + if bin_chunk.is_empty() { 0 } else { 8 + bin_chunk.len() };

    let mut glb = Vec::with_capacity(total_length);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total_length as u32).to_le_bytes());

    // JSON chunk (0x4E4F534A)
    glb.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f534au32.to_le_bytes());
    glb.extend_from_slice(&json_chunk);

    // BIN chunk (0x004E4942)
    if !bin_chunk.is_empty() {
        glb.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x004e4942u32.to_le_bytes());
        glb.extend_from_slice(&bin_chunk);
    }

    glb
}

/// Minimal glTF 2.0 asset with an external buffer URI, mirroring `gltf2` in
/// GltfJsonLoaderSpec.js.
fn gltf2_json() -> serde_json::Value {
    serde_json::json!({
        "asset": { "version": "2.0" },
        "buffers": [ { "uri": "https://example.com/external.bin", "byteLength": 12 } ],
        "bufferViews": [ { "buffer": 0, "byteOffset": 0, "byteLength": 12 } ]
    })
}

// it("load throws if an unsupported extension is required")
#[test]
fn load_throws_if_an_unsupported_extension_is_required() {
    let mut json = gltf2_json();
    json["extensionsRequired"] = serde_json::json!(["NOT_supported_extension"]);

    let mut loader = GltfJsonLoader::new();
    let error = loader
        .load_from_typed_array(&generate_json_buffer(&json))
        .unwrap_err();
    assert_eq!(
        error.message.as_str(),
        "Unsupported glTF Extension: NOT_supported_extension"
    );
    assert_eq!(loader.state(), GltfJsonLoaderState::Failed);
}

// it("loads glTF 2.0")
#[test]
fn loads_gltf_2_0() {
    let json = gltf2_json();

    let mut loader = GltfJsonLoader::new();
    loader
        .load_from_typed_array(&generate_json_buffer(&json))
        .unwrap();

    assert_eq!(loader.state(), GltfJsonLoaderState::Ready);
    let gltf = loader.gltf().unwrap();
    assert_eq!(gltf.asset.version, "2.0");
    // DEVIATION: external buffer fetch is deferred (T5); the URI is kept.
    assert_eq!(
        gltf.buffers[0].uri.as_deref(),
        Some("https://example.com/external.bin")
    );
    assert_eq!(gltf.buffers[0].byte_length, 12);
    assert_eq!(gltf.buffer_views.len(), 1);
}

// it("loads glTF 2.0 binary")
#[test]
fn loads_gltf_2_0_binary() {
    let mut json = gltf2_json();
    json["buffers"][0].as_object_mut().unwrap().remove("uri");

    let binary: Vec<u8> = (0..12u8).collect();
    let typed_array = create_glb2(&json, Some(&binary));

    let mut loader = GltfJsonLoader::new();
    loader.load_from_typed_array(&typed_array).unwrap();

    let gltf = loader.gltf().unwrap();
    assert_eq!(gltf.buffers[0].uri, None);
    // The BIN chunk is attached to buffers[0].data.
    assert_eq!(gltf.buffers[0].data.as_deref(), Some(binary.as_slice()));
}

// it("loads glTF 2.0 with data uri")
#[test]
fn loads_gltf_2_0_with_data_uri() {
    let mut json = gltf2_json();
    json["buffers"][0]["uri"] =
        serde_json::json!("data:application/octet-stream;base64,AAAAAAAAAAAAAAAA");

    let mut loader = GltfJsonLoader::new();
    loader
        .load_from_typed_array(&generate_json_buffer(&json))
        .unwrap();

    let gltf = loader.gltf().unwrap();
    // The data URI is decoded into `data` and removed from the JSON.
    assert_eq!(gltf.buffers[0].uri, None);
    assert_eq!(gltf.buffers[0].data.as_deref().map(|d| d.len()), Some(12));
    assert!(gltf.buffers[0].data.as_ref().unwrap().iter().all(|b| *b == 0));
}

// it("loads typed array")
#[test]
fn loads_typed_array() {
    let mut json = gltf2_json();
    json["buffers"][0].as_object_mut().unwrap().remove("uri");

    let typed_array = create_glb2(&json, None);

    let mut loader = GltfJsonLoader::new();
    loader.load_from_typed_array(&typed_array).unwrap();

    let gltf = loader.gltf().unwrap();
    assert_eq!(gltf.asset.version, "2.0");
}

// it("loads JSON directly")
#[test]
fn loads_json_directly() {
    let json: cesium_scene::gltf_loader::GltfJson =
        serde_json::from_value(gltf2_json()).unwrap();

    let mut loader = GltfJsonLoader::new();
    loader.load_from_gltf_json(json).unwrap();

    assert_eq!(loader.state(), GltfJsonLoaderState::Ready);
    assert!(loader.gltf().is_some());
}

// it("loads glTF 2.0 from a JSON string")
#[test]
fn loads_from_json_string() {
    let json_string = serde_json::to_string(&gltf2_json()).unwrap();

    let mut loader = GltfJsonLoader::new();
    loader.load_from_json_string(&json_string).unwrap();

    assert_eq!(loader.state(), GltfJsonLoaderState::Ready);
    assert_eq!(loader.gltf().unwrap().asset.version, "2.0");
}

// parseGlb.js: "File is not valid binary glTF"
#[test]
fn parse_glb_throws_for_invalid_magic() {
    let error = parse_glb(b"not a glb container at all").unwrap_err();
    assert_eq!(error.message.as_str(), "File is not valid binary glTF");
}

// parseGlb.js: "Binary glTF version is not 1 or 2"
#[test]
fn parse_glb_throws_for_unsupported_version() {
    let mut glb = create_glb2(&gltf2_json(), None);
    glb[4..8].copy_from_slice(&3u32.to_le_bytes());
    let error = parse_glb(&glb).unwrap_err();
    assert_eq!(error.message.as_str(), "Binary glTF version is not 1 or 2");
}

// parseGlb.js v2: missing JSON chunk
#[test]
fn parse_glb_throws_when_json_chunk_missing() {
    // Header only, one BIN chunk, no JSON chunk.
    let mut glb = Vec::new();
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&20u32.to_le_bytes());
    glb.extend_from_slice(&0u32.to_le_bytes()); // chunk length 0
    glb.extend_from_slice(&0x004e4942u32.to_le_bytes()); // BIN type

    let error = parse_glb(&glb).unwrap_err();
    assert_eq!(error.message.as_str(), "Binary glTF JSON chunk is missing");
}

// GltfJsonLoader.js: unsupported version check
#[test]
fn throws_for_unsupported_gltf_version() {
    let json = serde_json::json!({ "asset": { "version": "3.0" }, "buffers": [] });

    let mut loader = GltfJsonLoader::new();
    let error = loader
        .load_from_typed_array(&generate_json_buffer(&json))
        .unwrap_err();
    assert_eq!(error.message.as_str(), "Unsupported glTF version: 3.0");
}

// Fixture-based: loads the BoxTextured.glb sample (mirrors the spec's use of
// Specs/Data/Models fixtures via `specs_data_root()`).
#[test]
fn loads_box_textured_glb_fixture() {
    let path = cesium_specs::data_path(
        "Models/glTF-2.0/BoxTextured/glTF-Binary/BoxTextured.glb",
    );
    assert!(path.exists(), "fixture missing: {}", path.display());
    let bytes = std::fs::read(&path).unwrap();

    let mut loader = GltfJsonLoader::new();
    loader.load_from_typed_array(&bytes).unwrap();

    let gltf = loader.gltf().unwrap();
    assert_eq!(gltf.asset.version, "2.0");
    assert_eq!(gltf.scenes.len(), 1);
    assert_eq!(gltf.meshes.len(), 1);
    assert!(!gltf.nodes.is_empty());
    assert!(!gltf.accessors.is_empty());
    assert!(!gltf.buffer_views.is_empty());
    // The embedded BIN chunk is attached to buffers[0].
    assert_eq!(gltf.buffers.len(), 1);
    assert!(gltf.buffers[0].data.is_some());
    assert_eq!(
        gltf.buffers[0].data.as_ref().unwrap().len(),
        gltf.buffers[0].byte_length as usize
    );
}

// Fixture-based: loads the Box.gltf JSON sample (embedded data-URI buffer).
#[test]
fn loads_box_gltf_fixture() {
    let path = cesium_specs::data_path("Models/glTF-2.0/Box/glTF/Box.gltf");
    assert!(path.exists(), "fixture missing: {}", path.display());
    let text = std::fs::read_to_string(&path).unwrap();

    let mut loader = GltfJsonLoader::new();
    loader.load_from_json_string(&text).unwrap();

    let gltf = loader.gltf().unwrap();
    assert_eq!(gltf.asset.version, "2.0");
    assert_eq!(gltf.meshes.len(), 1);
    assert_eq!(gltf.buffers.len(), 1);
    // The embedded data URI is decoded and removed, mirroring decodeDataUris.
    assert_eq!(gltf.buffers[0].uri, None);
    assert_eq!(
        gltf.buffers[0].data.as_ref().map(|d| d.len()),
        Some(gltf.buffers[0].byte_length as usize)
    );
    assert_eq!(gltf.buffers[0].byte_length, 648);
}

// GltfJsonLoader.js: cache key support
#[test]
fn stores_cache_key() {
    let loader = GltfJsonLoader::with_cache_key("example-cache-key".to_string());
    assert_eq!(loader.cache_key(), Some("example-cache-key"));
    assert_eq!(loader.state(), GltfJsonLoaderState::Unloaded);
}


