//! Ported from the GPU-facing branches of
//! `packages/engine/Specs/Scene/ModelSpec.js` and
//! `packages/engine/Specs/Scene/ModelSceneGraphSpec.js`.
//!
//! Track A9-T5 batch 2: Model render materialization — glTF primitives →
//! GPU vertex arrays / textures → per-primitive [`DrawCommand`]s through
//! the model WGSL pairs (color / textured), with the scene-graph node
//! world transforms folded into the per-draw model matrix.
//!
//! GPU-required tests skip gracefully (not fail) when no adapter exists,
//! mirroring the Track B smoke-test convention.

use std::sync::Arc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::julian_date::JulianDate;
use cesium_core::pixel_format::PixelFormat;
use cesium_renderer::context::{Context, DefaultRenderTarget};
use cesium_renderer::texture::{Texture, TextureOptions};
use cesium_scene::frame_state::FrameState;
use cesium_scene::gltf_loader::{GltfJson, GltfNode};
use cesium_scene::gltf_pipeline::parse_glb::parse_glb;
use cesium_scene::model::model::Model;
use cesium_scene::scene::Scene;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
/// `copy_texture_to_buffer` requires bytes_per_row to be a multiple of 256.
const PADDED_BYTES_PER_ROW: u32 = 1024;

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
        label: Some("scene_model_gpu_batch"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .ok()?;
    Some(Gpu { device, queue })
}

/// Copies the whole target texture back to CPU (row-major RGBA, unpadded).
fn read_pixels(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("model smoke readback"),
        size: (PADDED_BYTES_PER_ROW * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("model smoke readback encoder"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PADDED_BYTES_PER_ROW),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let mapped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mapped_flag = mapped.clone();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        result.expect("readback mapping failed");
        mapped_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    let mut attempts = 0u32;
    while !mapped.load(std::sync::atomic::Ordering::SeqCst) {
        attempts += 1;
        assert!(attempts < 100, "readback mapping never completed");
        let _ = gpu.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        });
    }
    let data = slice.get_mapped_range().expect("mapped range unavailable");
    let mut pixels = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for row in 0..HEIGHT as usize {
        let src = row * PADDED_BYTES_PER_ROW as usize;
        let dst = row * WIDTH as usize * 4;
        pixels[dst..dst + WIDTH as usize * 4]
            .copy_from_slice(&data[src..src + WIDTH as usize * 4]);
    }
    drop(data);
    readback.unmap();
    pixels
}

fn is_background(pixel: &[u8]) -> bool {
    pixel[0] < 40 && pixel[1] < 40 && pixel[2] < 90
}

/// Loads and parses the BoxTextured.glb fixture.
fn box_textured_gltf() -> GltfJson {
    let path = cesium_specs::data_path(
        "Models/glTF-2.0/BoxTextured/glTF-Binary/BoxTextured.glb",
    );
    let glb = std::fs::read(path).expect("BoxTextured.glb fixture missing");
    parse_glb(&glb).expect("BoxTextured.glb must parse")
}

// ---------------------------------------------------------------------------
// Pure-logic specs (ModelSpec.js / ModelSceneGraphSpec.js mirrors)
// ---------------------------------------------------------------------------

/// Mirrors `ModelSpec.js` `it("gets default values")`.
#[test]
fn model_default_values() {
    let model = Model::new();
    assert!(model.show);
    assert_eq!(model.scale, 1.0);
    assert_eq!(model.minimum_pixel_size, 0.0);
    assert!(model.maximum_scale.is_none());
    assert!(model.back_face_culling);
    assert!(!model.ready);
    assert!(!model.is_destroyed());
}

/// Mirrors `ModelSpec.js` `it("destroys")`.
#[test]
fn model_destroys() {
    let mut model = Model::new();
    assert!(!model.is_destroyed());
    model.destroy();
    assert!(model.is_destroyed());
}

/// Mirrors the fromGltf scene-graph construction contract: one runtime
/// node per glTF node, default-scene roots installed, ready deferred to
/// the first GPU update.
#[test]
fn from_gltf_builds_scene_graph_from_box_textured() {
    let gltf = box_textured_gltf();
    let node_count = gltf.nodes.len();
    let root_count = gltf.scenes[gltf.scene.unwrap() as usize].nodes.len();
    let model = Model::from_gltf(gltf);

    assert_eq!(model.scene_graph().nodes_count(), node_count);
    assert_eq!(model.scene_graph().root_nodes().len(), root_count);
    assert!(!model.ready, "ready must wait for the first GPU update");
    assert!(model.runtime_primitives().is_empty());
}

/// Mirrors the glTF node transform contract: an explicit `node.matrix`
/// wins over TRS, and TRS composes translation × rotation × scale.
#[test]
fn node_local_transform_matrix_and_trs() {
    let mut explicit = [0.0f64; 16];
    explicit[0] = 2.0;
    explicit[5] = 2.0;
    explicit[10] = 2.0;
    explicit[15] = 1.0;
    explicit[12] = 7.0;
    let mut gltf = GltfJson {
        scene: Some(0),
        scenes: vec![Default::default()],
        nodes: vec![
            GltfNode {
                name: Some("explicit".to_string()),
                matrix: Some(explicit),
                // Conflicting TRS must be ignored when matrix is present.
                translation: Some([100.0, 0.0, 0.0]),
                ..Default::default()
            },
            GltfNode {
                name: Some("trs".to_string()),
                translation: Some([1.0, 2.0, 3.0]),
                scale: Some([2.0, 2.0, 2.0]),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    // Fix the scene roots (Default scene has none).
    gltf.scenes[0].nodes = vec![0, 1];

    let model = Model::from_gltf(gltf);
    let explicit_node = model.get_node("explicit").expect("node by name");
    assert_eq!(explicit_node.matrix.elements[0], 2.0);
    assert_eq!(explicit_node.matrix.elements[12], 7.0);

    let trs_node = model.get_node("trs").expect("node by name");
    assert_eq!(trs_node.matrix.elements[0], 2.0);
    assert_eq!(trs_node.matrix.elements[5], 2.0);
    assert_eq!(trs_node.matrix.elements[10], 2.0);
    assert_eq!(trs_node.matrix.elements[12], 1.0);
    assert_eq!(trs_node.matrix.elements[13], 2.0);
    assert_eq!(trs_node.matrix.elements[14], 3.0);
}

/// glTF assets without a default scene: roots fall back to the nodes no
/// other node references as children.
#[test]
fn scene_roots_fallback_when_no_default_scene() {
    let gltf = GltfJson {
        nodes: vec![
            GltfNode {
                name: Some("parent".to_string()),
                children: vec![1],
                ..Default::default()
            },
            GltfNode {
                name: Some("child".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let model = Model::from_gltf(gltf);
    assert_eq!(model.scene_graph().root_nodes(), &[0usize][..]);
}

// ---------------------------------------------------------------------------
// GPU specs (skip gracefully when no adapter exists)
// ---------------------------------------------------------------------------

/// GPU-required: the first update with a context builds the runtime
/// primitive — one textured primitive for BoxTextured with the POSITION
/// min/max bounding sphere (unit cube: radius = √3/2).
#[test]
fn model_builds_textured_runtime_primitive() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let mut context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let mut model = Model::from_gltf(box_textured_gltf());
    assert!(!model.ready);

    model.update(&FrameState::new(), &mut context);

    assert!(model.ready, "the first GPU update must finish the model");
    assert_eq!(model.runtime_primitives().len(), 1);
    let primitive = &model.runtime_primitives()[0];
    assert!(
        primitive.is_textured(),
        "BoxTextured must take the textured shader path"
    );
    assert_eq!(primitive.count, 36, "unit cube: 12 triangles indexed");
    assert!(primitive.vertex_array.is_some());
    let radius = model.bounding_sphere.radius;
    assert!(
        (radius - (3.0_f64.sqrt() / 2.0)).abs() < 1e-6,
        "bounding sphere radius {radius} must be the unit-cube half diagonal"
    );
}

/// Builds the model smoke scene: BoxTextured at the origin, camera 3 m
/// away on +z looking at it, dark blue background.
fn build_scene() -> Scene {
    let mut scene = Scene::new();
    scene.set_background_color(Color::new(0.0, 0.0, 0.2, 1.0));
    scene.viewport_quad_mut().show = false;

    let model = Model::from_gltf(box_textured_gltf());
    scene.primitives_mut().add(Box::new(model));

    scene.camera_mut().set_view(
        &Cartesian3::new(0.0, 0.0, 3.0),
        Some(&Cartesian3::new(0.0, 0.0, -1.0)),
        Some(&Cartesian3::new(0.0, 1.0, 0.0)),
        &Ellipsoid::WGS84,
    );
    scene
}

fn render_frames(gpu: &Gpu, scene: &mut Scene, frames: u32) -> Texture {
    let mut context = Context::new(
        gpu.device.clone(),
        gpu.queue.clone(),
        WIDTH,
        HEIGHT,
        None,
    );
    let target = Texture::new(
        &gpu.device,
        TextureOptions {
            width: Some(WIDTH),
            height: Some(HEIGHT),
            pixel_format: PixelFormat::Rgba,
            ..Default::default()
        },
    );
    for _ in 0..frames {
        let view = target.create_view();
        let render_target = DefaultRenderTarget {
            view: &view,
            format: target.wgpu_format(),
            width: WIDTH,
            height: HEIGHT,
        };
        let time = JulianDate::now();
        scene.render_with_context(&time, &mut context, Some(render_target));
    }
    target
}

/// GPU-required: the BoxTextured model produces visible non-background
/// pixels through the full scene render path (glTF → GPU buffers/texture
/// → model WGSL pair → readback).
#[test]
fn model_renders_box_textured_with_visible_pixels() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let mut scene = build_scene();

    let target = render_frames(&gpu, &mut scene, 2);
    let pixels = read_pixels(&gpu, target.wgpu_texture());

    let mut non_background = 0usize;
    for pixel in pixels.chunks_exact(4) {
        if !is_background(pixel) {
            non_background += 1;
        }
    }
    // The unit cube at 3 m covers a solid fraction of the 256² viewport;
    // anything below a few hundred pixels means the draw was culled or
    // the buffers were empty.
    assert!(
        non_background > 500,
        "the model must cover visible pixels, got {non_background}"
    );

    // Screenshot evidence (repo docs/), mirroring the viewer-demo capture.
    let screenshot_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/screenshots");
    std::fs::create_dir_all(&screenshot_dir).unwrap();
    let image = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels.clone()).unwrap();
    image.save(screenshot_dir.join("model_smoke.png")).unwrap();
}

/// GPU-required: pipeline-cache convergence for the model path — the
/// textured (+ blit) pipelines are created once and rendering more frames
/// must not grow the cache (no per-primitive/per-frame key explosion).
#[test]
fn model_pipeline_cache_converges_across_frames() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let mut scene = build_scene();
    let mut context = Context::new(
        gpu.device.clone(),
        gpu.queue.clone(),
        WIDTH,
        HEIGHT,
        None,
    );
    let target = Texture::new(
        &gpu.device,
        TextureOptions {
            width: Some(WIDTH),
            height: Some(HEIGHT),
            pixel_format: PixelFormat::Rgba,
            ..Default::default()
        },
    );
    let render_one = |context: &mut Context, scene: &mut Scene, target: &Texture| {
        let view = target.create_view();
        let render_target = DefaultRenderTarget {
            view: &view,
            format: target.wgpu_format(),
            width: WIDTH,
            height: HEIGHT,
        };
        let time = JulianDate::now();
        scene.render_with_context(&time, context, Some(render_target));
    };

    for _ in 0..3 {
        render_one(&mut context, &mut scene, &target);
    }
    let settled = context.pipeline_cache_size();
    assert!(settled > 0, "model + blit pipelines must exist");
    for _ in 0..8 {
        render_one(&mut context, &mut scene, &target);
    }
    let after = context.pipeline_cache_size();
    assert_eq!(
        settled, after,
        "pipeline cache must not grow across frames (key explosion guard)"
    );
    assert!(
        after <= 8,
        "pipeline count {after} unexpectedly high for the model path"
    );
}

/// GPU-required: hiding the model suppresses its draw (the frame keeps the
/// background clear), mirroring the JS `primitive.show` contract.
#[test]
fn model_show_false_suppresses_draw() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let mut scene = build_scene();
    scene.primitives_mut().get_mut(0).unwrap().set_show(false);

    let target = render_frames(&gpu, &mut scene, 1);
    let pixels = read_pixels(&gpu, target.wgpu_texture());
    let non_background = pixels
        .chunks_exact(4)
        .filter(|pixel| !is_background(pixel))
        .count();
    assert_eq!(non_background, 0, "a hidden model must not draw");
}
