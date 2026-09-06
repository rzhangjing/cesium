//! viewer-demo — Minimal Cesium viewer replicating Sandcastle HelloWorld,
//! upgraded (Track B4-3/B4-4/B4-5) to render the ellipsoid globe with an
//! offline local-file imagery layer and an offline heightmap terrain.
//!
//! Creates a winit window, initializes wgpu, and runs a frame loop that
//! renders the Cesium scene: background clear → globe tiles (offscreen
//! color+depth pass) → globe blit → present.
//!
//! The imagery is fully offline: a deterministic XYZ tile pyramid is
//! generated on first run under `assets/offline-imagery` (pole-marker
//! pattern: north red / south blue / mid-latitudes green-white checker) and
//! served through `FileImageryProvider` (file:// semantics, no network).
//!
//! The terrain is likewise offline (B4-5): a deterministic heightmap-1.0
//! tileset is generated under `assets/offline-terrain` (`layer.json` +
//! 65×65 u16 TMS tiles, west→east height ramp) and fed through
//! `FileTerrainFetcher` (a `CesiumTerrainProvider` over file:// URLs).
//!
//! Set `CESIUM_DEMO_SCREENSHOT=<path.png>` to capture a frame readback PNG
//! after the scene settles (headless acceptance evidence).

use std::sync::Arc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_scene::file_imagery_provider::FileImageryProvider;
use cesium_scene::globe::Globe;
use cesium_scene::globe_terrain_fetcher::FileTerrainFetcher;
use cesium_scene::imagery_layer::ImageryLayer;
use cesium_scene::imagery_provider::ImageryProvider;
use cesium_scene::web_mercator_imagery_provider::WebMercatorImageryProvider;
use cesium_widgets::viewer::Viewer;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

/// Number of frames rendered before the optional screenshot is captured.
/// Terrain tiles stream in over a few frames (fetch → createMesh → geometry
/// upload), so the delay covers the tileset settling.
const SCREENSHOT_FRAME_DELAY: u64 = 30;
/// Highest tile level generated for the offline imagery pyramid.
const OFFLINE_IMAGERY_MAXIMUM_LEVEL: u32 = 3;
/// Highest tile level generated for the offline heightmap terrain tileset.
const OFFLINE_TERRAIN_MAXIMUM_LEVEL: u32 = 2;
/// Heightmap grid width (heightmap-1.0 default).
const TERRAIN_GRID_SIZE: usize = 65;

/// The application state, holding GPU resources and the Cesium viewer.
struct State {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    /// The Cesium render context (frame orchestration over wgpu).
    context: Option<cesium_renderer::context::Context>,
    viewer: Option<Viewer>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    /// Frames rendered so far (drives the screenshot delay).
    frames_rendered: u64,
    /// Whether the optional screenshot was already captured.
    screenshot_done: bool,

    // ── Orbit camera state ──────────────────────────────────────
    /// Azimuth angle around the Z axis (radians). 0 = camera on +X.
    orbit_heading: f64,
    /// Elevation angle from the XY (equatorial) plane (radians).
    /// 0 = camera on the equator; +π/2 = above north pole; -π/2 = above south pole.
    orbit_pitch: f64,
    /// Distance from the Earth center (meters).
    orbit_distance: f64,
    /// Previous mouse cursor position (pixels) for drag delta computation.
    last_mouse_pos: Option<(f64, f64)>,
    /// Whether the left mouse button is currently held.
    left_dragging: bool,
    /// Whether the right mouse button is currently held.
    right_dragging: bool,
    /// Whether the orbit camera parameters changed this frame (needs update).
    orbit_dirty: bool,
}

impl State {
    /// Creates an uninitialized State (no GPU resources yet).
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            context: None,
            viewer: None,
            surface_config: None,
            frames_rendered: 0,
            screenshot_done: false,
            orbit_heading: 0.0,
            orbit_pitch: 0.3, // ~17° above equator for a classic globe view
            orbit_distance: 0.0, // set in init_gpu after viewer is created
            last_mouse_pos: None,
            left_dragging: false,
            right_dragging: false,
            orbit_dirty: false,
        }
    }

    /// Initializes GPU resources and the Cesium viewer.
    ///
    /// Called from `resumed()` once the event loop provides a valid display handle.
    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();

        // ── wgpu initialization ──────────────────────────────────────
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("create_surface failed");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        log::info!("GPU adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("cesium_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("request_device failed");

        // ── Surface configuration ────────────────────────────────────
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            // COPY_SRC enables the optional acceptance-screenshot readback.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            color_space: wgpu::SurfaceColorSpace::Auto,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        // ── Cesium Viewer ────────────────────────────────────────────
        // In CesiumJS: const viewer = new Cesium.Viewer("cesiumContainer");
        let mut viewer = Viewer::default();
        configure_scene(viewer.cesium_widget_mut().scene_mut());
        // Initialize orbit camera distance to 3× Earth radius.
        self.orbit_distance = Ellipsoid::WGS84.maximum_radius() * 3.0;
        self.orbit_dirty = true; // need initial camera update
        self.viewer = Some(viewer);

        // ── Cesium render context (wgpu frame orchestration) ─────────
        // DEVIATION: CesiumJS creates the Context inside CesiumWidget from
        // the canvas; the wgpu port builds it here from the same
        // device/queue (both are ref-counted handles, so cloning is cheap).
        let context = cesium_renderer::context::Context::new(
            device.clone(),
            queue.clone(),
            size.width.max(1),
            size.height.max(1),
            None,
        );

        log::info!(
            "viewer-demo initialized: {}x{}, format {:?}",
            size.width,
            size.height,
            surface_format
        );

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.context = Some(context);
        self.surface_config = Some(surface_config);
    }

    /// Updates the Cesium camera from the orbit state (heading/pitch/distance).
    ///
    /// The camera orbits the Earth center. Heading rotates around the Z axis
    /// (polar axis), pitch is the elevation from the XY plane, and distance
    /// is from the center. The camera always looks at the origin with the
    /// up vector derived from the ENU frame.
    fn update_orbit_camera(&mut self) {
        if !self.orbit_dirty {
            return;
        }
        self.orbit_dirty = false;

        let Some(ref mut viewer) = self.viewer else { return };
        let scene = viewer.cesium_widget_mut().scene_mut();

        let cos_pitch = self.orbit_pitch.cos();
        let sin_pitch = self.orbit_pitch.sin();
        let cos_heading = self.orbit_heading.cos();
        let sin_heading = self.orbit_heading.sin();

        // Camera position in ECEF.
        let position = Cartesian3::new(
            self.orbit_distance * cos_pitch * cos_heading,
            self.orbit_distance * cos_pitch * sin_heading,
            self.orbit_distance * sin_pitch,
        );

        // Use set_view which derives the orientation from the ENU frame,
        // matching CesiumJS behavior exactly.
        scene.camera_mut().set_view(&position, None, None, &Ellipsoid::WGS84);
    }

    /// Renders a single frame.
    ///
    /// Acquires the next surface texture, runs the Cesium scene render
    /// (background clear → globe offscreen pass → blit → execute), and
    /// presents. Optionally captures a readback screenshot.
    fn render(&mut self) {
        let device = self.device.as_ref().unwrap().clone();
        // Owned clone: the frame render below mutably destructures `self`.
        let config = self.surface_config.clone().unwrap();

        let surface = self.surface.as_ref().unwrap();
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&device, &config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Validation => {
                log::warn!("Surface texture lost or validation error; reconfiguring.");
                surface.configure(&device, &config);
                return;
            }
        };

        let texture_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Drive the Cesium scene through the wgpu Context: begin_frame →
        // clear → globe pass (offscreen) → blit → execute → end_frame.
        // Update the orbit camera before rendering (only when dirty).
        self.update_orbit_camera();

        let time = cesium_core::julian_date::JulianDate::now();
        let State { context, viewer, .. } = self;
        let context = context.as_mut().unwrap();
        let scene = viewer.as_mut().unwrap().cesium_widget_mut().scene_mut();
        let target = cesium_renderer::context::DefaultRenderTarget {
            view: &texture_view,
            format: config.format,
            width: config.width,
            height: config.height,
        };
        scene.render_with_context(&time, context, Some(target));

        self.frames_rendered += 1;
        if !self.screenshot_done && self.frames_rendered >= SCREENSHOT_FRAME_DELAY {
            if let Some(path) = std::env::var_os("CESIUM_DEMO_SCREENSHOT") {
                let width = config.width;
                let height = config.height;
                let format = config.format;
                self.capture_screenshot(&frame.texture, width, height, format, &path);
            }
        }

        let queue = self.queue.as_ref().unwrap();
        queue.present(frame);

        // Update the viewer (clock, data sources) for the next frame
        if let Some(viewer) = self.viewer.as_mut() {
            viewer.render();
        }
    }

    /// Copies the presented frame back to CPU and writes it as a PNG
    /// (acceptance evidence: the window content with the imaged globe).
    fn capture_screenshot(
        &mut self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        path: &std::ffi::OsStr,
    ) {
        self.screenshot_done = true;
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();

        let padded_bytes_per_row = ((width * 4) + 255) & !255;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewer-demo screenshot readback"),
            size: (padded_bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewer-demo screenshot encoder"),
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
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        // wgpu 30 buffer mapping is callback-based: map_async + poll.
        let slice = readback.slice(..);
        let mapped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mapped_flag = mapped.clone();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            if result.is_ok() {
                mapped_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        let mut attempts = 0u32;
        while !mapped.load(std::sync::atomic::Ordering::SeqCst) {
            attempts += 1;
            if attempts >= 100 {
                log::warn!("screenshot readback never completed");
                return;
            }
            let _ = device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            });
        }

        let data = slice.get_mapped_range().expect("mapped range unavailable");
        let bgra = matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for row in 0..height as usize {
            let src = row * padded_bytes_per_row as usize;
            let dst = row * width as usize * 4;
            pixels[dst..dst + width as usize * 4]
                .copy_from_slice(&data[src..src + width as usize * 4]);
        }
        drop(data);
        readback.unmap();

        if bgra {
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        match image::RgbaImage::from_raw(width, height, pixels) {
            Some(image) => match image.save(path) {
                Ok(()) => log::info!("screenshot saved to {:?}", path),
                Err(error) => log::warn!("screenshot save failed: {error}"),
            },
            None => log::warn!("screenshot buffer size mismatch"),
        }
    }

    /// Handles window resize by reconfiguring the surface and notifying the viewer.
    fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            if let Some(ref mut viewer) = self.viewer {
                viewer.resize(new_width, new_height);
            }
            if let Some(ref mut context) = self.context {
                context.resize(new_width, new_height);
            }
            if let Some(config) = self.surface_config.as_mut() {
                config.width = new_width;
                config.height = new_height;
            }
            if let (Some(surface), Some(device), Some(config)) =
                (&self.surface, &self.device, &self.surface_config)
            {
                surface.configure(device, config);
            }
        }
    }
}

/// Generates the offline XYZ tile pyramid under `root` when missing.
///
/// Layout: `{root}/{level}/{x}/{y}.png`, geographic tiling scheme with a
/// 2×1 level-0 root (columns = 2·2ᴸ, rows = 2ᴸ, y = 0 at the north pole).
/// The pattern is a UV-orientation marker: red above +45° latitude, blue
/// below −45°, green/white checker with a longitude gradient in between —
/// an upright globe must show red on top and blue at the bottom.
/// The demo's scene setup: globe + offline imagery + offline terrain +
/// 3D model + camera.
fn configure_scene(scene: &mut cesium_scene::scene::Scene) {
    // Imagery source priority:
    // 1) The NaturalEarthII base map shipped with the original CesiumJS engine
    //    (a geographic XYZ pyramid — byte-for-byte the format FileImageryProvider
    //    expects, and the exact default imagery CesiumJS shows with no ion token);
    // 2) fall back to the procedurally generated checkerboard when the asset is
    //    absent, so the demo still runs in a stripped/offline checkout.
    let natural_earth_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("packages")
        .join("engine")
        .join("Source")
        .join("Assets")
        .join("Textures")
        .join("NaturalEarthII");
    let (imagery_root, flip_y) = if natural_earth_root.join("0").is_dir() {
        // NaturalEarthII ships as TMS (row 0 at the south) → flip to north-first.
        (natural_earth_root, true)
    } else {
        let generated = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("offline-imagery");
        ensure_offline_imagery(&generated);
        (generated, false)
    };
    let provider = FileImageryProvider::new(&imagery_root, None);
    let provider = if flip_y { provider.with_flip_y() } else { provider };
    log::info!(
        "imagery root: {} (maximum_level = {:?})",
        imagery_root.display(),
        provider.maximum_level()
    );

    let mut globe = Globe::new(Some(Ellipsoid::WGS84));
    globe.enable_lighting = true;
    {
        let layers = globe.imagery_layers_mut();
        // Base layer: the local NaturalEarthII (or checkerboard) — always
        // resolves offline, so the globe is never blank.
        layers.add(ImageryLayer::with_provider(Box::new(provider)));
        // Top layer: real satellite imagery streamed from a Web Mercator XYZ
        // service and re-projected onto the geographic grid on the CPU. Where
        // the network has no tile the provider reports Transient, so the base
        // map shows through — graceful degradation when offline. Esri World
        // Imagery needs no API key; note its {z}/{y}/{x} path ordering.
        let satellite = WebMercatorImageryProvider::new(
            "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
            4,
        );
        layers.add(ImageryLayer::with_provider(Box::new(satellite)));
    }

    // B4-5: offline heightmap terrain through the cesium-core
    // CesiumTerrainProvider (file:// backend, no network). A load
    // failure falls back to the ellipsoid terrain path.
    let terrain_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("offline-terrain");
    ensure_offline_terrain(&terrain_root);
    let terrain_url = format!(
        "file:///{}/layer.json",
        terrain_root.display().to_string().replace('\\', "/")
    );
    match FileTerrainFetcher::from_url(&terrain_url) {
        Ok(fetcher) => {
            log::info!("offline terrain root: {}", terrain_root.display());
            globe.set_terrain_fetcher(Some(Box::new(fetcher)));
        }
        Err(error) => {
            log::warn!("terrain provider load failed ({error:?}); using ellipsoid terrain");
        }
    }

    scene.set_globe(Some(globe));

    // The smoke quad is replaced by the globe path.
    scene.viewport_quad_mut().show = false;
    scene.set_background_color(cesium_core::color::Color::new(0.02, 0.02, 0.08, 1.0));

    // Camera is managed by the orbit camera in the viewer-demo State.
    // Set an initial view so the first frame renders correctly before
    // the orbit camera takes over.
    let destination = Cartesian3::new(Ellipsoid::WGS84.maximum_radius() * 3.0, 0.0, 0.0);
    scene
        .camera_mut()
        .set_view(&destination, None, None, &Ellipsoid::WGS84);
}

fn ensure_offline_imagery(root: &std::path::Path) {
    if root.join("0").is_dir() {
        return;
    }
    for level in 0..=OFFLINE_IMAGERY_MAXIMUM_LEVEL {
        let columns = 2u32 << level;
        let rows = 1u32 << level;
        for x in 0..columns {
            for y in 0..rows {
                let image = generate_tile(level, x, y, columns, rows);
                let path = root.join(format!("{level}/{x}/{y}.png"));
                std::fs::create_dir_all(path.parent().expect("tile dir parent"))
                    .expect("create tile dir");
                image.save(&path).expect("save tile png");
            }
        }
    }
    log::info!(
        "generated offline imagery pyramid (levels 0..={}) at {}",
        OFFLINE_IMAGERY_MAXIMUM_LEVEL,
        root.display()
    );
}

/// Encodes a metric height into the heightmap-1.0 u16 domain
/// (`height = encoded * (1/5) + (-1000)`).
fn encode_terrain_height(height_meters: f64) -> u16 {
    ((height_meters + 1000.0) * 5.0).round() as u16
}

/// Generates the offline heightmap-1.0 tileset under `root` when missing.
///
/// Layout mirrors the `globe_terrain_smoke` fixture: `layer.json` plus
/// `{level}/{x}/{y}.terrain` tiles with TMS y order on disk (the provider's
/// `scheme: "tms"` default flips y internally). Each tile is 65×65 u16-LE
/// heights (west→east ramp, so the mesh is visibly non-flat) + 1-byte
/// childTileMask + 1-byte water mask.
fn ensure_offline_terrain(root: &std::path::Path) {
    if root.join("layer.json").is_file() {
        return;
    }
    let layer_json = format!(
        "{{\n  \"tilejson\": \"2.1.0\",\n  \"format\": \"heightmap-1.0\",\n  \
         \"version\": \"1.0.0\",\n  \"scheme\": \"tms\",\n  \
         \"projection\": \"EPSG:4326\",\n  \"maxzoom\": {maxzoom},\n  \
         \"tiles\": [\"{{z}}/{{x}}/{{y}}.terrain\"]\n}}\n",
        maxzoom = OFFLINE_TERRAIN_MAXIMUM_LEVEL
    );
    std::fs::create_dir_all(root).expect("create terrain root");
    std::fs::write(root.join("layer.json"), layer_json).expect("write layer.json");

    let count = TERRAIN_GRID_SIZE * TERRAIN_GRID_SIZE;
    for level in 0..=OFFLINE_TERRAIN_MAXIMUM_LEVEL {
        let columns = 2u32 << level;
        let rows = 1u32 << level;
        for x in 0..columns {
            for y_geo in 0..rows {
                // Heights ramp west→east so the decoded mesh is non-flat.
                let mut buffer: Vec<u8> = Vec::with_capacity(count * 2 + 2);
                for _row in 0..TERRAIN_GRID_SIZE {
                    for col in 0..TERRAIN_GRID_SIZE {
                        let u = col as f64 / (TERRAIN_GRID_SIZE - 1) as f64;
                        let height = 100.0 * level as f64 + 300.0 * u;
                        buffer.extend_from_slice(&encode_terrain_height(height).to_le_bytes());
                    }
                }
                // childTileMask: all four children exist below the leaf level.
                let child_mask: u8 = if level < OFFLINE_TERRAIN_MAXIMUM_LEVEL { 0x0F } else { 0x00 };
                buffer.push(child_mask);
                // One-byte water mask (all land).
                buffer.push(0);

                let tms_y = rows - y_geo - 1;
                let path = root.join(format!("{level}/{x}/{tms_y}.terrain"));
                std::fs::create_dir_all(path.parent().expect("terrain tile dir parent"))
                    .expect("create terrain tile dir");
                std::fs::write(&path, &buffer).expect("write terrain tile");
            }
        }
    }
    log::info!(
        "generated offline terrain tileset (levels 0..={}) at {}",
        OFFLINE_TERRAIN_MAXIMUM_LEVEL,
        root.display()
    );
}

/// Renders one 256×256 tile for the geographic scheme (y = 0 at north).
fn generate_tile(
    _level: u32,
    x: u32,
    y: u32,
    columns: u32,
    rows: u32,
) -> image::RgbaImage {
    use std::f64::consts::{FRAC_PI_2, PI};
    const SIZE: u32 = 256;

    let west = -PI + (x as f64) * (2.0 * PI) / (columns as f64);
    let north = FRAC_PI_2 - (y as f64) * PI / (rows as f64);
    let lon_step = (2.0 * PI) / (columns as f64) / (SIZE as f64);
    let lat_step = -PI / (rows as f64) / (SIZE as f64);

    let mut image = image::RgbaImage::new(SIZE, SIZE);
    for py in 0..SIZE {
        let latitude = north + (py as f64 + 0.5) * lat_step;
        for px in 0..SIZE {
            let longitude = west + (px as f64 + 0.5) * lon_step;
            let color = if latitude > PI / 4.0 {
                // North polar cap: red (UV flip marker — must appear on top).
                [255, 24, 24, 255]
            } else if latitude < -PI / 4.0 {
                // South polar cap: blue (must appear at the bottom).
                [24, 64, 255, 255]
            } else {
                // Mid latitudes: green/white checker + longitude gradient,
                // giving visible texture to spot seams and stretching.
                let checker = ((px / 32) + (py / 32)) % 2 == 0;
                let gradient = ((longitude + PI) / (2.0 * PI) * 128.0) as u8;
                if checker {
                    [32 + gradient / 4, 200, 64, 255]
                } else {
                    [232, 232, 224, 255]
                }
            };
            image.put_pixel(px, py, image::Rgba(color));
        }
    }
    image
}

impl ApplicationHandler for State {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // On desktop, resumed is called once after the event loop starts.
        // Initialize GPU resources if not already done.
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32))
                            .with_min_inner_size(winit::dpi::LogicalSize::new(256u32, 256u32))
                            .with_title("CesiumRust Globe Viewer"),
                    )
                    .expect("Failed to create window"),
            );

            // pollster::block_on is safe here because we're not inside an async context.
            pollster::block_on(self.init_gpu(window));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(ref window) = self.window {
            if window_id != window.id() {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Window close requested, shutting down.");
                if let Some(ref mut viewer) = self.viewer {
                    viewer.destroy();
                }
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size.width, physical_size.height);
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let current_pos = (position.x, position.y);
                if let Some(prev_pos) = self.last_mouse_pos {
                    let dx = current_pos.0 - prev_pos.0;
                    let dy = current_pos.1 - prev_pos.1;

                    // Sensitivity: radians per pixel.
                    const ROTATE_SENSITIVITY: f64 = 0.005;

                    if self.left_dragging {
                        // Left drag: orbit around the globe.
                        // Horizontal → heading (rotate around Z / polar axis).
                        self.orbit_heading -= dx * ROTATE_SENSITIVITY;
                        // Vertical → pitch (elevation from equatorial plane).
                        self.orbit_pitch += dy * ROTATE_SENSITIVITY;
                        self.orbit_pitch = self.orbit_pitch.clamp(-1.5, 1.5);
                        self.orbit_dirty = true;
                    }

                    if self.right_dragging {
                        // Right drag: pan (translate the camera perpendicular
                        // to the view direction). Approximate by shifting
                        // the orbit position laterally.
                        let pan_scale = self.orbit_distance * 0.001;
                        let cos_h = self.orbit_heading.cos();
                        let sin_h = self.orbit_heading.sin();
                        // Right vector at the camera (tangent to the orbit sphere).
                        let right_x = -sin_h;
                        let right_y = cos_h;
                        // Up vector component (tangent toward north pole).
                        let cos_p = self.orbit_pitch.cos();
                        let sin_p = self.orbit_pitch.sin();
                        let up_x = -sin_p * cos_h;
                        let up_y = -sin_p * sin_h;
                        let up_z = cos_p;

                        // Shift the virtual "look-at" point (origin) by the
                        // pan amount. We approximate by adjusting heading and
                        // pitch to simulate panning.
                        self.orbit_heading -= dx * pan_scale / self.orbit_distance;
                        self.orbit_pitch += dy * pan_scale / self.orbit_distance;
                        self.orbit_pitch = self.orbit_pitch.clamp(-1.5, 1.5);
                        self.orbit_dirty = true;
                        let _ = (right_x, right_y, up_x, up_y, up_z);
                    }
                }
                self.last_mouse_pos = Some(current_pos);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        self.left_dragging = pressed;
                        if !pressed {
                            self.last_mouse_pos = None;
                        }
                    }
                    MouseButton::Right => {
                        self.right_dragging = pressed;
                        if !pressed {
                            self.last_mouse_pos = None;
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Zoom: based on surface height (distance - R) for consistent
                // feel at all altitudes (per memory: avoid "one scroll = crash").
                let scroll_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(pos) => pos.y / 100.0,
                };
                let earth_radius = Ellipsoid::WGS84.maximum_radius();
                let min_surface = earth_radius * 0.01;
                let max_surface = earth_radius * 9.0;
                let surface_dist = (self.orbit_distance - earth_radius).clamp(min_surface, max_surface);
                let zoom_speed = 0.25;
                let new_surface = (surface_dist * (1.0 - scroll_y * zoom_speed))
                    .clamp(min_surface, max_surface);
                self.orbit_distance = earth_radius + new_surface;
                self.orbit_dirty = true;
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Continuous rendering (equivalent to CesiumJS requestAnimationFrame)
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut state = State::new();

    if let Err(e) = event_loop.run_app(&mut state) {
        log::error!("Event loop error: {:?}", e);
    }
}
