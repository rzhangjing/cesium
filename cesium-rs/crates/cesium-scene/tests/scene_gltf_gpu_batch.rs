//! Ported from the GPU-side branches of
//! `packages/engine/Specs/Scene/GltfVertexBufferLoaderSpec.js`,
//! `packages/engine/Specs/Scene/GltfIndexBufferLoaderSpec.js`,
//! `packages/engine/Specs/Scene/GltfImageLoaderSpec.js` and
//! `packages/engine/Specs/Scene/GltfTextureLoaderSpec.js`.
//!
//! Track A9-T5 batch 1: GPU buffer/texture products for the glTF resource
//! loaders (the CPU decode branches live in
//! `scene_gltf_buffer_batch.rs`).
//!
//! GPU-required tests skip gracefully (not fail) when no adapter exists,
//! mirroring the Track B smoke-test convention.

use std::io::Cursor;

use cesium_renderer::context::Context;
use cesium_scene::gltf_image_loader::{GltfImageLoader, GltfImageLoaderOptions};
use cesium_scene::gltf_index_buffer_loader::{
    GltfIndexBufferLoader, GltfIndexBufferLoaderOptions,
};
use cesium_scene::gltf_loader::{
    GltfAccessor, GltfBuffer, GltfBufferView, GltfImage, GltfJson, GltfSampler,
    GltfTexture,
};
use cesium_scene::gltf_pipeline::parse_glb::parse_glb;
use cesium_scene::gltf_texture_loader::{GltfTextureLoader, GltfTextureLoaderOptions};
use cesium_scene::gltf_vertex_buffer_loader::{
    GltfVertexBufferLoader, GltfVertexBufferLoaderOptions,
};
use cesium_scene::resource_loader_state::ResourceLoaderState;

// ---------------------------------------------------------------------------
// GPU acquisition (mirrors the Track B smoke-test convention)
// ---------------------------------------------------------------------------

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn try_gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
    )
    .ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("scene_gltf_gpu_batch"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .ok()?;
    Some(Gpu { device, queue })
}

fn try_context() -> Option<Context> {
    let gpu = try_gpu()?;
    Some(Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None))
}

// ---------------------------------------------------------------------------
// Fixtures (mirroring the JS spec fixtures)
// ---------------------------------------------------------------------------

/// `bufferTypedArray = new Uint8Array([1, 3, 7, 15, 31, 63, 127, 255])`
fn buffer_typed_array() -> Vec<u8> {
    vec![1, 3, 7, 15, 31, 63, 127, 255]
}

/// Embedded-buffer glTF with one vertex buffer view (`byteOffset: 2`,
/// `byteLength: 3`), mirroring the GltfVertexBufferLoaderSpec fixture.
fn gltf_vertex_embedded() -> GltfJson {
    GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: 8,
            data: Some(buffer_typed_array()),
            ..Default::default()
        }],
        buffer_views: vec![GltfBufferView {
            buffer: 0,
            byte_offset: 2,
            byte_length: 3,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Embedded-buffer glTF with three u16 indices, mirroring the
/// GltfIndexBufferLoaderSpec `indicesUint16` fixture.
fn gltf_indices_u16_embedded() -> GltfJson {
    let mut data = Vec::new();
    for value in [0_u16, 1, 2] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: data.len() as u32,
            data: Some(data),
            ..Default::default()
        }],
        buffer_views: vec![GltfBufferView {
            buffer: 0,
            byte_offset: 0,
            byte_length: 6,
            ..Default::default()
        }],
        accessors: vec![GltfAccessor {
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: 5123,
            count: 3,
            gl_type: "SCALAR".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Embedded-buffer glTF with three u8 indices, mirroring the
/// GltfIndexBufferLoaderSpec `indicesUint8` fixture.
fn gltf_indices_u8_embedded() -> GltfJson {
    GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: 3,
            data: Some(vec![0, 1, 2]),
            ..Default::default()
        }],
        buffer_views: vec![GltfBufferView {
            buffer: 0,
            byte_offset: 0,
            byte_length: 3,
            ..Default::default()
        }],
        accessors: vec![GltfAccessor {
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: 5121,
            count: 3,
            gl_type: "SCALAR".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Encodes a 2×2 red RGBA PNG (the GltfImageLoaderSpec "loads an image"
/// fixture uses a small generated image).
fn red_png_bytes() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
    let mut cursor = Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .expect("png encode");
    cursor.into_inner()
}

fn minimal_base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let accumulator = (b0 << 16) | (b1 << 8) | b2;
        output.push(ALPHABET[(accumulator >> 18) as usize & 63] as char);
        output.push(ALPHABET[(accumulator >> 12) as usize & 63] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(accumulator >> 6) as usize & 63] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[accumulator as usize & 63] as char
        } else {
            '='
        });
    }
    output
}

// ---------------------------------------------------------------------------
// GltfVertexBufferLoader GPU path (JS: `loads a vertex buffer` loadBuffer
// branch — the buffer spy becomes a size/format assertion on the wgpu
// buffer)
// ---------------------------------------------------------------------------

/// GPU-required: unlocks when a wgpu adapter is present (skips otherwise).
#[test]
fn vertex_buffer_loader_creates_gpu_buffer() {
    let Some(context) = try_context() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let gltf = gltf_vertex_embedded();
    let mut loader = GltfVertexBufferLoader::try_new(GltfVertexBufferLoaderOptions {
        buffer_view_id: Some(0),
        primitive: None,
        draco: None,
        spz: None,
        attribute_semantic: None,
        accessor_id: None,
        cache_key: None,
        load_buffer: true,
        load_typed_array: false,
    })
    .unwrap();
    loader.load(&gltf).unwrap();
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
    // loadBuffer without loadTypedArray drops the CPU copy (JS process()).
    assert!(loader.typed_array().is_none());

    loader.create_buffer(&context).unwrap();
    let buffer = loader.buffer().expect("gpu buffer created");
    assert_eq!(buffer.size_in_bytes(), 3);

    // A second upload has nothing pending (the GPU buffer keeps the data).
    let error = loader.create_buffer(&context).unwrap_err();
    assert!(error.message.contains("No buffer data pending upload"));
}

#[test]
fn vertex_buffer_loader_create_buffer_fails_without_load_buffer() {
    let Some(context) = try_context() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let gltf = gltf_vertex_embedded();
    let mut loader = GltfVertexBufferLoader::try_new(GltfVertexBufferLoaderOptions {
        buffer_view_id: Some(0),
        primitive: None,
        draco: None,
        spz: None,
        attribute_semantic: None,
        accessor_id: None,
        cache_key: None,
        load_buffer: false,
        load_typed_array: true,
    })
    .unwrap();
    loader.load(&gltf).unwrap();
    let error = loader.create_buffer(&context).unwrap_err();
    assert!(error.message.contains("No buffer data pending upload"));
}

// ---------------------------------------------------------------------------
// GltfIndexBufferLoader GPU path
// ---------------------------------------------------------------------------

/// GPU-required: unlocks when a wgpu adapter is present (skips otherwise).
#[test]
fn index_buffer_loader_creates_gpu_buffer_u16() {
    let Some(context) = try_context() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let gltf = gltf_indices_u16_embedded();
    let mut loader = GltfIndexBufferLoader::try_new(
        &gltf,
        GltfIndexBufferLoaderOptions {
            accessor_id: 0,
            draco: None,
            cache_key: None,
            load_buffer: true,
            load_typed_array: false,
        },
    )
    .unwrap();
    loader.load(&gltf).unwrap();
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
    assert!(loader.typed_array().is_none());

    loader.create_buffer(&context).unwrap();
    let index_buffer = loader.buffer().expect("gpu index buffer created");
    assert_eq!(index_buffer.number_of_indices(), 3);
    assert_eq!(index_buffer.bytes_per_index(), 2);
    assert_eq!(index_buffer.index_format(), wgpu::IndexFormat::Uint16);
}

/// GPU-required: documents the u8→u16 widening deviation (wgpu has no
/// Uint8 index format); unlocks when a wgpu adapter is present.
#[test]
fn index_buffer_loader_widens_u8_indices() {
    let Some(context) = try_context() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let gltf = gltf_indices_u8_embedded();
    let mut loader = GltfIndexBufferLoader::try_new(
        &gltf,
        GltfIndexBufferLoaderOptions {
            accessor_id: 0,
            draco: None,
            cache_key: None,
            load_buffer: true,
            load_typed_array: false,
        },
    )
    .unwrap();
    loader.load(&gltf).unwrap();
    loader.create_buffer(&context).unwrap();
    let index_buffer = loader.buffer().expect("gpu index buffer created");
    assert_eq!(index_buffer.number_of_indices(), 3);
    // DEVIATION: widened to 16-bit for wgpu.
    assert_eq!(index_buffer.bytes_per_index(), 2);
    assert_eq!(index_buffer.buffer().size_in_bytes(), 6);
}

// ---------------------------------------------------------------------------
// GltfImageLoader (JS: `loads an image from a bufferView` / data URI /
// external URI branches)
// ---------------------------------------------------------------------------

#[test]
fn image_loader_decodes_embedded_buffer_view_png() {
    let png = red_png_bytes();
    let png_len = png.len() as u32;
    let gltf = GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: png_len,
            data: Some(png),
            ..Default::default()
        }],
        buffer_views: vec![GltfBufferView {
            buffer: 0,
            byte_offset: 0,
            byte_length: png_len,
            ..Default::default()
        }],
        images: vec![GltfImage {
            buffer_view: Some(0),
            mime_type: Some("image/png".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut loader = GltfImageLoader::try_new(
        &gltf,
        GltfImageLoaderOptions { image_id: 0, cache_key: None },
    )
    .unwrap();
    loader.load(&gltf).unwrap();
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
    let image = loader.image().expect("decoded image");
    assert_eq!(image.width, 2);
    assert_eq!(image.height, 2);
    assert_eq!(image.pixels.len(), 2 * 2 * 4);
    // Every pixel is the red fixture.
    for pixel in image.pixels.chunks_exact(4) {
        assert_eq!(pixel, [255, 0, 0, 255]);
    }
}

#[test]
fn image_loader_decodes_data_uri() {
    let png = red_png_bytes();
    let data_uri = format!("data:image/png;base64,{}", minimal_base64(&png));
    let gltf = GltfJson {
        images: vec![GltfImage {
            uri: Some(data_uri),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut loader = GltfImageLoader::try_new(
        &gltf,
        GltfImageLoaderOptions { image_id: 0, cache_key: None },
    )
    .unwrap();
    loader.load(&gltf).unwrap();
    let image = loader.image().expect("decoded image");
    assert_eq!((image.width, image.height), (2, 2));
}

/// JS: external URIs resolve through the ResourceCache fetch; the Rust
/// port defers fetching (load_external supplies the bytes instead).
#[test]
fn image_loader_external_uri_is_deferred() {
    let gltf = GltfJson {
        images: vec![GltfImage {
            uri: Some("texture.png".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut loader = GltfImageLoader::try_new(
        &gltf,
        GltfImageLoaderOptions { image_id: 0, cache_key: None },
    )
    .unwrap();
    let error = loader.load(&gltf).unwrap_err();
    assert!(error.message.contains("External image URIs"));
    assert_eq!(loader.state(), ResourceLoaderState::Failed);
}

#[test]
fn image_loader_rejects_out_of_range_image_id() {
    let gltf = GltfJson::default();
    let error = GltfImageLoader::try_new(
        &gltf,
        GltfImageLoaderOptions { image_id: 0, cache_key: None },
    )
    .err()
    .expect("expected an out-of-range error");
    assert!(error.message.contains("imageId 0 is out of range"));
}

// ---------------------------------------------------------------------------
// GltfTextureLoader (JS: `loads a texture` → createTexture(context))
// ---------------------------------------------------------------------------

/// GPU-required: loads the BoxTextured.glb fixture end to end
/// (parseGlb → image decode → GPU texture upload); unlocks when a wgpu
/// adapter is present.
#[test]
fn texture_loader_creates_gpu_texture_from_box_textured() {
    let Some(context) = try_context() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let path = cesium_specs::data_path(
        "Models/glTF-2.0/BoxTextured/glTF-Binary/BoxTextured.glb",
    );
    assert!(path.exists(), "fixture missing: {}", path.display());
    let glb = std::fs::read(&path).unwrap();
    let gltf = parse_glb(&glb).unwrap();
    assert!(!gltf.textures.is_empty());

    let mut loader = GltfTextureLoader::try_new(
        &gltf,
        GltfTextureLoaderOptions { texture_id: 0, cache_key: None },
    )
    .unwrap();
    loader.load(&gltf).unwrap();
    assert_eq!(loader.state(), ResourceLoaderState::Ready);
    loader.create_texture(&context, &gltf).unwrap();
    let texture = loader.texture().expect("gpu texture created");
    assert_eq!(texture.width(), 256);
    assert_eq!(texture.height(), 256);
}

#[test]
fn texture_loader_rejects_texture_without_source() {
    let gltf = GltfJson {
        textures: vec![GltfTexture {
            sampler: Some(0),
            source: None,
            ..Default::default()
        }],
        samplers: vec![GltfSampler::default()],
        ..Default::default()
    };
    let error = GltfTextureLoader::try_new(
        &gltf,
        GltfTextureLoaderOptions { texture_id: 0, cache_key: None },
    )
    .err()
    .expect("expected a missing-source error");
    assert!(error.message.contains("has no source image"));
}

/// GPU-required: embedded PNG through the texture loader (mirrors the
/// synthetic-fixture branch of GltfTextureLoaderSpec); unlocks when a
/// wgpu adapter is present.
#[test]
fn texture_loader_uploads_embedded_png() {
    let Some(context) = try_context() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let png = red_png_bytes();
    let png_len = png.len() as u32;
    let gltf = GltfJson {
        buffers: vec![GltfBuffer {
            byte_length: png_len,
            data: Some(png),
            ..Default::default()
        }],
        buffer_views: vec![GltfBufferView {
            buffer: 0,
            byte_offset: 0,
            byte_length: png_len,
            ..Default::default()
        }],
        samplers: vec![GltfSampler {
            wrap_s: 10497,
            wrap_t: 33071,
            ..Default::default()
        }],
        images: vec![GltfImage {
            buffer_view: Some(0),
            mime_type: Some("image/png".to_string()),
            ..Default::default()
        }],
        textures: vec![GltfTexture {
            sampler: Some(0),
            source: Some(0),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut loader = GltfTextureLoader::try_new(
        &gltf,
        GltfTextureLoaderOptions { texture_id: 0, cache_key: None },
    )
    .unwrap();
    loader.load(&gltf).unwrap();
    loader.create_texture(&context, &gltf).unwrap();
    let texture = loader.texture().expect("gpu texture created");
    assert_eq!(texture.width(), 2);
    assert_eq!(texture.height(), 2);
}
