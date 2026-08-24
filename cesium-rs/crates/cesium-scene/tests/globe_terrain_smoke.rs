//! Headless acceptance tests for the globe terrain upgrade (Track B4-5).
//!
//! Renders the globe against an offline heightmap-1.0 terrain tileset
//! (`layer.json` + 65×65 u16 tiles, generated at test time) and asserts the
//! terrain loading contract:
//! - real heightmap tiles reach `Ready` and feed the terrain geometry path,
//! - a deterministically missing tile (404) is inherited by upsampling its
//!   nearest ancestor (`upsampledFrom`), never left as a hole,
//! - transient failures cool down and retry, and are NEVER stamped as
//!   permanent no-data (failed/placeholder discipline),
//! - level-zero deterministic no-data is the only terminal negative state.
//!
//! The mock-fetcher tests are pure CPU (no GPU required); the end-to-end
//! render test skips itself when no GPU adapter is available.

use std::collections::HashMap;
use std::sync::Arc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::cesium_terrain_provider::TerrainTileData;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::heightmap_terrain_data::{
    HeightmapBuffer, HeightmapStructureOptions, HeightmapTerrainData,
    HeightmapTerrainDataOptions,
};
use cesium_core::julian_date::JulianDate;
use cesium_core::pixel_format::PixelFormat;
use cesium_core::rectangle::Rectangle;
use cesium_core::tiling_scheme::TilingScheme;
use cesium_renderer::context::{Context, DefaultRenderTarget};
use cesium_renderer::texture::{Texture, TextureOptions};
use cesium_scene::file_imagery_provider::FileImageryProvider;
use cesium_scene::globe::Globe;
use cesium_scene::globe_terrain_fetcher::{
    FileTerrainFetcher, GlobeTerrainFetcher, TerrainGeometryOutcome,
};
use cesium_scene::globe_surface_tile_provider::{GlobeSurfaceTileProvider, TileTerrainState};
use cesium_scene::imagery_layer::ImageryLayer;
use cesium_scene::quadtree_tile::QuadtreeTile;
use cesium_scene::scene::Scene;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
/// `copy_texture_to_buffer` requires bytes_per_row to be a multiple of 256;
/// 1024 px × 4 bytes = 4096 already is.
const PADDED_BYTES_PER_ROW: u32 = 4096;
/// Imagery pyramid ceiling (drives the quadtree refinement cap).
const IMAGERY_MAXIMUM_LEVEL: u32 = 3;
/// Terrain tileset depth (levels 0..=TERRAIN_MAXIMUM_LEVEL on disk).
const TERRAIN_MAXIMUM_LEVEL: u32 = 2;
/// Heightmap grid width (heightmap-1.0 default).
const TERRAIN_SIZE: usize = 65;
/// The deterministic no-data probe tile: level 2, x 5, geographic y 0.
const MISSING_TILE: (i32, i32, i32) = (2, 5, 0);

// ────────────────────────── GPU harness ──────────────────────────

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
        label: Some("globe_terrain_smoke"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .ok()?;
    Some(Gpu { device, queue })
}

/// Copies the whole `size`×`size` target texture back to CPU (row-major
/// RGBA, unpadded).
fn read_pixels(gpu: &Gpu, texture: &wgpu::Texture, size: u32) -> Vec<u8> {
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("globe terrain smoke readback"),
        size: (PADDED_BYTES_PER_ROW * size) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("globe terrain smoke readback encoder"),
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
            width: size,
            height: size,
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
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    for row in 0..size as usize {
        let src = row * PADDED_BYTES_PER_ROW as usize;
        let dst = row * size as usize * 4;
        pixels[dst..dst + size as usize * 4]
            .copy_from_slice(&data[src..src + size as usize * 4]);
    }
    drop(data);
    readback.unmap();
    pixels
}

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
    target
}

// ────────────────────────── fixtures ──────────────────────────

/// The marker imagery pyramid (same orientation contract as `globe_smoke`):
/// red north cap / blue south cap / green-white checker mid-latitudes.
fn generate_marker_tiles(root: &std::path::Path) {
    use std::f64::consts::{FRAC_PI_2, PI};
    const SIZE: u32 = 256;
    for level in 0..=IMAGERY_MAXIMUM_LEVEL {
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

/// Encodes a metric height into the heightmap-1.0 u16 domain
/// (`height = encoded * (1/5) + (-1000)`).
fn encode_height(height_meters: f64) -> u16 {
    ((height_meters + 1000.0) * 5.0).round() as u16
}

/// Generates the offline heightmap-1.0 tileset: `layer.json` + 65×65 u16
/// tiles at levels 0..=TERRAIN_MAXIMUM_LEVEL (TMS y order on disk, matching
/// the provider's `scheme: "tms"` default). Everything EXCEPT
/// [`MISSING_TILE`] exists — its 404 drives the upsample-inheritance probe.
fn generate_terrain_tiles(root: &std::path::Path) {
    let layer_json = serde_json::json!({
        "tilejson": "2.1.0",
        "format": "heightmap-1.0",
        "version": "1.0.0",
        "scheme": "tms",
        "projection": "EPSG:4326",
        "maxzoom": TERRAIN_MAXIMUM_LEVEL,
        "tiles": ["{z}/{x}/{y}.terrain"]
    });
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("layer.json"),
        serde_json::to_string_pretty(&layer_json).unwrap(),
    )
    .unwrap();

    let count = TERRAIN_SIZE * TERRAIN_SIZE;
    for level in 0..=TERRAIN_MAXIMUM_LEVEL {
        let columns = 2u32 << level;
        let rows = 1u32 << level;
        for x in 0..columns {
            for y_geo in 0..rows {
                if (level as i32, x as i32, y_geo as i32) == MISSING_TILE {
                    continue; // deterministic 404 → upsample inheritance probe
                }
                // Heights ramp west→east so the decoded mesh is non-flat.
                let mut buffer: Vec<u8> = Vec::with_capacity(count * 2 + 2);
                for _row in 0..TERRAIN_SIZE {
                    for col in 0..TERRAIN_SIZE {
                        let u = col as f64 / (TERRAIN_SIZE - 1) as f64;
                        let height = 100.0 * level as f64 + 300.0 * u;
                        buffer.extend_from_slice(&encode_height(height).to_le_bytes());
                    }
                }
                // childTileMask: all four children exist below the leaf level.
                let child_mask: u8 = if level < TERRAIN_MAXIMUM_LEVEL { 0x0F } else { 0x00 };
                buffer.push(child_mask);
                // One-byte water mask (all land).
                buffer.push(0);

                let tms_y = rows - y_geo - 1;
                let path = root.join(format!("{level}/{x}/{tms_y}.terrain"));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &buffer).unwrap();
            }
        }
    }
}

fn file_url(root: &std::path::Path) -> String {
    format!("file:///{}", root.display().to_string().replace('\\', "/"))
}

// ────────────────────────── mock fetcher (CPU tests) ──────────────────────────

/// A scripted [`GlobeTerrainFetcher`] for the failed/placeholder discipline
/// tests: per-tile outcome queues popped in request order.
struct MockFetcher {
    outcomes: HashMap<(i32, i32, i32), Vec<TerrainGeometryOutcome>>,
    request_log: Vec<(i32, i32, i32)>,
}

impl MockFetcher {
    fn new() -> Self {
        Self {
            outcomes: HashMap::new(),
            request_log: Vec::new(),
        }
    }

    fn script(&mut self, key: (i32, i32, i32), outcomes: Vec<TerrainGeometryOutcome>) {
        self.outcomes.insert(key, outcomes);
    }
}

/// Builds a ready-to-mesh heightmap payload (65×65, heightmap-1.0 structure,
/// flat at 0 m, all children available).
fn make_heightmap() -> TerrainTileData {
    let count = TERRAIN_SIZE * TERRAIN_SIZE;
    let encoded = encode_height(0.0);
    TerrainTileData::Heightmap(HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::U16(vec![encoded; count])),
        width: Some(TERRAIN_SIZE),
        height: Some(TERRAIN_SIZE),
        child_tile_mask: Some(0x0F),
        water_mask: Some(vec![0]),
        structure: Some(HeightmapStructureOptions {
            height_scale: Some(1.0 / 5.0),
            height_offset: Some(-1000.0),
            elements_per_height: Some(1),
            stride: Some(1),
            element_multiplier: Some(256.0),
            is_big_endian: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    }))
}

impl GlobeTerrainFetcher for MockFetcher {
    fn make_tiling_scheme(&self) -> Box<dyn TilingScheme> {
        Box::new(GeographicTilingScheme::new(None, None, None, None))
    }

    fn level_zero_maximum_geometric_error(&self) -> f64 {
        15000.0
    }

    fn maximum_level(&self) -> Option<i32> {
        None
    }

    fn get_tile_data_available(&self, _x: i32, _y: i32, _level: i32) -> Option<bool> {
        None
    }

    fn request_tile_geometry(&mut self, x: i32, y: i32, level: i32) -> TerrainGeometryOutcome {
        let key = (level, x, y);
        self.request_log.push(key);
        match self.outcomes.get_mut(&key) {
            Some(queue) if !queue.is_empty() => queue.remove(0),
            _ => TerrainGeometryOutcome::NoData,
        }
    }
}

fn make_quadtree_tile(level: i32, x: i32, y: i32) -> QuadtreeTile {
    QuadtreeTile::new(x, y, level, Rectangle::default())
}

// ────────────────────────── tests ──────────────────────────

/// Transient IO failures must cool down and retry — NEVER be stamped as
/// permanent no-data (cesiumrust failed/placeholder checkpoint).
#[test]
fn terrain_transient_cooldown_retries_and_never_stamps_no_data() {
    let mut fetcher = MockFetcher::new();
    fetcher.script(
        (0, 0, 0),
        vec![
            TerrainGeometryOutcome::Transient("disk busy".to_string()),
            TerrainGeometryOutcome::Data(make_heightmap()),
        ],
    );

    let mut provider = GlobeSurfaceTileProvider::new();
    provider.set_terrain_fetcher(Some(Box::new(fetcher)));

    let tiles = vec![make_quadtree_tile(0, 0, 0)];

    // Frame 1: the request fails transiently → Transient (NOT NoData).
    provider.prepare_terrain(&tiles, 1);
    assert_eq!(
        provider.terrain_tile_state(0, 0, 0),
        Some(TileTerrainState::Transient),
        "a transient IO failure must land in the Transient class"
    );

    // Frames inside the cooldown window: no retry, still Transient — the
    // placeholder is never converted to permanent no-data. (Retry fires at
    // failure frame + TERRAIN_RETRY_COOLDOWN_FRAMES = 31.)
    for frame in 2..=30 {
        provider.prepare_terrain(&tiles, frame);
        assert_eq!(
            provider.terrain_tile_state(0, 0, 0),
            Some(TileTerrainState::Transient),
            "cooldown frame {frame} must not re-request nor give up"
        );
    }

    // After the cooldown: the retry succeeds and the tile becomes Ready.
    provider.prepare_terrain(&tiles, 31);
    assert_eq!(
        provider.terrain_tile_state(0, 0, 0),
        Some(TileTerrainState::Ready),
        "the post-cooldown retry must reach Ready"
    );
}

/// Deterministic no-data inheritance: a missing child receives an upsampled
/// copy of its nearest ready ancestor (`upsampledFrom`); a level-0 tile with
/// no ancestor is the only tile allowed to terminate in permanent `NoData`.
#[test]
fn terrain_no_data_inherits_ancestor_and_level_zero_is_terminal() {
    let mut fetcher = MockFetcher::new();
    // Root arrives; its (1,1,1) child is deterministically absent.
    fetcher.script((0, 0, 0), vec![TerrainGeometryOutcome::Data(make_heightmap())]);
    fetcher.script((1, 1, 1), vec![TerrainGeometryOutcome::NoData]);
    fetcher.script((2, 3, 3), vec![TerrainGeometryOutcome::NoData]);

    let mut provider = GlobeSurfaceTileProvider::new();
    provider.set_terrain_fetcher(Some(Box::new(fetcher)));

    // Deep request: (2,3,3) chains up through the missing (1,1,1) to the
    // ready root; every link is upsampled within the same prepare pass.
    let tiles = vec![make_quadtree_tile(2, 3, 3)];
    provider.prepare_terrain(&tiles, 1);

    assert_eq!(provider.terrain_tile_state(0, 0, 0), Some(TileTerrainState::Ready));
    assert_eq!(
        provider.terrain_tile_state(1, 1, 1),
        Some(TileTerrainState::Ready),
        "a deterministic no-data child must inherit via upsample"
    );
    assert_eq!(
        provider.terrain_upsampled_from(1, 1, 1),
        Some(Some((0, 0, 0))),
        "upsampledFrom must record the ancestor the mesh came from"
    );
    assert_eq!(
        provider.terrain_tile_state(2, 3, 3),
        Some(TileTerrainState::Ready),
        "the upsample chain must extend across multiple levels"
    );
    assert_eq!(provider.terrain_upsampled_from(2, 3, 3), Some(Some((1, 1, 1))));

    // A level-0 deterministic no-data has no ancestor: permanent NoData.
    let mut fetcher = MockFetcher::new();
    fetcher.script((0, 0, 0), vec![TerrainGeometryOutcome::NoData]);
    let mut provider = GlobeSurfaceTileProvider::new();
    provider.set_terrain_fetcher(Some(Box::new(fetcher)));
    provider.prepare_terrain(&[make_quadtree_tile(0, 0, 0)], 1);
    assert_eq!(
        provider.terrain_tile_state(0, 0, 0),
        Some(TileTerrainState::NoData),
        "level-0 deterministic absence is the terminal negative state"
    );
}

/// End-to-end: the globe renders against the offline heightmap tileset; the
/// deterministically missing tile (2,5,0) reaches Ready via ancestor
/// upsample, and every rendered tile's terrain settles in a terminal state.
#[test]
fn terrain_heightmap_tileset_renders_and_missing_tile_upsamples() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let imagery_root = std::env::temp_dir().join("cesium_rs_globe_terrain_smoke_imagery");
    if !imagery_root.join("0").is_dir() {
        generate_marker_tiles(&imagery_root);
    }
    let terrain_root = std::env::temp_dir().join("cesium_rs_globe_terrain_smoke_terrain");
    if !terrain_root.join("layer.json").is_file() {
        generate_terrain_tiles(&terrain_root);
    }

    let fetcher = FileTerrainFetcher::from_url(&file_url(&terrain_root))
        .expect("offline layer.json must load");

    let mut scene = Scene::new();
    scene.set_background_color(cesium_core::color::Color::new(0.0, 0.0, 0.2, 1.0));
    scene.viewport_quad_mut().show = false;

    let imagery = FileImageryProvider::new(&imagery_root, None);
    let mut globe = Globe::new(Some(Ellipsoid::WGS84));
    globe
        .imagery_layers_mut()
        .add(ImageryLayer::with_provider(Box::new(imagery)));
    globe.set_terrain_fetcher(Some(Box::new(fetcher)));
    scene.set_globe(Some(globe));

    // Camera straight down over the missing tile's rectangle (45°E..90°E,
    // 0..90°N → center 67.5°E, 45°N) so refinement lands on the probe tile.
    let nadir = Cartographic::new(
        67.5_f64.to_radians(),
        45.0_f64.to_radians(),
        Ellipsoid::WGS84.maximum_radius() * 0.5,
    );
    let mut destination = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(&nadir, &mut destination);
    scene
        .camera_mut()
        .set_view(&destination, None, None, &Ellipsoid::WGS84);

    let target = render_frames(&gpu, &mut scene, 3, 1024);
    let pixels = read_pixels(&gpu, target.wgpu_texture(), 1024);

    // The globe disc must be visible (terrain geometry actually drew).
    let center_offset = ((512usize) * 1024 + 512) * 4;
    let center = &pixels[center_offset..center_offset + 4];
    let is_background = center[0] < 40 && center[1] < 40 && center[2] < 90;
    assert!(!is_background, "globe disc must be visible at the center");

    let globe = scene.globe().expect("globe installed");
    let provider = globe.surface_tile_provider();
    let tiles = globe.surface().tiles_to_render();
    assert!(!tiles.is_empty(), "traversal must select tiles");

    // Every rendered tile's terrain must have settled to a terminal state
    // (Ready — real or upsampled; never Transient at rest for a local-file
    // provider, and NoData would fall back to the ellipsoid grid).
    for tile in tiles {
        let state = provider.terrain_tile_state(tile.level, tile.x, tile.y);
        assert_eq!(
            state,
            Some(TileTerrainState::Ready),
            "rendered tile ({}, {}, {}) must have ready terrain",
            tile.level,
            tile.x,
            tile.y
        );
    }

    // The missing probe tile was requested through the ancestor chain and
    // inherited real data by upsampling (never stamped NoData).
    assert_eq!(
        provider.terrain_tile_state(MISSING_TILE.0, MISSING_TILE.1, MISSING_TILE.2),
        Some(TileTerrainState::Ready),
        "the 404 probe tile must inherit ancestor terrain via upsample"
    );
    assert_eq!(
        provider.terrain_upsampled_from(MISSING_TILE.0, MISSING_TILE.1, MISSING_TILE.2),
        Some(Some((1, 2, 0))),
        "upsampledFrom must point at the probe tile's parent"
    );

    // Screenshot evidence (repo docs/).
    let screenshot_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/screenshots");
    std::fs::create_dir_all(&screenshot_dir).unwrap();
    let mut small = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    // Downsample 1024 → 256 (nearest) for the evidence shot.
    for row in 0..HEIGHT as usize {
        for col in 0..WIDTH as usize {
            let src = ((row * 4) * 1024 + col * 4) * 4;
            let dst = (row * WIDTH as usize + col) * 4;
            small[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
        }
    }
    let image = image::RgbaImage::from_raw(WIDTH, HEIGHT, small).unwrap();
    image.save(screenshot_dir.join("globe_terrain_smoke.png")).unwrap();
}
