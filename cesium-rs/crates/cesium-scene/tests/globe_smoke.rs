//! Headless acceptance tests for the globe rendering path (Track B4-3/B4-4).
//!
//! Renders the ellipsoid globe with an offline marker imagery pyramid
//! (north polar cap red / south polar cap blue / mid-latitudes green-white
//! checker) and asserts:
//! - the UV v-flip orientation (red on top, blue at the bottom),
//! - the LOD invariants: maximum_screen_space_error = 2.0, the single-tile
//!   screen-pixel cap (288 px lesson), no ancestor/descendant overlap in the
//!   rendered tile set, and deep refinement near the camera,
//! - pipeline-cache convergence across frames (no key explosion).
//!
//! Skipped (not failed) when no GPU adapter is available.

use std::sync::Arc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::julian_date::JulianDate;
use cesium_core::pixel_format::PixelFormat;
use cesium_renderer::context::{Context, DefaultRenderTarget};
use cesium_renderer::texture::{Texture, TextureOptions};
use cesium_scene::file_imagery_provider::FileImageryProvider;
use cesium_scene::globe::Globe;
use cesium_scene::imagery_layer::ImageryLayer;
use cesium_scene::quadtree_primitive::DEFAULT_TILE_IMAGE_WIDTH;
use cesium_scene::scene::Scene;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
/// `copy_texture_to_buffer` requires bytes_per_row to be a multiple of 256.
const PADDED_BYTES_PER_ROW: u32 = 1024;
/// The cesiumrust historical single-tile screen-pixel cap lesson.
const SINGLE_TILE_PIXEL_CAP: f64 = 288.0;
const MAXIMUM_LEVEL: u32 = 3;

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
        label: Some("globe_smoke"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .ok()?;
    Some(Gpu { device, queue })
}

fn create_target_texture(device: &wgpu::Device) -> Texture {
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

/// Copies the whole target texture back to CPU (row-major RGBA, unpadded).
fn read_pixels(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("globe smoke readback"),
        size: (PADDED_BYTES_PER_ROW * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("globe smoke readback encoder"),
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

/// Generates the offline marker pyramid (geographic 2×1 root, y = 0 north):
/// red above +45°, blue below −45°, green/white checker in between.
fn generate_marker_tiles(root: &std::path::Path) {
    use std::f64::consts::{FRAC_PI_2, PI};
    const SIZE: u32 = 256;
    for level in 0..=MAXIMUM_LEVEL {
        let columns = 2u32 << level;
        let rows = 1u32 << level;
        for x in 0..columns {
            for y in 0..rows {
                let north = FRAC_PI_2 - (y as f64) * PI / (rows as f64);
                let lat_step = -PI / (rows as f64) / (SIZE as f64);
                let mut image = image::RgbaImage::new(SIZE, SIZE);
                for py in 0..SIZE {
                    let latitude = north + (py as f64 + 0.5) * lat_step;
                    for px in 0..SIZE {
                        let color = if latitude > PI / 4.0 {
                            [255, 24, 24, 255]
                        } else if latitude < -PI / 4.0 {
                            [24, 64, 255, 255]
                        } else {
                            let checker = ((px / 32) + (py / 32)) % 2 == 0;
                            if checker {
                                [64, 200, 64, 255]
                            } else {
                                [232, 232, 224, 255]
                            }
                        };
                        image.put_pixel(px, py, image::Rgba(color));
                    }
                }
                let path = root.join(format!("{level}/{x}/{y}.png"));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                image.save(&path).unwrap();
            }
        }
    }
}

/// Builds the globe scene: marker imagery layer + straight-down camera over
/// the equator/prime meridian at three ellipsoid radii.
fn build_scene(imagery_root: &std::path::Path) -> Scene {
    let mut scene = Scene::new();
    scene.set_background_color(cesium_core::color::Color::new(0.0, 0.0, 0.2, 1.0));
    scene.viewport_quad_mut().show = false;

    let provider = FileImageryProvider::new(imagery_root, None);
    let mut globe = Globe::new(Some(Ellipsoid::WGS84));
    globe
        .imagery_layers_mut()
        .add(ImageryLayer::with_provider(Box::new(provider)));
    scene.set_globe(Some(globe));

    let destination = Cartesian3::new(Ellipsoid::WGS84.maximum_radius() * 3.0, 0.0, 0.0);
    scene
        .camera_mut()
        .set_view(&destination, None, None, &Ellipsoid::WGS84);
    scene
}

/// Renders `frames` frames into a fresh target texture of `size` and
/// returns it.
fn render_frames(gpu: &Gpu, scene: &mut Scene, frames: u32, size: u32) -> Texture {
    let mut context = Context::new(
        gpu.device.clone(),
        gpu.queue.clone(),
        size,
        size,
        None,
    );
    let target = Texture::new(
        &gpu.device,
        TextureOptions {
            width: Some(size),
            height: Some(size),
            pixel_format: PixelFormat::Rgba,
            ..Default::default()
        },
    );
    for _ in 0..frames {
        let view = target.create_view();
        let render_target = DefaultRenderTarget {
            view: &view,
            format: target.wgpu_format(),
            width: size,
            height: size,
        };
        let time = JulianDate::now();
        scene.render_with_context(&time, &mut context, Some(render_target));
    }
    // Keep the context alive until the queue drains, then drop it.
    target
}

fn is_background(pixel: &[u8]) -> bool {
    pixel[0] < 40 && pixel[1] < 40 && pixel[2] < 90
}
fn is_red(pixel: &[u8]) -> bool {
    pixel[0] > 180 && pixel[1] < 120 && pixel[2] < 120
}
fn is_blue(pixel: &[u8]) -> bool {
    pixel[2] > 180 && pixel[0] < 120
}
fn is_checker(pixel: &[u8]) -> bool {
    (pixel[1] > 150 && pixel[0] < 200) || (pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 150)
}

/// UV v-flip acceptance (cesiumrust pitfall checkpoint): the marker imagery
/// must appear upright — red polar cap at the top of the globe disc, blue
/// at the bottom — proving the single 1.0-v decision point at imagery
/// upload compensates the wgpu top-left texture origin.
#[test]
fn globe_renders_imaged_ellipsoid_with_upright_uv_orientation() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let root = std::env::temp_dir().join("cesium_rs_globe_smoke_imagery");
    if !root.join("0").is_dir() {
        generate_marker_tiles(&root);
    }
    let mut scene = build_scene(&root);

    let target = render_frames(&gpu, &mut scene, 2, WIDTH);
    let pixels = read_pixels(&gpu, target.wgpu_texture());

    // Scan the middle column: first/last non-background pixels are the top
    // and bottom silhouette edges of the globe disc.
    let column = WIDTH as usize / 2;
    let mut top: Option<[u8; 4]> = None;
    let mut bottom: Option<[u8; 4]> = None;
    for row in 0..HEIGHT as usize {
        let offset = (row * WIDTH as usize + column) * 4;
        let pixel = [
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        ];
        if !is_background(&pixel) {
            if top.is_none() {
                top = Some(pixel);
            }
            bottom = Some(pixel);
        }
    }
    let top = top.expect("globe disc must be visible");
    let bottom = bottom.expect("globe disc must be visible");
    assert!(
        is_red(&top),
        "top of the globe must show the red north polar cap, got {top:?}"
    );
    assert!(
        is_blue(&bottom),
        "bottom of the globe must show the blue south polar cap, got {bottom:?}"
    );
    let center_offset = ((HEIGHT as usize / 2) * WIDTH as usize + column) * 4;
    let center = [
        pixels[center_offset],
        pixels[center_offset + 1],
        pixels[center_offset + 2],
        pixels[center_offset + 3],
    ];
    assert!(
        is_checker(&center),
        "equator must show the checker imagery, got {center:?}"
    );

    // Screenshot evidence (repo docs/), mirroring the viewer-demo capture.
    let screenshot_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/screenshots");
    std::fs::create_dir_all(&screenshot_dir).unwrap();
    let image = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels.clone()).unwrap();
    image.save(screenshot_dir.join("globe_smoke.png")).unwrap();
}

/// LOD invariants (cesiumrust pitfall checkpoints):
/// - maximum_screen_space_error = 2.0 drives refinement: every rendered tile
///   that could still refine has SSE ≤ 2.0;
/// - single-tile screen-pixel cap (288 lesson): the projected screen width
///   of every refinable rendered tile stays ≤ 288 px;
/// - no ancestor/descendant overlap in the rendered set (seam discipline);
/// - refinement actually reaches the imagery ceiling near the camera.
#[test]
fn quadtree_lod_invariants_sse_and_pixel_cap() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let root = std::env::temp_dir().join("cesium_rs_globe_smoke_imagery");
    if !root.join("0").is_dir() {
        generate_marker_tiles(&root);
    }
    let mut scene = build_scene(&root);
    // 1024px viewport: at 256px the root tiles already meet maxSSE 2.0 (no
    // refinement needed); the larger buffer makes the traversal subdivide.
    // Camera pulled in to 1.5R (build_scene parks it at 3R) so refinement is
    // forced all the way down to the imagery ceiling near the nadir point.
    let destination = Cartesian3::new(Ellipsoid::WGS84.maximum_radius() * 1.5, 0.0, 0.0);
    scene
        .camera_mut()
        .set_view(&destination, None, None, &Ellipsoid::WGS84);
    let _target = render_frames(&gpu, &mut scene, 1, 1024);

    let globe = scene.globe().expect("globe installed");
    let surface = globe.surface();
    let tiles = surface.tiles_to_render();
    assert!(!tiles.is_empty(), "traversal must select tiles");

    let sse_denominator = scene.frame_state().sse_denominator;
    let camera_position = scene.frame_state().camera_position;
    let maximum_level = surface.maximum_level().expect("imagery ceiling set");
    assert_eq!(maximum_level, MAXIMUM_LEVEL as i32);

    // Root tiles must NOT meet the SSE target (otherwise nothing refines).
    for root_tile in surface.root_tiles() {
        assert!(
            root_tile.screen_space_error > 2.0,
            "root tile SSE {} must exceed maxSSE 2.0 (camera at 3R)",
            root_tile.screen_space_error
        );
    }

    let rendered: std::collections::HashSet<(i32, i32, i32)> = tiles
        .iter()
        .map(|tile| (tile.level, tile.x, tile.y))
        .collect();
    let mut deepest = 0i32;
    for tile in tiles {
        deepest = deepest.max(tile.level);
        if tile.level >= maximum_level {
            continue; // cannot refine further: SSE may exceed the target
        }
        assert!(
            tile.screen_space_error <= 2.0 + 1e-6,
            "refinable tile ({}, {}, {}) rendered with SSE {} > 2.0",
            tile.level,
            tile.x,
            tile.y,
            tile.screen_space_error
        );
        // Single-tile screen-pixel cap (288 lesson) + SSE distance-floor
        // discipline. Re-derive the TRUE camera->bounding-sphere distance
        // independently instead of trusting the traversal's stored value: a
        // distance-floor bug inflates the apparent distance, keeps the
        // traversal SSE under maxSSE while undersplitting, and blows the
        // true screen footprint past the cap.
        let to_center = Cartesian3::subtract_new(
            &camera_position,
            &tile.bounding_sphere.center,
        );
        let true_distance = (Cartesian3::magnitude(&to_center)
            - tile.bounding_sphere.radius)
            .max(0.0);
        assert!(
            tile.camera_distance <= true_distance * (1.0 + 1e-9) + 1e-6,
            "tile ({}, {}, {}) stored distance {} exceeds the true minimum \
             {} - a floor above the camera's actual minimum distance is \
             forbidden",
            tile.level, tile.x, tile.y, tile.camera_distance, true_distance
        );
        let true_sse = if true_distance <= 0.0 {
            0.0 // camera inside the bounding volume: refinement forced
        } else {
            (tile.geometric_error * 1024.0) / (true_distance * sse_denominator)
        };
        assert!(
            true_sse <= 2.0 + 1e-6,
            "tile ({}, {}, {}) true SSE {} exceeds maxSSE 2.0 \
             (undersplitting at the true camera distance)",
            tile.level, tile.x, tile.y, true_sse
        );
        // Historical 288px cap, expressed in texel space: the heightmap
        // geometric-error estimate is inflated by 1/HEIGHTMAP_TERRAIN_QUALITY
        // (arc-width/error ratio 65/0.25 = 260), so normalize the true
        // screen width back to the quality-1 texel model (ratio 65) where
        // the cap was calibrated: width = true_sse * DEFAULT_TILE_IMAGE_WIDTH.
        let footprint = true_sse * DEFAULT_TILE_IMAGE_WIDTH as f64;
        assert!(
            footprint <= SINGLE_TILE_PIXEL_CAP,
            "tile ({}, {}, {}) footprint {:.1}px exceeds the {}px cap",
            tile.level,
            tile.x,
            tile.y,
            footprint,
            SINGLE_TILE_PIXEL_CAP
        );

        // No ancestor of a rendered tile may itself be rendered
        // (all-or-nothing replacement; prevents overdraw/seam artifacts).
        let (mut level, mut x, mut y) = (tile.level, tile.x, tile.y);
        while level > 0 {
            x >>= 1;
            y >>= 1;
            level -= 1;
            assert!(
                !rendered.contains(&(level, x, y)),
                "ancestor ({level}, {x}, {y}) of a rendered tile is also rendered"
            );
        }
    }
    assert_eq!(
        deepest, maximum_level,
        "refinement must reach the imagery ceiling near the camera"
    );
}

/// Pipeline-cache convergence (wgpu 30 frame-orchestration discipline): the
/// globe + blit pipelines are created once; rendering more frames must not
/// grow the cache (no per-tile/per-frame key explosion).
#[test]
fn pipeline_cache_converges_across_frames() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let root = std::env::temp_dir().join("cesium_rs_globe_smoke_imagery");
    if !root.join("0").is_dir() {
        generate_marker_tiles(&root);
    }
    let mut scene = build_scene(&root);

    let mut context = Context::new(
        gpu.device.clone(),
        gpu.queue.clone(),
        WIDTH,
        HEIGHT,
        None,
    );
    let target = create_target_texture(&gpu.device);
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
    assert!(settled > 0, "globe + blit pipelines must exist");
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
        "pipeline count {after} unexpectedly high for the globe path"
    );
}
