//! Ported from `packages/engine/Specs/Scene/GltfBufferViewLoaderSpec.js`,
//! `packages/engine/Specs/Scene/GltfIndexBufferLoaderSpec.js` and
//! `packages/engine/Specs/Scene/GltfVertexBufferLoaderSpec.js`.
//!
//! Batch of CPU-side tests for the glTF buffer view / index buffer /
//! vertex buffer loaders.
//!
//! DEVIATION: the JS specs exercise the asynchronous `ResourceCache`
//! pipeline (network fetch, GPU buffer creation, JobScheduler, Draco/SPZ/
//! meshopt decoding). The Rust port is synchronous and CPU-only, so the
//! async / GPU / compressed-codec cases are mirrored as deferred-error
//! assertions and the fetch spy is replaced by `load_external(bytes)`.

use cesium_core::runtime_error::RuntimeError;
use cesium_scene::gltf_buffer_view_loader::GltfBufferViewLoader;
use cesium_scene::gltf_index_buffer_loader::{
    GltfIndexBufferLoader, GltfIndexBufferLoaderOptions, IndicesTypedArray,
};
use cesium_scene::gltf_loader::{GltfAccessor, GltfBuffer, GltfBufferView, GltfJson};
use cesium_scene::gltf_vertex_buffer_loader::{
    GltfVertexBufferLoader, GltfVertexBufferLoaderOptions,
};
use cesium_scene::resource_loader_state::ResourceLoaderState;
use serde_json::json;

// ---------------------------------------------------------------------------
// Fixtures (mirroring the JS spec fixtures)
// ---------------------------------------------------------------------------

/// `bufferTypedArray = new Uint8Array([1, 3, 7, 15, 31, 63, 127, 255])`
fn buffer_typed_array() -> Vec<u8> {
    vec![1, 3, 7, 15, 31, 63, 127, 255]
}

/// `gltfEmbedded` from GltfBufferViewLoaderSpec.js (with the embedded
/// buffer bytes attached, the Rust analogue of `extras._pipeline.source`).
fn gltf_embedded() -> GltfJson {
    GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: 8,
            data: Some(buffer_typed_array()),
            ..Default::default()
        }],
        buffer_views: vec![buffer_view(0, 2, 3)],
        ..Default::default()
    }
}

/// `gltfExternal` from GltfBufferViewLoaderSpec.js.
fn gltf_external() -> GltfJson {
    GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: 8,
            uri: Some("external.bin".to_string()),
            ..Default::default()
        }],
        buffer_views: vec![buffer_view(0, 2, 3)],
        ..Default::default()
    }
}

fn buffer_view(buffer: u32, byte_offset: u32, byte_length: u32) -> GltfBufferView {
    GltfBufferView {
        buffer,
        byte_offset,
        byte_length,
        ..Default::default()
    }
}

fn accessor(
    component_type: u32,
    count: u32,
    gl_type: &str,
    buffer_view: Option<u32>,
) -> GltfAccessor {
    GltfAccessor {
        buffer_view,
        byte_offset: 0,
        component_type,
        count,
        gl_type: gl_type.to_string(),
        ..Default::default()
    }
}

/// The concatenated fixture bytes of GltfIndexBufferLoaderSpec.js:
/// positions (Float32 x 9) + normals (Float32 x 9) + indicesUint32 +
/// indicesUint16 + indicesUint8.
fn uncompressed_buffer_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [-1.0_f32, -1.0, -1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [-1.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u32, 1, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[0_u8, 1, 2]);
    bytes
}

/// `gltfUncompressed` from GltfIndexBufferLoaderSpec.js. `data` attaches
/// the buffer bytes for the embedded path (JS specs fetch the external
/// buffer through a `fetchArrayBuffer` spy instead).
fn gltf_uncompressed(data: Option<Vec<u8>>) -> GltfJson {
    GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: 78,
            uri: if data.is_some() {
                None
            } else {
                Some("external.bin".to_string())
            },
            data,
            ..Default::default()
        }],
        buffer_views: vec![
            buffer_view(0, 0, 36),
            buffer_view(0, 36, 36),
            buffer_view(0, 72, 12),
            buffer_view(0, 84, 6),
            buffer_view(0, 90, 3),
        ],
        accessors: vec![
            accessor(5126, 3, "VEC3", Some(0)),
            accessor(5126, 3, "VEC3", Some(1)),
            accessor(5125, 3, "SCALAR", Some(2)), // UNSIGNED_INT
            accessor(5123, 3, "SCALAR", Some(3)), // UNSIGNED_SHORT
            accessor(5121, 3, "SCALAR", Some(4)), // UNSIGNED_BYTE
        ],
        ..Default::default()
    }
}

fn err_message<T>(result: Result<T, RuntimeError>) -> String {
    result.err().expect("expected an error").message
}

// ---------------------------------------------------------------------------
// GltfBufferViewLoader (GltfBufferViewLoaderSpec.js)
// ---------------------------------------------------------------------------

#[test]
fn buffer_view_throws_if_buffer_view_id_is_out_of_range() {
    let gltf = gltf_embedded();
    let message = err_message(GltfBufferViewLoader::try_new(&gltf, 1, None));
    assert_eq!(message, "bufferViewId 1 is out of range.");
}

#[test]
fn buffer_view_loads_for_embedded_buffer() {
    let gltf = gltf_embedded();
    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, Some("key".to_string())).unwrap();
    assert_eq!(loader.state(), ResourceLoaderState::Unloaded);

    loader.load(&gltf).unwrap();

    assert_eq!(loader.typed_array(), Some([7, 15, 31].as_slice()));
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
    assert_eq!(loader.cache_key(), Some("key"));
}

#[test]
fn buffer_view_load_throws_if_embedded_buffer_has_no_data() {
    let mut gltf = gltf_embedded();
    gltf.buffers[0].data = None;

    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();
    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load buffer view\nEmbedded buffer has no data."
    );
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

#[test]
fn buffer_view_load_throws_if_buffer_id_is_out_of_range() {
    let mut gltf = gltf_embedded();
    gltf.buffer_views[0].buffer = 1;

    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();
    let message = err_message(loader.load(&gltf));
    assert_eq!(message, "bufferId 1 is out of range.");
}

#[test]
fn buffer_view_load_defers_external_buffer_fetch() {
    let gltf = gltf_external();
    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();

    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load buffer view\nExternal buffer must be fetched by the caller: external.bin"
    );
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

#[test]
fn buffer_view_loads_for_external_buffer() {
    let gltf = gltf_external();
    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();

    loader.load_external(&buffer_typed_array()).unwrap();

    assert_eq!(loader.typed_array(), Some([7, 15, 31].as_slice()));
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
}

#[test]
fn buffer_view_load_throws_if_view_is_out_of_bounds() {
    let mut gltf = gltf_embedded();
    gltf.buffers[0].data = Some(vec![1, 3, 7, 15]);

    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();
    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load buffer view\nBuffer view is out of bounds."
    );
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

#[test]
fn buffer_view_unload_clears_typed_array() {
    let gltf = gltf_embedded();
    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();
    loader.load(&gltf).unwrap();
    assert!(loader.typed_array().is_some());

    loader.unload();

    assert!(loader.typed_array().is_none());
}

#[test]
fn buffer_view_parses_ext_meshopt_compression_fields() {
    let mut gltf = gltf_embedded();
    gltf.buffer_views[0].extensions = Some(json!({
        "EXT_meshopt_compression": {
            "buffer": 0,
            "byteOffset": 0,
            "byteLength": 124,
            "byteStride": 8,
            "mode": "ATTRIBUTES",
            "count": 24,
        },
    }));

    let loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();

    assert!(loader.has_meshopt());
    assert_eq!(loader.meshopt_byte_stride(), Some(8));
    assert_eq!(loader.meshopt_count(), Some(24));
    assert_eq!(loader.meshopt_mode(), Some("ATTRIBUTES"));
    assert_eq!(loader.meshopt_filter(), "NONE");
}

#[test]
fn buffer_view_prefers_khr_meshopt_over_ext() {
    let mut gltf = gltf_embedded();
    gltf.buffer_views[0].extensions = Some(json!({
        "KHR_meshopt_compression": {
            "buffer": 0,
            "byteOffset": 0,
            "byteLength": 29,
            "byteStride": 2,
            "mode": "TRIANGLES",
            "count": 36,
        },
        "EXT_meshopt_compression": {
            "buffer": 0,
            "byteOffset": 0,
            "byteLength": 124,
            "byteStride": 8,
            "mode": "ATTRIBUTES",
            "count": 24,
        },
    }));

    let loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();

    assert!(loader.has_meshopt());
    assert_eq!(loader.meshopt_count(), Some(36));
    assert_eq!(loader.meshopt_mode(), Some("TRIANGLES"));
}

#[test]
fn buffer_view_parses_meshopt_color_filter() {
    let mut gltf = gltf_embedded();
    gltf.buffer_views[0].extensions = Some(json!({
        "KHR_meshopt_compression": {
            "buffer": 0,
            "byteOffset": 0,
            "byteLength": 74,
            "byteStride": 4,
            "count": 24,
            "mode": "ATTRIBUTES",
            "filter": "COLOR",
        },
    }));

    let loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();

    assert_eq!(loader.meshopt_filter(), "COLOR");
}

#[test]
fn buffer_view_load_defers_meshopt_decoding() {
    let mut gltf = gltf_embedded();
    gltf.buffers[0].data = Some(vec![0; 124]);
    gltf.buffer_views[0].extensions = Some(json!({
        "EXT_meshopt_compression": {
            "buffer": 0,
            "byteOffset": 0,
            "byteLength": 124,
            "byteStride": 8,
            "mode": "ATTRIBUTES",
            "count": 24,
        },
    }));

    let mut loader = GltfBufferViewLoader::try_new(&gltf, 0, None).unwrap();
    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load buffer view\nmeshopt decoding is not supported yet."
    );
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

// ---------------------------------------------------------------------------
// GltfIndexBufferLoader (GltfIndexBufferLoaderSpec.js)
// ---------------------------------------------------------------------------

fn index_options(accessor_id: u32, load_buffer: bool, load_typed_array: bool) -> GltfIndexBufferLoaderOptions {
    GltfIndexBufferLoaderOptions {
        accessor_id,
        draco: None,
        cache_key: None,
        load_buffer,
        load_typed_array,
    }
}

#[test]
fn index_loader_throws_if_both_load_flags_are_false() {
    let gltf = gltf_uncompressed(None);
    let message = err_message(GltfIndexBufferLoader::try_new(
        &gltf,
        index_options(3, false, false),
    ));
    assert_eq!(
        message,
        "At least one of loadBuffer and loadTypedArray must be true."
    );
}

#[test]
fn index_loader_throws_if_accessor_id_is_out_of_range() {
    let gltf = gltf_uncompressed(None);
    let message = err_message(GltfIndexBufferLoader::try_new(
        &gltf,
        index_options(5, true, false),
    ));
    assert_eq!(message, "accessorId 5 is out of range.");
}

#[test]
fn index_loader_throws_if_index_datatype_is_invalid() {
    let gltf = gltf_uncompressed(None);
    // Accessor 0 is FLOAT (5126), not an index datatype.
    let message = err_message(GltfIndexBufferLoader::try_new(
        &gltf,
        index_options(0, true, false),
    ));
    assert_eq!(message, "Invalid index datatype: 5126");
}

#[test]
fn index_loader_loads_uint16_indices_as_typed_array() {
    let gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(3, false, true)).unwrap();

    loader.load(&gltf).unwrap();

    let typed_array = loader.typed_array().expect("typed array");
    assert_eq!(typed_array, &IndicesTypedArray::U16(vec![0, 1, 2]));
    assert_eq!(typed_array.byte_length(), 6);
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
}

#[test]
fn index_loader_loads_uint32_indices_as_typed_array() {
    let gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(2, false, true)).unwrap();

    loader.load(&gltf).unwrap();

    let typed_array = loader.typed_array().expect("typed array");
    assert_eq!(typed_array, &IndicesTypedArray::U32(vec![0, 1, 2]));
    assert_eq!(typed_array.byte_length(), 12);
}

#[test]
fn index_loader_loads_uint8_indices_as_typed_array() {
    let gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(4, false, true)).unwrap();

    loader.load(&gltf).unwrap();

    let typed_array = loader.typed_array().expect("typed array");
    assert_eq!(typed_array, &IndicesTypedArray::U8(vec![0, 1, 2]));
    assert_eq!(typed_array.byte_length(), 3);
}

#[test]
fn index_loader_drops_typed_array_when_only_load_buffer_is_requested() {
    let gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(3, true, false)).unwrap();

    loader.load(&gltf).unwrap();

    // DEVIATION: the GPU index buffer is deferred; mirrors the JS
    // `typedArray === undefined` assertion of "loads from accessor into
    // buffer".
    assert!(loader.typed_array().is_none());
    assert!(loader.load_buffer());
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
}

#[test]
fn index_loader_keeps_typed_array_when_both_flags_are_requested() {
    let gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(3, true, true)).unwrap();

    loader.load(&gltf).unwrap();

    assert_eq!(
        loader.typed_array(),
        Some(&IndicesTypedArray::U16(vec![0, 1, 2]))
    );
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
}

#[test]
fn index_loader_load_defers_draco_decoding() {
    let gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    let mut options = index_options(3, true, false);
    options.draco = Some(json!({
        "bufferView": 0,
        "attributes": { "POSITION": 0, "NORMAL": 1 },
    }));
    let mut loader = GltfIndexBufferLoader::try_new(&gltf, options).unwrap();

    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load index buffer\nDraco decoding is not supported yet."
    );
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

#[test]
fn index_loader_load_throws_if_accessor_has_no_buffer_view() {
    let mut gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    gltf.accessors[3].buffer_view = None;

    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(3, false, true)).unwrap();
    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load index buffer\nAccessor has no bufferView."
    );
}

#[test]
fn index_loader_loads_from_external_buffer_bytes() {
    let gltf = gltf_uncompressed(None);
    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(3, false, true)).unwrap();

    loader
        .load_external(&gltf, &uncompressed_buffer_bytes())
        .unwrap();

    assert_eq!(
        loader.typed_array(),
        Some(&IndicesTypedArray::U16(vec![0, 1, 2]))
    );
}

#[test]
fn index_loader_unload_clears_typed_array() {
    let gltf = gltf_uncompressed(Some(uncompressed_buffer_bytes()));
    let mut loader =
        GltfIndexBufferLoader::try_new(&gltf, index_options(3, false, true)).unwrap();
    loader.load(&gltf).unwrap();
    assert!(loader.typed_array().is_some());

    loader.unload();

    assert!(loader.typed_array().is_none());
}

// ---------------------------------------------------------------------------
// GltfVertexBufferLoader (GltfVertexBufferLoaderSpec.js)
// ---------------------------------------------------------------------------

fn vertex_options() -> GltfVertexBufferLoaderOptions {
    GltfVertexBufferLoaderOptions {
        buffer_view_id: None,
        primitive: None,
        draco: None,
        spz: None,
        attribute_semantic: None,
        accessor_id: None,
        cache_key: None,
        load_buffer: false,
        load_typed_array: false,
    }
}

fn draco_extension() -> serde_json::Value {
    json!({
        "bufferView": 0,
        "attributes": { "POSITION": 0, "NORMAL": 1 },
    })
}

#[test]
fn vertex_loader_throws_if_both_load_flags_are_false() {
    let mut options = vertex_options();
    options.buffer_view_id = Some(0);
    let message = err_message(GltfVertexBufferLoader::try_new(options));
    assert_eq!(
        message,
        "At least one of loadBuffer and loadTypedArray must be true."
    );
}

#[test]
fn vertex_loader_throws_if_buffer_view_id_and_draco_are_both_defined() {
    let mut options = vertex_options();
    options.buffer_view_id = Some(0);
    options.draco = Some(draco_extension());
    options.attribute_semantic = Some("POSITION".to_string());
    options.accessor_id = Some(0);
    options.primitive = Some(json!({}));
    options.load_typed_array = true;

    let message = err_message(GltfVertexBufferLoader::try_new(options));
    assert_eq!(
        message,
        "Exactly one vertex buffer source must be effective: options.bufferViewId, options.spz, or options.draco for options.attributeSemantic."
    );
}

#[test]
fn vertex_loader_throws_if_all_sources_are_undefined() {
    let mut options = vertex_options();
    options.load_typed_array = true;

    let message = err_message(GltfVertexBufferLoader::try_new(options));
    assert_eq!(
        message,
        "Exactly one vertex buffer source must be effective: options.bufferViewId, options.spz, or options.draco for options.attributeSemantic."
    );
}

#[test]
fn vertex_loader_throws_if_buffer_view_id_and_spz_are_both_defined() {
    let mut options = vertex_options();
    options.buffer_view_id = Some(0);
    options.spz = Some(json!({}));
    options.load_typed_array = true;

    let message = err_message(GltfVertexBufferLoader::try_new(options));
    assert_eq!(
        message,
        "Exactly one vertex buffer source must be effective: options.bufferViewId, options.spz, or options.draco for options.attributeSemantic."
    );
}

#[test]
fn vertex_loader_throws_if_draco_and_spz_are_both_defined() {
    let mut options = vertex_options();
    options.draco = Some(draco_extension());
    options.attribute_semantic = Some("POSITION".to_string());
    options.accessor_id = Some(0);
    options.primitive = Some(json!({}));
    options.spz = Some(json!({}));
    options.load_typed_array = true;

    let message = err_message(GltfVertexBufferLoader::try_new(options));
    assert_eq!(
        message,
        "Exactly one vertex buffer source must be effective: options.bufferViewId, options.spz, or options.draco for options.attributeSemantic."
    );
}

#[test]
fn vertex_loader_does_not_throw_if_spz_and_unrelated_draco_are_both_defined() {
    // The Draco extension does not define the NORMAL semantic, so only the
    // SPZ source is effective.
    let mut options = vertex_options();
    options.draco = Some(draco_extension());
    options.attribute_semantic = Some("NORMAL_OR_UNRELATED".to_string());
    options.spz = Some(json!({}));
    options.load_typed_array = true;

    assert!(GltfVertexBufferLoader::try_new(options).is_ok());
}

#[test]
fn vertex_loader_throws_if_buffer_view_id_draco_and_spz_are_defined() {
    let mut options = vertex_options();
    options.buffer_view_id = Some(0);
    options.draco = Some(draco_extension());
    options.attribute_semantic = Some("POSITION".to_string());
    options.accessor_id = Some(0);
    options.primitive = Some(json!({}));
    options.spz = Some(json!({}));
    options.load_typed_array = true;

    let message = err_message(GltfVertexBufferLoader::try_new(options));
    assert_eq!(
        message,
        "Exactly one vertex buffer source must be effective: options.bufferViewId, options.spz, or options.draco for options.attributeSemantic."
    );
}

/// `gltfUncompressed` from GltfVertexBufferLoaderSpec.js (3 buffer views).
fn gltf_uncompressed_vertex(data: Option<Vec<u8>>) -> GltfJson {
    GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: 78,
            uri: if data.is_some() {
                None
            } else {
                Some("external.bin".to_string())
            },
            data,
            ..Default::default()
        }],
        buffer_views: vec![
            buffer_view(0, 0, 36),
            buffer_view(0, 36, 36),
            buffer_view(0, 72, 6),
        ],
        accessors: vec![
            accessor(5126, 3, "VEC3", Some(0)),
            accessor(5126, 3, "VEC3", Some(1)),
            accessor(5123, 3, "SCALAR", Some(2)),
        ],
        ..Default::default()
    }
}

fn vertex_buffer_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [-1.0_f32, -1.0, -1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [-1.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[test]
fn vertex_loader_loads_from_buffer_view_as_typed_array() {
    let gltf = gltf_uncompressed_vertex(Some(vertex_buffer_bytes()));
    let mut options = vertex_options();
    options.buffer_view_id = Some(0);
    options.load_typed_array = true;
    let mut loader = GltfVertexBufferLoader::try_new(options).unwrap();

    loader.load(&gltf).unwrap();

    let expected: Vec<u8> = [-1.0_f32, -1.0, -1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    assert_eq!(loader.typed_array(), Some(expected.as_slice()));
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
}

#[test]
fn vertex_loader_drops_typed_array_when_only_load_buffer_is_requested() {
    let gltf = gltf_uncompressed_vertex(Some(vertex_buffer_bytes()));
    let mut options = vertex_options();
    options.buffer_view_id = Some(0);
    options.load_buffer = true;
    let mut loader = GltfVertexBufferLoader::try_new(options).unwrap();

    loader.load(&gltf).unwrap();

    assert!(loader.typed_array().is_none());
    assert!(loader.load_buffer());
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
}

#[test]
fn vertex_loader_load_defers_spz_decoding() {
    let gltf = gltf_uncompressed_vertex(Some(vertex_buffer_bytes()));
    let mut options = vertex_options();
    options.spz = Some(json!({}));
    options.load_typed_array = true;
    let mut loader = GltfVertexBufferLoader::try_new(options).unwrap();

    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load vertex buffer\nSPZ decoding is not supported yet."
    );
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

#[test]
fn vertex_loader_load_defers_draco_decoding() {
    let gltf = gltf_uncompressed_vertex(Some(vertex_buffer_bytes()));
    let mut options = vertex_options();
    options.draco = Some(draco_extension());
    options.attribute_semantic = Some("POSITION".to_string());
    options.accessor_id = Some(0);
    options.primitive = Some(json!({ "attributes": { "POSITION": 0 } }));
    options.load_typed_array = true;
    let mut loader = GltfVertexBufferLoader::try_new(options).unwrap();

    let message = err_message(loader.load(&gltf));
    assert_eq!(
        message,
        "Failed to load vertex buffer\nDraco decoding is not supported yet."
    );
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

#[test]
fn vertex_loader_loads_from_external_buffer_bytes() {
    let gltf = gltf_uncompressed_vertex(None);
    let mut options = vertex_options();
    options.buffer_view_id = Some(1);
    options.load_typed_array = true;
    let mut loader = GltfVertexBufferLoader::try_new(options).unwrap();

    loader.load_external(&gltf, &vertex_buffer_bytes()).unwrap();

    let expected: Vec<u8> = [-1.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    assert_eq!(loader.typed_array(), Some(expected.as_slice()));
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
}

#[test]
fn vertex_loader_unload_clears_typed_array() {
    let gltf = gltf_uncompressed_vertex(Some(vertex_buffer_bytes()));
    let mut options = vertex_options();
    options.buffer_view_id = Some(0);
    options.load_typed_array = true;
    let mut loader = GltfVertexBufferLoader::try_new(options).unwrap();
    loader.load(&gltf).unwrap();
    assert!(loader.typed_array().is_some());

    loader.unload();

    assert!(loader.typed_array().is_none());
}
