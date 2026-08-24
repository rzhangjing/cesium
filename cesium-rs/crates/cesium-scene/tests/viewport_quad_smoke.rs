//! B3 smoke: end-to-end wgpu render of the scene viewport quad.
//!
//! Renders the full-screen quad into GPU targets (off-screen framebuffer and
//! a simulated default target), reads pixels back, and asserts the material
//! color — the headless acceptance evidence for the ViewportQuad smoke
//! milestone. Skipped (not failed) when no GPU adapter is available.

use std::sync::Arc;

use cesium_core::julian_date::JulianDate;
use cesium_core::pixel_format::PixelFormat;
use cesium_renderer::clear_command::ClearCommand;
use cesium_renderer::context::{Context, DefaultRenderTarget};
use cesium_renderer::framebuffer::{Framebuffer, FramebufferOptions};
use cesium_renderer::texture::{Texture, TextureOptions};
use cesium_scene::frame_state::FrameState;
use cesium_scene::scene::Scene;
use cesium_scene::viewport_quad::ViewportQuad;

const WIDTH: u32 = 16;
const HEIGHT: u32 = 16;
/// `copy_texture_to_buffer` requires bytes_per_row to be a multiple of 256.
const PADDED_BYTES_PER_ROW: u32 = 256;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// Acquires a headless wgpu device; returns `None` when no adapter exists
/// (CI without GPU), in which case tests skip themselves.
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
        label: Some("viewport_quad_smoke"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .ok()?;
    Some(Gpu { device, queue })
}

fn create_color_texture(device: &wgpu::Device) -> Texture {
    Texture::new(
        device,
        TextureOptions {
            width: Some(WIDTH),
            height: Some(HEIGHT),
            pixel_format: PixelFormat::Rgba,
            ..Default::default()
        },
    )
}

/// Copies the target texture back to CPU and returns the center pixel RGBA.
fn read_center_pixel(gpu: &Gpu, texture: &wgpu::Texture) -> [u8; 4] {
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("smoke readback"),
        size: (PADDED_BYTES_PER_ROW * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("smoke readback encoder"),
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

    // wgpu 30 buffer mapping is callback-based: `map_async` registers the
    // callback and `Device::poll` (blocking with `PollType::Wait`) delivers
    // it once the copy completes.
    let slice = readback.slice(..);
    let mapped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
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
    let offset =
        (HEIGHT / 2) as usize * PADDED_BYTES_PER_ROW as usize + (WIDTH / 2) as usize * 4;
    let pixel = [data[offset], data[offset + 1], data[offset + 2], data[offset + 3]];
    drop(data);
    readback.unmap();
    pixel
}

/// Direct path: ViewportQuad → DrawCommand targeting an off-screen
/// framebuffer → Context frame orchestration → pixel readback.
#[test]
fn viewport_quad_renders_material_color_into_framebuffer() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };

    let mut context = Context::new(
        gpu.device.clone(),
        gpu.queue.clone(),
        WIDTH,
        HEIGHT,
        None,
    );

    let framebuffer = Arc::new(Framebuffer::new(FramebufferOptions {
        color_textures: Some(vec![Arc::new(create_color_texture(&gpu.device))]),
        ..Default::default()
    }));

    let mut quad = ViewportQuad::with_color([1.0, 0.0, 1.0, 1.0]); // magenta
    quad.framebuffer = Some(framebuffer.clone());

    let frame_state = FrameState::default();
    context.begin_frame();
    let mut clear = ClearCommand::all();
    clear.framebuffer = Some(framebuffer.clone());
    context.clear(clear);
    quad.update(&frame_state, &mut context);
    context.execute(None);
    context.end_frame();

    let pixel = read_center_pixel(
        &gpu,
        framebuffer.get_color_texture(0).unwrap().wgpu_texture(),
    );
    assert_eq!(pixel, [255, 0, 255, 255], "quad must cover the target with the material color");
}

/// Full scene path: Scene::render_with_context with a simulated default
/// (surface) target — exercises the background clear, command collection,
/// and the surface-format pipeline key fix.
#[test]
fn scene_render_with_context_covers_default_target() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };

    let mut context = Context::new(
        gpu.device.clone(),
        gpu.queue.clone(),
        WIDTH,
        HEIGHT,
        None,
    );
    let mut scene = Scene::new();
    // Red quad over the black background (Scene::new defaults).
    scene
        .viewport_quad_mut()
        .color = [1.0, 0.0, 0.0, 1.0];

    let target_texture = create_color_texture(&gpu.device);
    let view = target_texture.create_view();
    let target = DefaultRenderTarget {
        view: &view,
        format: target_texture.wgpu_format(),
        width: WIDTH,
        height: HEIGHT,
    };

    let time = JulianDate::now();
    scene.render_with_context(&time, &mut context, Some(target));

    let pixel = read_center_pixel(&gpu, target_texture.wgpu_texture());
    assert_eq!(pixel, [255, 0, 0, 255], "scene quad must cover the default target");
}
