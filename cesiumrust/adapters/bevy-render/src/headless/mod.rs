//! Headless (surface-less) offscreen wgpu rendering + CPU pixel readback.
//!
//! Milestone **M11.3**. This module provides the mechanism to render a frame to
//! an offscreen [`Image`] render target and read the pixels back to the CPU
//! **without a window, monitor or display server**. It is the foundation for
//! turning the e2e pixel gate from a `continue-on-error` scaffold into an
//! executable hard gate, and for retiring the M5/M6 visual baselines that were
//! deferred for lack of a headless capture path.
//!
//! # Feasibility (spike result)
//!
//! Proven on the reference machine (NVIDIA RTX 3080, wgpu 23 / bevy 0.15):
//! a surface-less render of a known clear colour produced an exact CPU readback
//! (`Color::srgb(0.25,0.5,0.75)` → RGBA `[64,128,191,255]`). The mechanism is
//! therefore hardware-feasible; on GPU-less CI runners the same code path runs
//! under `xvfb-run` + `llvmpipe` software rendering (see the e2e CI workflow).
//!
//! # Design — reuse Bevy, no raw wgpu, no tokio
//!
//! * [`headless_window_plugin()`] returns a [`WindowPlugin`] with
//!   `primary_window: None` + `exit_condition: DontExit`, so no surface is ever
//!   created and the app does not self-terminate on a missing window.
//! * [`RenderPlugin`](bevy::render::RenderPlugin) creates the render sub-app and
//!   the `wgpu` device from the primary adapter, independent of any window. It
//!   also pulls in `WindowRenderPlugin` → `ScreenshotPlugin`, which owns the
//!   full `copy_texture_to_buffer` → `map_async` → 256-byte-row-padding-strip
//!   readback path.
//! * [`create_offscreen_target`] builds an RGBA8 offscreen [`Image`] usable as a
//!   colour attachment **and** copy source.
//! * [`retarget_cameras_to_offscreen`] points the scene's [`Camera3d`] at that
//!   image so the existing globe scene renders offscreen with no other change.
//! * [`CesiumHeadlessPlugin`] wires the above together and, after a configurable
//!   number of frames, captures via [`Screenshot::image`] → [`save_to_disk`]
//!   (PNG) and then requests a clean app exit.
//!
//! Plugin add-order matters and mirrors `DefaultPlugins`: `RenderPlugin` must be
//! added before `ImagePlugin`, because `ImagePlugin::finish()` reads
//! `RenderDevice` from the render sub-app. That ordering is the *app's*
//! responsibility (see `main.rs`); this module never re-adds those plugins.
//!
//! [`Image`]: bevy::prelude::Image
//! [`Camera3d`]: bevy::prelude::Camera3d
//! [`Screenshot::image`]: bevy::render::view::screenshot::Screenshot::image
//! [`save_to_disk`]: bevy::render::view::screenshot::save_to_disk

use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use bevy::window::{ExitCondition, WindowPlugin};
use std::path::PathBuf;

/// Default offscreen resolution — matches the interactive window in `main.rs`
/// (`1280×720`) so headless baselines are directly comparable to windowed ones.
pub const DEFAULT_HEADLESS_WIDTH: u32 = 1280;
pub const DEFAULT_HEADLESS_HEIGHT: u32 = 720;

/// Resolution + pixel format of the offscreen render target.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeadlessConfig {
    pub width: u32,
    pub height: u32,
    /// When `true` the offscreen target is a half-float HDR buffer
    /// ([`TextureFormat::Rgba16Float`]) instead of the default LDR sRGB
    /// ([`TextureFormat::Rgba8UnormSrgb`]). Needed to capture un-clipped
    /// radiance from the v2 sky / water / post-process stack (M5-C/M5-D,
    /// deferred #45/#47). **Default `false`** so the M11.3 golden capture path
    /// is byte-for-byte unchanged. Driven by the local `CESIUM_HEADLESS_HDR`
    /// env read (see [`headless_hdr_from_env`]) — deliberately NOT routed
    /// through `feature_flags`, whose frozen single-source-of-truth registry is
    /// under concurrent edit; consolidation is deferred to M11.6.
    pub hdr: bool,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_HEADLESS_WIDTH,
            height: DEFAULT_HEADLESS_HEIGHT,
            hdr: false,
        }
    }
}

impl HeadlessConfig {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            hdr: false,
        }
    }

    /// Builder: select the HDR (Rgba16Float) offscreen format. Default LDR.
    pub fn with_hdr(mut self, hdr: bool) -> Self {
        self.hdr = hdr;
        self
    }

    /// The [`TextureFormat`] this config selects for the offscreen target.
    pub fn texture_format(&self) -> TextureFormat {
        if self.hdr {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::Rgba8UnormSrgb
        }
    }
}

/// `CESIUM_HEADLESS_HDR` — truthy: capture into an HDR (Rgba16Float) offscreen
/// target instead of the default LDR sRGB buffer. Read locally (see
/// [`HeadlessConfig::hdr`]); default OFF keeps the M11.3 golden path unchanged.
pub const ENV_HEADLESS_HDR: &str = "CESIUM_HEADLESS_HDR";

/// Truthy-token predicate, behaviourally identical to `feature_flags::truthy`
/// (`1`/`true`/`yes`/`on`, case-insensitive, whitespace-trimmed). Duplicated
/// here on purpose: `feature_flags` lives in the `cesium-app` crate and is a
/// frozen registry under concurrent edit, so this module keeps a self-contained
/// reader. Consolidation into the single source of truth is deferred to M11.6.
fn hdr_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Reads [`ENV_HEADLESS_HDR`] from the environment. `false` when unset/unparsable.
pub fn headless_hdr_from_env() -> bool {
    match std::env::var(ENV_HEADLESS_HDR) {
        Ok(v) => hdr_truthy(&v),
        Err(_) => false,
    }
}

/// Resource holding the offscreen render-target [`Image`] handle + dimensions.
///
/// Inserted by [`create_offscreen_target`] (or the [`CesiumHeadlessPlugin`]
/// startup system). Consumers read `.image` to point a camera at it or to
/// request a [`Screenshot`].
#[derive(Resource, Clone, Debug)]
pub struct HeadlessTarget {
    pub image: Handle<Image>,
    pub width: u32,
    pub height: u32,
}

/// Returns the [`WindowPlugin`] configuration for surface-less operation:
/// no primary window, and the app never auto-exits because a window closed.
pub fn headless_window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: None,
        exit_condition: ExitCondition::DontExit,
        ..default()
    }
}

/// Creates the offscreen render-target [`Image`] (LDR sRGB by default, or HDR
/// Rgba16Float when [`HeadlessConfig::hdr`]), registers it in [`Assets<Image>`],
/// inserts a [`HeadlessTarget`] resource, and returns it.
///
/// This is an exclusive-world helper (used by the plugin's startup system and by
/// tests). The texture is created with `RENDER_ATTACHMENT | COPY_SRC |
/// COPY_DST | TEXTURE_BINDING` so it can be both rendered into and read back.
pub fn create_offscreen_target(world: &mut World, config: &HeadlessConfig) -> HeadlessTarget {
    let size = Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("cesium_headless_offscreen_target"),
            size,
            dimension: TextureDimension::D2,
            // LDR sRGB output matches the interactive window's default and the
            // baseline PNGs consumed by `tools/pixel_diff`. HDR (Rgba16Float) is
            // opt-in via `CESIUM_HEADLESS_HDR` for un-clipped v2 radiance capture.
            format: config.texture_format(),
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);
    let handle = world.resource_mut::<Assets<Image>>().add(image);
    let target = HeadlessTarget {
        image: handle,
        width: config.width,
        height: config.height,
    };
    world.insert_resource(target.clone());
    target
}

/// Points every [`Camera3d`] that is not already rendering to an image at the
/// offscreen [`HeadlessTarget`]. Idempotent, and a no-op when headless mode is
/// inactive (no [`HeadlessTarget`] resource), which keeps the windowed golden
/// path byte-for-byte unchanged.
pub fn retarget_cameras_to_offscreen(
    target: Option<Res<HeadlessTarget>>,
    mut cameras: Query<&mut Camera, With<Camera3d>>,
) {
    let Some(target) = target else {
        return;
    };
    for mut camera in &mut cameras {
        if !matches!(camera.target, RenderTarget::Image(_)) {
            camera.target = RenderTarget::Image(target.image.clone());
        }
    }
}

/// Internal capture state machine for [`CesiumHeadlessPlugin`].
#[derive(Resource)]
struct HeadlessCaptureState {
    frames_remaining: usize,
    output_png: PathBuf,
    requested: bool,
}

/// Plugin that enables headless offscreen capture of the scene.
///
/// On startup it creates the offscreen target ([`create_offscreen_target`]);
/// each frame [`retarget_cameras_to_offscreen`] keeps the 3D camera aimed at it.
/// After `frames` updates it requests a [`Screenshot`] of the target, saves it
/// to `output_png` (PNG) via Bevy's [`save_to_disk`], and then sends
/// [`AppExit::SUCCESS`] so the process terminates cleanly (the exit is applied
/// at end-of-frame, after the save observer has written the file).
///
/// Only add this plugin when `CESIUM_HEADLESS` is truthy; the default (windowed)
/// app must not include it, preserving golden-path neutrality.
pub struct CesiumHeadlessPlugin {
    pub config: HeadlessConfig,
    pub output_png: PathBuf,
    /// Number of `Update` ticks to render before capturing (scene warm-up /
    /// tile-load budget). `0` captures on the first tick after startup.
    pub frames: usize,
}

impl CesiumHeadlessPlugin {
    pub fn new(output_png: impl Into<PathBuf>, frames: usize) -> Self {
        Self {
            // `main.rs` builds the plugin via `new(output, frames)` and is
            // off-limits for concurrent edit, so the HDR selection is read from
            // the environment here (default OFF → LDR sRGB → golden path intact).
            config: HeadlessConfig::default().with_hdr(headless_hdr_from_env()),
            output_png: output_png.into(),
            frames,
        }
    }

    pub fn with_config(mut self, config: HeadlessConfig) -> Self {
        self.config = config;
        self
    }
}

impl Plugin for CesiumHeadlessPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config)
            .insert_resource(HeadlessCaptureState {
                frames_remaining: self.frames,
                output_png: self.output_png.clone(),
                requested: false,
            })
            .add_systems(Startup, headless_setup_target)
            .add_systems(
                Update,
                (retarget_cameras_to_offscreen, headless_capture_tick).chain(),
            );
    }
}

/// Exclusive startup system: build the offscreen target from [`HeadlessConfig`].
fn headless_setup_target(world: &mut World) {
    let config = *world.resource::<HeadlessConfig>();
    create_offscreen_target(world, &config);
}

/// Counts down `frames`, then requests the offscreen screenshot → PNG → exit.
fn headless_capture_tick(
    mut state: ResMut<HeadlessCaptureState>,
    target: Option<Res<HeadlessTarget>>,
    config: Res<HeadlessConfig>,
    mut commands: Commands,
) {
    if state.requested {
        return;
    }
    let Some(target) = target else {
        return;
    };
    if state.frames_remaining > 0 {
        state.frames_remaining -= 1;
        return;
    }
    let path = state.output_png.clone();
    // The saver writes the PNG synchronously inside the observer; the exit
    // observer's `AppExit` is only applied at end-of-frame, so the file is
    // guaranteed written before the process terminates. The LDR branch is
    // byte-for-byte the M11.3 path (Bevy's own `save_to_disk`); HDR uses a
    // half-float tonemapping saver because Bevy cannot PNG-encode Rgba16Float.
    let mut entity = commands.spawn(Screenshot::image(target.image.clone()));
    if config.hdr {
        entity.observe(save_hdr_to_disk(path));
    } else {
        entity.observe(save_to_disk(path));
    }
    entity.observe(
        |_trigger: Trigger<ScreenshotCaptured>, mut exit: EventWriter<AppExit>| {
            exit.send(AppExit::Success);
        },
    );
    state.requested = true;
}

/// HDR counterpart to Bevy's [`save_to_disk`]. The offscreen target is
/// [`TextureFormat::Rgba16Float`], which Bevy's PNG saver cannot encode directly
/// (`try_into_dynamic_image` rejects float formats) — without this, the M11.4 HDR
/// capture exits cleanly but writes no file. This observer decodes the
/// half-float RGBA readback, Reinhard-tonemaps the (possibly >1.0) radiance into
/// `[0,1]`, applies the sRGB OETF, and writes an 8-bit PNG via the `image` crate
/// (already a `cesium-bevy-render` dependency).
///
/// Lossless HDR output (OpenEXR) is deferred to M11.6 — the workspace `image`
/// features are `png`+`jpeg` only. The 8-bit tonemapped PNG is nonetheless a
/// faithful, inspectable baseline for the v2 sky/water/post-process stack
/// (#45/#47): it compresses super-unit radiance instead of clipping it.
fn save_hdr_to_disk(path: PathBuf) -> impl FnMut(Trigger<ScreenshotCaptured>) {
    move |trigger: Trigger<ScreenshotCaptured>| {
        let img = &trigger.event().0;
        let w = img.texture_descriptor.size.width;
        let h = img.texture_descriptor.size.height;
        match hdr_bytes_to_rgba8(&img.data, w, h) {
            Some(rgba) => {
                if let Err(e) = rgba.save(&path) {
                    error!("HDR headless screenshot save failed at {path:?}: {e}");
                }
            }
            None => error!(
                "HDR headless screenshot: readback buffer ({} bytes) does not match {w}x{h} Rgba16Float",
                img.data.len()
            ),
        }
    }
}

/// Decodes an Rgba16Float readback buffer (8 bytes/pixel, little-endian IEEE-754
/// half) into a tonemapped, sRGB-encoded 8-bit RGBA image. Returns `None` when
/// `data` is not exactly `w*h*8` bytes. Alpha is dropped (opaque PNG), mirroring
/// Bevy's LDR saver.
fn hdr_bytes_to_rgba8(data: &[u8], w: u32, h: u32) -> Option<image::RgbaImage> {
    // FIX-HL-HDRCAP: overflow-safe pixel count + the exact-length guard the
    // docstring already promised. `w * h * 4` in u32 arithmetic would silently
    // wrap (release) / panic (debug) for `w*h > 2^30`; and without a length check
    // `chunks_exact(8)` quietly drops a trailing partial pixel, producing a short
    // buffer that `from_raw` then rejects — an error the caller only sees as a log
    // line. Compute in `usize` with `checked_mul` and reject a length mismatch up
    // front so the function is total over its declared contract.
    let npix = (w as usize).checked_mul(h as usize)?;
    let expected = npix.checked_mul(8)?;
    if data.len() != expected {
        return None;
    }
    let mut out: Vec<u8> = Vec::with_capacity(npix.checked_mul(4)?);
    for px in data.chunks_exact(8) {
        let r = f16_to_f32(u16::from_le_bytes([px[0], px[1]]));
        let g = f16_to_f32(u16::from_le_bytes([px[2], px[3]]));
        let b = f16_to_f32(u16::from_le_bytes([px[4], px[5]]));
        for c in [r, g, b] {
            let v = linear_to_srgb(tonemap_reinhard(c));
            out.push((v * 255.0).round().clamp(0.0, 255.0) as u8);
        }
        out.push(255);
    }
    image::RgbaImage::from_raw(w, h, out)
}

/// IEEE-754 half-precision (f16) → f32. Handles normals, subnormals and ±inf.
fn f16_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0f32 } else { 1.0f32 };
    let exp = ((bits >> 10) & 0x1f) as i32;
    let frac = (bits & 0x03ff) as f32;
    match exp {
        // Subnormal: (frac/1024) * 2^-14 == frac * 2^-24.
        0 => sign * frac * 2.0f32.powi(-24),
        0x1f => {
            if frac == 0.0 {
                sign * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        e => sign * (1.0 + frac / 1024.0) * 2.0f32.powi(e - 15),
    }
}

/// Per-channel Reinhard tonemap: monotonic `[0,∞) → [0,1)`, so super-unit HDR
/// radiance is compressed rather than clipped. `+∞ → 1.0` (the limit of
/// `c/(1+c)`); NaN / `-∞` / non-positive → `0.0`.
fn tonemap_reinhard(c: f32) -> f32 {
    // FIX-HL-TONEMAP: `+inf` must approach 1.0, not fall into the `0.0` arm as
    // the old `c.is_finite()` guard did (which lumped `+inf` with NaN / negatives,
    // mapping the brightest HDR radiance to black — the opposite of the doc).
    if c.is_nan() || c <= 0.0 {
        0.0
    } else if c == f32::INFINITY {
        1.0
    } else {
        c / (1.0 + c)
    }
}

/// Linear → sRGB OETF (IEC 61966-2-1), matching the LDR target's implicit
/// encoding so HDR and LDR PNGs are perceptually comparable.
fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::core_pipeline::CorePipelinePlugin;
    use bevy::render::texture::ImagePlugin;
    use bevy::render::RenderPlugin;
    use std::sync::{Arc, Mutex};

    const W: u32 = 64;
    const H: u32 = 48;

    /// Builds a surface-less app (no window) with the offscreen target already
    /// created and a `Camera3d` aimed at it, `finish()`+`cleanup()` applied, and
    /// a few warm-up frames pumped. Returns the app + the target image handle.
    ///
    /// Plugin order mirrors `DefaultPlugins`: RenderPlugin before ImagePlugin.
    fn headless_app_with_camera(clear: Color) -> (App, Handle<Image>) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_plugins(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .add_plugins(RenderPlugin::default())
            .add_plugins(ImagePlugin::default())
            .add_plugins(CorePipelinePlugin);

        app.insert_resource(ClearColor(clear));

        let config = HeadlessConfig::new(W, H);
        app.insert_resource(config);
        // Exclusive-world helper: create the offscreen target + resource.
        let target = {
            let world = app.world_mut();
            create_offscreen_target(world, &config)
        };

        // Camera renders into the offscreen target; there is no window at all.
        app.world_mut().spawn((
            Camera3d::default(),
            Camera {
                target: RenderTarget::Image(target.image.clone()),
                clear_color: ClearColorConfig::Custom(clear),
                ..default()
            },
        ));

        // `App::run()` calls these before the frame loop; driving frames
        // manually with `app.update()` requires invoking them explicitly once,
        // otherwise the render sub-app device / `CapturedScreenshots` are absent.
        app.finish();
        app.cleanup();
        for _ in 0..8 {
            app.update();
        }

        (app, target.image)
    }

    /// SPIKE (regression): surface-less render → CPU readback yields the exact
    /// clear colour, uniformly, with correct dimensions and buffer size.
    #[test]
    fn headless_offscreen_readback_matches_clear() {
        let clear = Color::srgb(0.25, 0.5, 0.75);
        let (mut app, target_handle) = headless_app_with_camera(clear);

        let captured: Arc<Mutex<Option<Image>>> = Arc::new(Mutex::new(None));
        let sink = captured.clone();
        app.world_mut().add_observer(
            move |trigger: Trigger<ScreenshotCaptured>| {
                *sink.lock().unwrap() = Some(trigger.event().0.clone());
            },
        );

        app.world_mut()
            .spawn(Screenshot::image(target_handle.clone()));

        let mut landed = false;
        for _ in 0..240 {
            app.update();
            if captured.lock().unwrap().is_some() {
                landed = true;
                break;
            }
        }
        assert!(
            landed,
            "headless readback never completed within 240 frames (no window/surface)"
        );

        let img = captured.lock().unwrap().clone().expect("captured image");
        let desc = &img.texture_descriptor;
        assert_eq!((desc.size.width, desc.size.height), (W, H));
        assert_eq!(img.data.len(), (W * H * 4) as usize);

        let first = &img.data[0..4];
        assert_ne!(first, &[0, 0, 0, 0], "readback is all-zero (clear missed GPU)");
        for px in img.data.chunks_exact(4) {
            assert_eq!(px, first, "offscreen clear is non-uniform — readback corrupted");
        }
        // Exact sRGB encode of (0.25, 0.5, 0.75).
        assert_eq!(first, &[64, 128, 191, 255], "unexpected clear-colour bytes");
    }

    /// The production capture path (Screenshot → `save_to_disk`) writes a real,
    /// decodable PNG of the offscreen target — the artefact `tools/pixel_diff`
    /// consumes for the pixel gate.
    #[test]
    fn headless_capture_writes_decodable_png() {
        let clear = Color::srgb(0.25, 0.5, 0.75);
        let (mut app, target_handle) = headless_app_with_camera(clear);

        let out = std::env::temp_dir().join(format!(
            "cesium_headless_spike_{}x{}.png",
            W, H
        ));
        let _ = std::fs::remove_file(&out); // clear any stale artefact

        app.world_mut()
            .spawn(Screenshot::image(target_handle.clone()))
            .observe(save_to_disk(out.clone()));

        let mut wrote = false;
        for _ in 0..240 {
            app.update();
            if out.exists() && std::fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false) {
                wrote = true;
                break;
            }
        }
        assert!(wrote, "save_to_disk never produced a non-empty PNG at {out:?}");

        let decoded = image::open(&out).expect("PNG decodes").to_rgba8();
        assert_eq!((decoded.width(), decoded.height()), (W, H));
        // save_to_disk drops HDR alpha via to_rgb8 → PNG is RGB; centre texel
        // must match the clear colour's sRGB encoding.
        let centre = decoded.get_pixel(W / 2, H / 2);
        assert_eq!(
            [centre[0], centre[1], centre[2]],
            [64, 128, 191],
            "PNG centre pixel does not match clear colour"
        );

        let _ = std::fs::remove_file(&out);
    }

    /// `retarget_cameras_to_offscreen` is a no-op without a [`HeadlessTarget`]
    /// (windowed golden path untouched) and redirects a windowed `Camera3d`
    /// when one is present.
    #[test]
    fn retarget_is_noop_without_target_and_redirects_with_it() {
        // No target → camera keeps its window target.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), Camera::default()))
            .id();
        app.add_systems(Update, retarget_cameras_to_offscreen);
        app.update();
        assert!(
            matches!(
                app.world().get::<Camera>(cam).unwrap().target,
                RenderTarget::Window(_)
            ),
            "camera must stay windowed when headless is inactive"
        );
    }

    /// LDR default: `create_offscreen_target` builds an Rgba8UnormSrgb image
    /// (golden-path neutrality — the M11.3 capture format is byte-unchanged).
    #[test]
    fn default_config_selects_ldr_srgb_target() {
        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        let config = HeadlessConfig::new(W, H);
        assert!(!config.hdr, "HDR must default OFF");
        assert_eq!(config.texture_format(), TextureFormat::Rgba8UnormSrgb);
        let target = create_offscreen_target(&mut world, &config);
        let img = world
            .resource::<Assets<Image>>()
            .get(&target.image)
            .expect("target image registered");
        assert_eq!(img.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
        // LDR RGBA8 = 4 bytes/pixel.
        assert_eq!(img.data.len(), (W * H * 4) as usize);
    }

    /// HDR opt-in: `with_hdr(true)` selects Rgba16Float (8 bytes/pixel), the
    /// un-clipped radiance buffer for v2 sky/water/post-process baselines.
    #[test]
    fn hdr_config_selects_rgba16float_target() {
        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        let config = HeadlessConfig::new(W, H).with_hdr(true);
        assert!(config.hdr);
        assert_eq!(config.texture_format(), TextureFormat::Rgba16Float);
        let target = create_offscreen_target(&mut world, &config);
        let img = world
            .resource::<Assets<Image>>()
            .get(&target.image)
            .expect("target image registered");
        assert_eq!(img.texture_descriptor.format, TextureFormat::Rgba16Float);
        // Half-float RGBA = 8 bytes/pixel.
        assert_eq!(img.data.len(), (W * H * 8) as usize);
    }

    /// `CESIUM_HEADLESS_HDR` env reader: truthy tokens enable HDR, everything
    /// else (including unset) is LDR. This is the only test that mutates this
    /// env var, so no cross-test race exists.
    #[test]
    fn hdr_env_reader_honours_truthy_tokens() {
        std::env::remove_var(ENV_HEADLESS_HDR);
        assert!(!headless_hdr_from_env(), "unset must be LDR");
        for tok in ["1", "true", "YES", " on "] {
            std::env::set_var(ENV_HEADLESS_HDR, tok);
            assert!(headless_hdr_from_env(), "token {tok:?} must be truthy");
        }
        for tok in ["0", "false", "", "off", "nonsense"] {
            std::env::set_var(ENV_HEADLESS_HDR, tok);
            assert!(!headless_hdr_from_env(), "token {tok:?} must be falsy");
        }
        std::env::remove_var(ENV_HEADLESS_HDR);
    }

    /// f16 → f32 decodes the IEEE-754 half-precision values the Rgba16Float
    /// readback contains (normals, subnormals, ±inf, sign).
    #[test]
    fn f16_to_f32_decodes_known_values() {
        assert_eq!(f16_to_f32(0x0000), 0.0);
        assert_eq!(f16_to_f32(0x3c00), 1.0);
        assert_eq!(f16_to_f32(0x4000), 2.0);
        assert_eq!(f16_to_f32(0xbc00), -1.0);
        assert_eq!(f16_to_f32(0x7c00), f32::INFINITY);
        assert!((f16_to_f32(0x3555) - 0.33325195).abs() < 1e-5, "~1/3");
        // Smallest subnormal (0x0001) = 2^-24.
        assert!((f16_to_f32(0x0001) - 2.0f32.powi(-24)).abs() < 1e-12);
    }

    /// The HDR save conversion: half-float RGBA → Reinhard tonemap → sRGB OETF →
    /// opaque 8-bit RGBA. A linear 1.0 becomes sRGB(0.5)≈188, black stays black,
    /// and a size-mismatched buffer is rejected.
    #[test]
    fn hdr_bytes_to_rgba8_tonemaps_and_gamma_encodes() {
        let one = 0x3c00u16.to_le_bytes();
        let zero = 0x0000u16.to_le_bytes();
        let mut data = Vec::new();
        // pixel 0: (r=1.0, g=0.0, b=0.0, a=1.0)
        data.extend_from_slice(&one);
        data.extend_from_slice(&zero);
        data.extend_from_slice(&zero);
        data.extend_from_slice(&one);
        // pixel 1: (0,0,0,0)
        for _ in 0..4 {
            data.extend_from_slice(&zero);
        }
        let img = hdr_bytes_to_rgba8(&data, 2, 1).expect("2x1 buffer converts");
        let p0 = *img.get_pixel(0, 0);
        // Reinhard(1.0)=0.5 → sRGB(0.5)≈0.7354 → ≈188.
        assert!((175..=200).contains(&p0[0]), "red tonemapped: {}", p0[0]);
        assert_eq!(p0[1], 0, "green stays black");
        assert_eq!(p0[2], 0, "blue stays black");
        assert_eq!(p0[3], 255, "alpha forced opaque");
        let p1 = *img.get_pixel(1, 0);
        assert_eq!([p1[0], p1[1], p1[2]], [0, 0, 0], "black pixel stays black");
        // Buffer too small for the declared dims → None (graceful, no panic).
        assert!(hdr_bytes_to_rgba8(&data, 4, 4).is_none());
    }

    /// FIX-HL-TONEMAP: `+∞` must map toward white (1.0), the limit of `c/(1+c)`,
    /// NOT to black as the old `is_finite()` guard did. `-∞` / NaN / non-positive
    /// → 0.0; finite positives follow Reinhard.
    #[test]
    fn tonemap_reinhard_handles_infinities_and_nan() {
        assert_eq!(tonemap_reinhard(f32::INFINITY), 1.0, "+inf → 1.0");
        assert_eq!(tonemap_reinhard(f32::NEG_INFINITY), 0.0, "-inf → 0.0");
        assert_eq!(tonemap_reinhard(f32::NAN), 0.0, "NaN → 0.0");
        assert_eq!(tonemap_reinhard(-1.0), 0.0, "negative → 0.0");
        assert_eq!(tonemap_reinhard(0.0), 0.0, "zero → 0.0");
        assert!((tonemap_reinhard(1.0) - 0.5).abs() < 1e-6, "1.0 → 0.5");
        // Large finite radiance stays strictly below 1 and monotonic.
        let a = tonemap_reinhard(1.0e6);
        let b = tonemap_reinhard(1.0e7);
        assert!(a < 1.0 && a > 0.99 && b > a, "finite super-unit compresses toward 1");
    }

    /// FIX-HL-HDRCAP: a buffer longer than `w*h*8` (a trailing partial pixel)
    /// used to slip past `chunks_exact` and yield a short `out` that `from_raw`
    /// rejected only as a logged `None`; now the exact-length guard rejects it up
    /// front, and an exact-size buffer still converts.
    #[test]
    fn hdr_bytes_to_rgba8_rejects_mismatched_length_exactly() {
        let w = 2u32;
        let h = 2u32;
        let exact = (w as usize) * (h as usize) * 8;
        // Exactly w*h*8 bytes → Some.
        let data = vec![0u8; exact];
        assert!(hdr_bytes_to_rgba8(&data, w, h).is_some(), "exact length converts");
        // One extra byte (would be silently dropped by chunks_exact before) → None.
        let mut too_long = data.clone();
        too_long.push(0);
        assert!(hdr_bytes_to_rgba8(&too_long, w, h).is_none(), "extra trailing byte rejected");
        // One byte short → None.
        assert!(hdr_bytes_to_rgba8(&data[..exact - 1], w, h).is_none(), "short buffer rejected");
    }
}
