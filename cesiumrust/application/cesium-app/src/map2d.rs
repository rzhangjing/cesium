//! 2D flat-map mode + the 2D/3D switch button (see plan `cesium-app_2D_地图模式`).
//!
//! ## Scope of this module (P1 skeleton)
//! A single [`MapMode`] resource decides whether the viewer runs the proven 3D
//! globe path or a new flat Geographic (equirectangular) map. In 2D the imagery
//! tiles will (P2) be laid on the XY plane via `x = R·lon, y = R·lat`; for P1 a
//! procedurally generated lat/lon grid stands in so pan / zoom / switch can be
//! validated without touching the tile pipeline.
//!
//! ## Determinism contract (hard requirement)
//! [`MapMode`] defaults to [`MapMode::ThreeD`]. The 3D orbit systems are gated
//! with `run_if(map_is_3d)` so in the default mode they run exactly as before —
//! byte-for-byte. This plugin is only registered on the *windowed* branch of
//! `main`, never under `CESIUM_HEADLESS`, so the offscreen capture path (v0 /
//! FIXED_CAMERA / camera-script golden tests) is untouched: no second camera, no
//! UI, no extra render pass.

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::RenderLayers;

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::orbit_camera::OrbitCamera;

/// `f64` copies of the constants the tile math needs (kept in double to avoid
/// visible column drift at deep zoom levels).
const PI6: f64 = std::f64::consts::PI;
const TAU6: f64 = std::f64::consts::TAU;

// ── Geographic world scale ──────────────────────────────────────────────────
// 1 render unit == the globe radius, so the equirectangular plane spans
// x ∈ [-π, π] (longitude) and y ∈ [-π/2, π/2] (latitude). The texture is one
// full world wide, so wrapping the camera centre by WORLD_W keeps the grid
// seamless → "infinite" longitude dragging.
const WORLD_W: f32 = std::f32::consts::TAU; // 2π, full-360° period in x
const LAT_MAX: f32 = std::f32::consts::FRAC_PI_2; // π/2, ±90° in y

/// Zoom (pixels per world unit) limits. `ZOOM_MIN` keeps the whole world plus
/// margin inside the oversized base quad so no empty edges show; `ZOOM_MAX` is a
/// sanity ceiling.
const ZOOM_MIN: f32 = 110.0;
const ZOOM_MAX: f32 = 8000.0;
// Default so the map fills the frame vertically (window height ≈ π·zoom) rather
// than floating as a small band with empty margins.
const ZOOM_DEFAULT: f32 = 240.0;

/// Camera height above the plane (render units). Only needs to sit inside the
/// orthographic near/far band; the projection is orthographic so this is
/// visually irrelevant beyond depth ordering.
const CAM_Z: f32 = 100.0;

// ── P2 imagery-tile tuning ────────────────────────────────────────────────
// Tile level is picked so one 256px tile spans ~256 screen px (native res).
const TILE_PX_TARGET: f32 = 256.0;
const TILE_Z_MIN: i32 = 1;
const TILE_Z_MAX: i32 = 19;
/// Tiles float just above the placeholder grid (z = 0) so they win depth tests.
const TILE_Z_ELEV: f32 = 0.5;
/// Background worker threads downloading 2D imagery tiles.
const MAP2D_DOWNLOAD_THREADS: usize = 4;

// ── 2D/3D switch palette ──────────────────────────────────────────────────
// `Color::srgba` is a `const fn` in bevy_color 0.15, so the whole theme can be
// compile-time data. Dark "glass" shell + azure accent for the active segment.
const SW_GLASS: Color = Color::srgba(0.07, 0.10, 0.15, 0.80);
const SW_GLASS_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.16);
const SW_ACCENT: Color = Color::srgba(0.16, 0.50, 0.86, 1.0);
const SW_ACCENT_PRESSED: Color = Color::srgba(0.11, 0.37, 0.66, 1.0);
const SW_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.14);
const SW_HOVER_PRESSED: Color = Color::srgba(1.0, 1.0, 1.0, 0.26);
const SW_ACTIVE_TEXT: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
const SW_IDLE_TEXT: Color = Color::srgba(0.72, 0.78, 0.86, 1.0);

/// Which projection the viewer is in. `ThreeD` is the default and the golden
/// path; `TwoD` is the flat map.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MapMode {
    #[default]
    ThreeD,
    TwoD,
}

/// System run-condition: true while the flat map is active.
pub fn map_is_2d(mode: Res<MapMode>) -> bool {
    matches!(*mode, MapMode::TwoD)
}

/// System run-condition: true while the 3D globe is active (the default). Used
/// by `OrbitCameraPlugin` to gate its systems without altering 3D behaviour.
pub fn map_is_3d(mode: Res<MapMode>) -> bool {
    matches!(*mode, MapMode::ThreeD)
}

/// Marker for the 2D orthographic camera (kept separate from [`OrbitCamera`] so
/// the two never fight over the same query).
#[derive(Component)]
struct Map2dCamera;

/// Marker for the base grid quad, hidden in 3D mode.
#[derive(Component)]
struct Map2dPlane;

/// One cell of the 2D/3D segmented switch. Remembers which mode it selects and
/// its own label entity, so a single system can restyle fill + text together.
#[derive(Component)]
struct ModeSegment {
    mode: MapMode,
    label: Entity,
}

/// Marker for the switch's outer pill container. Also a UI root, so it needs an
/// explicit [`TargetCamera`] (see [`sync_ui_target_camera`]).
#[derive(Component)]
struct ModeSwitchRoot;

/// Marker for the bottom-left readout container — the other UI root.
#[derive(Component)]
struct ReadoutRoot;

/// Component carrying the flat-map camera state (Geographic world coords).
#[derive(Component)]
struct Map2dCam {
    /// Camera centre in Geographic world units (x = R·lon, y = R·lat).
    center: Vec2,
    /// Pixels per world unit; the orthographic scale is `1 / zoom`.
    zoom: f32,
}

/// Live cursor readout text (lon°/lat°).
#[derive(Component)]
struct CoordText;

/// Live zoom-level text.
#[derive(Component)]
struct LevelText;

// ── P2: real imagery tiles ─────────────────────────────────────────────────

/// Marker for a per-tile imagery quad, keyed by its canonical Web-Mercator
/// `(x, y, z)`. Placement uses a raw (possibly out-of-range) column so the map
/// wraps infinitely in longitude while the texture is fetched by canonical col.
#[derive(Component)]
struct Map2dTile {
    key: (u32, u32, u32),
}

/// A decoded tile image shipped from a worker thread back to the main world.
struct Map2dTileImg {
    x: u32,
    y: u32,
    z: u32,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

/// Cross-frame state for the 2D imagery layer: a small worker pool downloads
/// Bing Aerial tiles (or reads `OFFLINE_IMAGERY_ROOT` from disk), decoded RGBA
/// is cached by canonical key, and one quad entity is maintained per visible
/// raw column/row. Kept fully separate from the 3D `TileManager` so the golden
/// globe path is never entangled with 2D panning.
#[derive(Resource)]
struct Map2dTiler {
    job_tx: mpsc::Sender<(u32, u32, u32)>,
    rx: Mutex<mpsc::Receiver<Map2dTileImg>>,
    cache: HashMap<(u32, u32, u32), Handle<Image>>,
    live: HashMap<(i64, u32), Entity>,
    in_flight: HashSet<(u32, u32, u32)>,
    unit_quad: Option<Handle<Mesh>>,
    cur_z: u32,
}

impl Default for Map2dTiler {
    fn default() -> Self {
        let (job_tx, job_rx) = mpsc::channel::<(u32, u32, u32)>();
        let (res_tx, res_rx) = mpsc::channel::<Map2dTileImg>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        for _ in 0..MAP2D_DOWNLOAD_THREADS {
            let jr = job_rx.clone();
            let tx = res_tx.clone();
            std::thread::spawn(move || map2d_worker(jr, tx));
        }
        Self {
            job_tx,
            rx: Mutex::new(res_rx),
            cache: HashMap::new(),
            live: HashMap::new(),
            in_flight: HashSet::new(),
            unit_quad: None,
            cur_z: 0,
        }
    }
}

/// Plugin wiring the 2D map. Registered only on the windowed branch of `main`.
pub struct Map2dPlugin;

impl Plugin for Map2dPlugin {
    fn build(&self, app: &mut App) {
        // MapMode itself is initialised by OrbitCameraPlugin (always present);
        // here we only add the 2D-specific systems and startup spawns.
        app.add_systems(
            Startup,
            (spawn_map2d_camera, spawn_map2d_plane, build_mode_ui).chain(),
        )
        .init_resource::<Map2dTiler>()
        .add_systems(
            Update,
            (
                sync_camera_by_mode,
                sync_ui_target_camera,
                (map2d_pan_system, map2d_zoom_system).run_if(map_is_2d),
                apply_map2d_cam.run_if(map_is_2d),
                update_map2d_tiles.run_if(map_is_2d),
                update_mode_segments,
                update_readout,
            )
                .chain(),
        )
        .add_systems(Update, on_mode_segment_click);
    }
}

// ── Spawns ──────────────────────────────────────────────────────────────────

fn spawn_map2d_camera(mut commands: Commands) {
    // Orthographic top-down over the XY plane. Identity rotation looks along -Z,
    // screen-right = +X (east), screen-up = +Y (north) — exactly the Geographic
    // layout. `ScalingMode::WindowSize` (default) makes `scale` == world units
    // per pixel, so `scale = 1 / zoom`. `order` starts low; sync promotes it in 2D.
    let projection = OrthographicProjection {
        scale: 1.0 / ZOOM_DEFAULT,
        near: -1000.0,
        far: 1000.0,
        ..OrthographicProjection::default_3d()
    };
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        // The workspace disables `tonemapping_luts`, so `Camera3d`'s default
        // (`TonyMcMapFace`) logs an error every frame. An unlit flat map needs
        // no tonemapping at all.
        Tonemapping::None,
        // Layer 1 = the flat grid, layer 2 = shared UI. It never sees the
        // globe (layer 0), so switching modes can't leave the 3D scene showing.
        RenderLayers::from_layers(&[1, 2]),
        Projection::Orthographic(projection),
        Map2dCamera,
        Map2dCam {
            center: Vec2::ZERO,
            zoom: ZOOM_DEFAULT,
        },
        Transform::from_xyz(0.0, 0.0, CAM_Z),
    ));
}

fn spawn_map2d_plane(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tex = build_grid_texture(&mut images);
    let mesh = build_grid_mesh();
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(tex),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Map2dPlane,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(mat),
        // Layer 1: only the 2D camera (which is `is_active` in 2D mode) sees it.
        RenderLayers::layer(1),
        Visibility::Visible,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn build_mode_ui(mut commands: Commands) {
    // ── Bottom-right 2D/3D segmented switch ─────────────────────────────────
    // A pill-shaped glass shell holding two rounded cells; the cell matching the
    // CURRENT mode carries the accent fill, so the control always shows where you
    // are (the old "[ 2D ]" toggle advertised the *other* mode and read as noise).
    // Hover/press feedback lives in `update_mode_segments`.
    let container = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(18.0),
                bottom: Val::Px(18.0),
                column_gap: Val::Px(3.0),
                padding: UiRect::all(Val::Px(4.0)),
                // A 1px border must be sized before `BorderColor` paints anything.
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(SW_GLASS),
            BorderColor(SW_GLASS_BORDER),
            // Half the container's 42px height → a true pill.
            BorderRadius::all(Val::Px(21.0)),
            BoxShadow {
                color: Color::srgba(0.0, 0.0, 0.0, 0.45),
                x_offset: Val::Px(0.0),
                y_offset: Val::Px(4.0),
                spread_radius: Val::Px(0.0),
                blur_radius: Val::Px(12.0),
            },
            // Layer 2: shared UI, seen by both the 3D and 2D cameras.
            RenderLayers::layer(2),
            ModeSwitchRoot,
        ))
        .id();

    spawn_mode_segment(&mut commands, container, MapMode::ThreeD);
    spawn_mode_segment(&mut commands, container, MapMode::TwoD);

    // Bottom-left readout block (coords + zoom level), stacked vertically.
    let readout = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            RenderLayers::layer(2),
            ReadoutRoot,
        ))
        .id();
    commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.9, 0.95)),
            RenderLayers::layer(2),
            CoordText,
        ))
        .set_parent(readout);
    commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.9, 0.95)),
            RenderLayers::layer(2),
            LevelText,
        ))
        .set_parent(readout);
}

/// One pill cell of the switch: `Button` + `Interaction` on the node, its label
/// as a child. The label entity is parked on [`ModeSegment`] so the styling
/// system can reach it without a `Children` walk every frame.
fn spawn_mode_segment(commands: &mut Commands, parent: Entity, mode: MapMode) {
    // ASCII only: the bundled FiraSans has no CJK/`°` glyphs (renders tofu).
    let title = match mode {
        MapMode::ThreeD => "3D",
        MapMode::TwoD => "2D",
    };
    let label = commands
        .spawn((
            Text::new(title),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(SW_IDLE_TEXT),
            RenderLayers::layer(2),
        ))
        .id();
    let cell = commands
        .spawn((
            Button,
            Interaction::default(),
            Node {
                width: Val::Px(60.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // Inactive cells are see-through; the shell's glass shows through.
            BackgroundColor(Color::NONE),
            BorderRadius::all(Val::Px(16.0)),
            RenderLayers::layer(2),
            ModeSegment { mode, label },
        ))
        .id();
    commands.entity(cell).set_parent(parent);
    commands.entity(label).set_parent(cell);
}

// ── Mode sync / toggling ────────────────────────────────────────────────────

/// Keep exactly one camera live for the active mode. `RenderLayers` already
/// isolate what each camera sees, so we simply enable the matching one and
/// disable the other — no double render, no clear-order ambiguity, and the
/// pre-existing 3D scene entities are never touched.
fn sync_camera_by_mode(
    mode: Res<MapMode>,
    mut cams: Query<(&mut Camera, Has<OrbitCamera>, Has<Map2dCamera>)>,
) {
    let two_d = matches!(*mode, MapMode::TwoD);
    for (mut c, orbit, flat) in cams.iter_mut() {
        if orbit {
            c.is_active = !two_d;
        } else if flat {
            c.is_active = two_d;
        }
    }
}

/// Pin every UI root to the camera that is actually active for the current mode.
////// bevy_ui binds a UI tree to ONE camera: `TargetCamera`, falling back to
/// `DefaultUiCamera` — which is only resolvable when the world has a single
/// camera. `TargetCamera`'s own doc says as much: *"Optional if there is only
/// one camera in the world. Required otherwise."* We have at least two (orbit +
/// flat), so the binding is ambiguous and lands on whichever camera the query
/// happens to yield. Extraction drops `DefaultCameraView` for **inactive**
/// cameras, so if that pick is the mode's dormant one the whole control simply
/// stops drawing — which is exactly why the switch could vanish on entering 2D.
///
/// Layout is window-derived and identical for both cameras, so re-pointing the
/// roots is safe. The write is guarded on "already correct" to avoid a change
/// tick (and a child re-propagation) on every idle frame.
fn sync_ui_target_camera(
    mode: Res<MapMode>,
    cams: Query<(Entity, Has<OrbitCamera>, Has<Map2dCamera>), With<Camera>>,
    roots: Query<Entity, Or<(With<ModeSwitchRoot>, With<ReadoutRoot>)>>,
    bound: Query<&TargetCamera>,
    mut commands: Commands,
) {
    let two_d = matches!(*mode, MapMode::TwoD);
    let Some(active) = cams
        .iter()
        .find_map(|(e, orbit, flat)| ((orbit && !two_d) || (flat && two_d)).then_some(e))
    else {
        // No camera owns this mode yet (e.g. the orbit camera spawns late) —
        // retry next frame rather than pinning the UI to nothing.
        return;
    };
    for root in roots.iter() {
        let already = matches!(bound.get(root), Ok(t) if t.0 == active);
        if !already {
            commands.entity(root).insert(TargetCamera(active));
        }
    }
}

/// Clicking a segment selects that mode. Deliberately *not* a toggle: clicking
/// the already-active cell is a no-op, so the switch can never flip out from
/// under a user who meant to press the other one.
fn on_mode_segment_click(
    mut mode: ResMut<MapMode>,
    cells: Query<(&ModeSegment, &Interaction), Changed<Interaction>>,
) {
    for (seg, i) in &cells {
        if *i == Interaction::Pressed && *mode != seg.mode {
            *mode = seg.mode;
            info!("[map2d] switched to {:?}", *mode);
        }
    }
}

/// Paint the switch: the cell for the active [`MapMode`] carries the accent
/// fill, every cell answers hover/press with a visible change. Values are
/// compared before writing so idle frames raise no UI change ticks.
fn update_mode_segments(
    mode: Res<MapMode>,
    mut cells: Query<(&ModeSegment, &Interaction, &mut BackgroundColor)>,
    mut labels: Query<&mut TextColor>,
) {
    for (seg, interaction, mut bg) in cells.iter_mut() {
        let active = *mode == seg.mode;
        let hovered = matches!(*interaction, Interaction::Hovered);
        let pressed = matches!(*interaction, Interaction::Pressed);

        let want_bg = if active {
            if pressed {
                SW_ACCENT_PRESSED
            } else {
                SW_ACCENT
            }
        } else if pressed {
            SW_HOVER_PRESSED
        } else if hovered {
            SW_HOVER
        } else {
            Color::NONE
        };
        if bg.0 != want_bg {
            bg.0 = want_bg;
        }

        if let Ok(mut tc) = labels.get_mut(seg.label) {
            // The active cell and any hovered cell read bright; only a resting
            // inactive cell is dimmed.
            let want_tc = if active || hovered {
                SW_ACTIVE_TEXT
            } else {
                SW_IDLE_TEXT
            };
            if tc.0 != want_tc {
                tc.0 = want_tc;
            }
        }
    }
}

// ── 2D camera interaction ───────────────────────────────────────────────────

/// Left-drag pans the flat map; the geographic world point grabbed under the
/// cursor stays glued to it, and the centre wraps in longitude (infinite drag).
/// Only mutates [`Map2dCam`]; [`apply_map2d_cam`] writes it to the camera.
fn map2d_pan_system(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    windows: Query<&Window>,
    read: Query<(&Camera, &GlobalTransform), With<Map2dCamera>>,
    mut write: Query<&mut Map2dCam, With<Map2dCamera>>,
) {
    if !mouse.pressed(MouseButton::Left) {
        for _ in motion.read() {}
        return;
    }
    let Ok(win) = windows.get_single() else { return };
    let Some(cursor) = win.cursor_position() else {
        for _ in motion.read() {}
        return;
    };
    let Ok((cam, ct)) = read.get_single() else { return };

    // Accumulate this frame's motion (usually one event, coalesce any burst).
    let mut delta = Vec2::ZERO;
    for m in motion.read() {
        delta += m.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    // Grab-the-map: the world point under the previous cursor should land under
    // the current one. Both samples are absolute world, so their difference is
    // the exact world-space pan for this frame's pixel delta.
    let prev = cursor - delta;
    let (Ok(w_now), Ok(w_prev)) = (
        cam.viewport_to_world_2d(ct, cursor),
        cam.viewport_to_world_2d(ct, prev),
    ) else {
        return;
    };
    let shift = w_prev - w_now;

    if let Ok(mut mc) = write.get_single_mut() {
        let mut center = mc.center + shift;
        center.x = wrap_x(center.x);
        center = clamp_center_y(center, mc.zoom, win.height());
        mc.center = center;
    }
}

/// Wheel zooms toward the cursor: the geographic point under the pointer is held
/// fixed across the zoom step (CesiumJS-style zoom-to-cursor).
fn map2d_zoom_system(
    mut wheel: EventReader<MouseWheel>,
    windows: Query<&Window>,
    mut cams: Query<(&Camera, &GlobalTransform, &mut Map2dCam), With<Map2dCamera>>,
) {
    let mut scroll = 0.0f32;
    for w in wheel.read() {
        scroll += w.y;
    }
    if scroll == 0.0 {
        return;
    }
    let Ok(win) = windows.get_single() else { return };
    let Some(cursor) = win.cursor_position() else { return };
    let Ok((cam, ct, mut mc)) = cams.get_single_mut() else { return };

    // Absolute geographic world point under the cursor at the CURRENT zoom.
    let Ok(g) = cam.viewport_to_world_2d(ct, cursor) else {
        return;
    };

    let factor = (1.0 + 0.3_f32.min(scroll.abs()) * scroll.signum()).max(0.2);
    let zoom_min = min_zoom_for(win.height());
    let new_zoom = (mc.zoom * factor).clamp(zoom_min, ZOOM_MAX);
    // scale ∝ 1/zoom, so to keep `g` pinned under the cursor the centre moves
    // a fraction (1 - old/new) of the way from the centre to `g`.
    let ratio = 1.0 - mc.zoom / new_zoom;
    let mut center = mc.center + (g - mc.center) * ratio;
    center.x = wrap_x(center.x);
    center = clamp_center_y(center, new_zoom, win.height());

    mc.zoom = new_zoom;
    mc.center = center;
}

/// Single writer: push [`Map2dCam`] centre/zoom onto the live camera transform +
/// orthographic scale (`1 / zoom`). Kept separate from the input systems so they
/// can read `Camera`/`GlobalTransform` while mapping the cursor without also
/// holding them mutably.
fn apply_map2d_cam(
    mut q: Query<(&mut Transform, &mut Projection, &Map2dCam), With<Map2dCamera>>,
) {
    for (mut tf, mut proj, mc) in q.iter_mut() {
        tf.translation = Vec3::new(mc.center.x, mc.center.y, CAM_Z);
        if let Projection::Orthographic(p) = proj.as_mut() {
            p.scale = 1.0 / mc.zoom;
        }
    }
}

// ── Readout ─────────────────────────────────────────────────────────────────

fn update_readout(
    mode: Res<MapMode>,
    windows: Query<&Window>,
    read: Query<(&Camera, &GlobalTransform), With<Map2dCamera>>,
    mut texts: Query<(&mut Text, Has<CoordText>, Has<LevelText>)>,
    cams: Query<&Map2dCam, With<Map2dCamera>>,
) {
    if !matches!(*mode, MapMode::TwoD) {
        // Clear the readout once on the 2D→3D transition so stale lon/lat text
        // never lingers over the globe. `is_changed` keeps this to one frame.
        // Scoped to the readout labels: an unfiltered sweep here used to blank
        // the mode-switch captions too, leaving a textless button in 3D.
        if mode.is_changed() {
            for (mut t, is_coord, is_level) in texts.iter_mut() {
                if (is_coord || is_level) && !t.is_empty() {
                    t.0.clear();
                }
            }
        }
        return;
    }
    let Ok(win) = windows.get_single() else { return };
    let Ok((cam, ct)) = read.get_single() else { return };

    // A single `&mut Text` query (with `Has<..>` role tags) avoids the ECS
    // conflict two separate `&mut Text` queries would raise.
    let level = cams.get_single().ok().map(|mc| approx_zoom_level(mc.zoom));
    let lonlat = win.cursor_position().and_then(|cursor| {
        cam.viewport_to_world_2d(ct, cursor).ok().map(|world| {
            let lon = wrap_lon_deg(world.x.to_degrees());
            let lat = world.y.to_degrees().clamp(-90.0, 90.0);
            (lon, lat)
        })
    });

    for (mut t, is_coord, is_level) in texts.iter_mut() {
        if is_coord {
            if let Some((lon, lat)) = lonlat {
                // ASCII only: the bundled FiraSans has no `°`/`≈` glyph (renders tofu).
                t.0 = format!("lon {lon:.4} deg   lat {lat:.4} deg");
            }
        } else if is_level {
            if let Some(z) = level {
                t.0 = format!("zoom level ~ {z}");
            }
        }
    }
}

// ── Pure helpers (unit-tested) ──────────────────────────────────────────────

/// Wrap a Geographic x (longitude·R, R = 1) into [-π, π) so dragging past the
/// antimeridian continues seamlessly.
fn wrap_x(x: f32) -> f32 {
    let mut v = x;
    while v > std::f32::consts::PI {
        v -= WORLD_W;
    }
    while v < -std::f32::consts::PI {
        v += WORLD_W;
    }
    v
}

/// Smallest zoom (px / world-unit) at which the world's full latitude band
/// (±90°, height `2·LAT_MAX = π`) still covers a `canvas_h`-pixel-tall viewport.
/// Zooming out past this would leave empty margins above/below the map, so it is
/// the effective lower bound (Google-maps-style "whole world fills the frame").
fn min_zoom_for(canvas_h: f32) -> f32 {
    (canvas_h / (2.0 * LAT_MAX)).max(ZOOM_MIN)
}

/// Clamp the camera centre so the imagery always fills the viewport vertically:
/// when the whole world fits (zoomed out) pin it to the equator; otherwise keep
/// the visible band inside ±90° so a pole edge never reveals empty background.
/// Longitude is left free (it wraps infinitely via [`wrap_x`]).
fn clamp_center_y(center: Vec2, zoom: f32, canvas_h: f32) -> Vec2 {
    let half_h = canvas_h * 0.5 / zoom;
    let cy = if half_h >= LAT_MAX {
        0.0
    } else {
        center.y.clamp(-LAT_MAX + half_h, LAT_MAX - half_h)
    };
    Vec2::new(center.x, cy)
}

/// Normalise a longitude in degrees into [-180, 180].
fn wrap_lon_deg(mut lon: f32) -> f32 {
    while lon > 180.0 {
        lon -= 360.0;
    }
    while lon < -180.0 {
        lon += 360.0;
    }
    lon
}

/// Inverse of the tile-size rule: the Web Mercator level at which one tile is
/// roughly `MAX_TILE_SCREEN_PX` wide, given the current zoom (px / world-unit,
/// where one world is `WORLD_W` units wide).
fn approx_zoom_level(zoom: f32) -> i32 {
    const MAX_TILE_SCREEN_PX: f32 = 288.0;
    // tile_px(z) = zoom * WORLD_W / 2^z == MAX_TILE_SCREEN_PX  →  z.
    let z = (zoom * WORLD_W / MAX_TILE_SCREEN_PX).log2();
    z.max(0.0).round() as i32
}

// ── Base grid (P1 placeholder) ──────────────────────────────────────────────

/// One full-world RGBA texture with an equirectangular lat/lon grid. Address
/// mode U = Repeat gives seamless longitude tiling; V = ClampToEdge avoids
/// repeating the (data-free) polar bands.
fn build_grid_texture(images: &mut Assets<Image>) -> Handle<Image> {
    const W: u32 = 768;
    const H: u32 = 384;
    let sea = [18u8, 32, 48, 255];
    let grid = [70u8, 110, 150, 255];
    let axis = [150u8, 190, 220, 255];

    let mut px = vec![0u8; (W * H * 4) as usize];
    // 24 meridians (15° apart), 12 parallels (15° apart).
    let col_lines = 24u32;
    let row_lines = 12u32;
    for y in 0..H {
        for x in 0..W {
            let m = (x * col_lines) / W;
            let p = (y * row_lines) / H;
            let on_meridian = (x - m * (W / col_lines)) < 1;
            let on_parallel = (y - p * (H / row_lines)) < 1;
            let is_axis =
                (x as i32 - (W / 2) as i32).abs() < 1 || (y as i32 - (H / 2) as i32).abs() < 1;
            let c = if is_axis {
                axis
            } else if on_meridian || on_parallel {
                grid
            } else {
                sea
            };
            let i = ((y * W + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&c);
        }
    }

    let mut img = Image::new(
        Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        px,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        // Repeat in both axes: the placeholder grid tiles seamlessly so the
        // oversized quad always fills the viewport (no empty pole margins).
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..default()
    });
    images.add(img)
}

/// A single oversized quad on the XY plane spanning x ∈ [-4π, 4π],
/// y ∈ [-3π, 3π] — big enough to fill the viewport at every allowed zoom. UV:
/// u = x / WORLD_W (one tile per 360°), v = y / π + 0.5 (one tile per 180°), so
/// the repeating texture reads as a consistent 15° lat/lon grid. Built as two
/// triangles with position + normal + uv.
fn build_grid_mesh() -> Mesh {
    let half_x = 4.0 * std::f32::consts::PI;
    let half_y = 3.0 * std::f32::consts::PI;
    // corners: 0 TL, 1 TR, 2 BL, 3 BR
    let positions = [
        [-half_x, half_y, 0.0],
        [half_x, half_y, 0.0],
        [-half_x, -half_y, 0.0],
        [half_x, -half_y, 0.0],
    ];
    let uvs = positions.map(|p| [p[0] / WORLD_W, p[1] / std::f32::consts::PI + 0.5]);
    let normals = [[0.0, 0.0, 1.0]; 4];
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    // Winding for a +Z-facing quad seen from a camera looking down -Z: the
    // triangles must be CCW as seen from +Z (screen-right = +X, up = +Y), so
    // the front faces the camera and survives back-face culling.
    mesh.insert_indices(Indices::U16(vec![0, 2, 3, 0, 3, 1]));
    mesh
}

// ── P2: imagery-tile layer ─────────────────────────────────────────────────

/// Pick the Web-Mercator tile level so one native 256px tile covers about
/// `TILE_PX_TARGET` screen pixels at the current `zoom` (px / world-unit).
fn tile_zoom_for(zoom: f32) -> u32 {
    let z = (f64::from(zoom) * TAU6 / TILE_PX_TARGET as f64).log2().round() as i32;
    z.clamp(TILE_Z_MIN, TILE_Z_MAX) as u32
}

/// Geographic latitude (radians) of the north edge of Mercator row `row`.
fn row_to_lat(row: f64, n: f64) -> f64 {
    (PI6 * (1.0 - 2.0 * row / n)).sinh().atan()
}

/// Inverse of [`row_to_lat`]: the (fractional) Mercator row of latitude `lat`.
/// Clamped just inside the poles so `tan`/`asinh` never blow up to infinity.
fn lat_to_row(lat: f64, n: f64) -> f64 {
    let l = lat.clamp(-1.4844, 1.4844);
    (1.0 - l.tan().asinh() / PI6) * 0.5 * n
}

/// Centre + size (world units) of the Geographic rectangle for raw column
/// `col`, Mercator `row`, at level denominator `n`. `col` may be outside
/// `[0, n)`; the returned x then sits beyond ±π, which is exactly what makes
/// the longitude wrap look continuous.
fn tile_rect(col: f64, row: f64, n: f64) -> (f32, f32, f32, f32) {
    let x0 = col / n * TAU6 - PI6;
    let x1 = (col + 1.0) / n * TAU6 - PI6;
    let yn = row_to_lat(row, n);
    let ys = row_to_lat(row + 1.0, n);
    (
        ((x0 + x1) * 0.5) as f32,
        ((yn + ys) * 0.5) as f32,
        (x1 - x0) as f32,
        (yn - ys) as f32,
    )
}

/// Convert XYZ tile coords to a Bing Maps quadkey.
fn quadkey(x: u32, y: u32, level: u32) -> String {
    let mut s = String::with_capacity(level as usize);
    for i in (0..level).rev() {
        let m = 1u32 << i;
        let mut d = 0u8;
        if x & m != 0 {
            d |= 1;
        }
        if y & m != 0 {
            d |= 2;
        }
        s.push((b'0' + d) as char);
    }
    s
}

/// A unit quad in the XY plane (local extent [-0.5, 0.5]), UV filling [0, 1] so
/// v=0 is the tile's north edge (row 0 of the downloaded image). Same winding
/// as [`build_grid_mesh`] so the front face points +Z at the top-down camera.
fn build_unit_quad() -> Mesh {
    let positions = [
        [-0.5, 0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    let normals = [[0.0, 0.0, 1.0]; 4];
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(Indices::U16(vec![0, 2, 3, 0, 3, 1]));
    mesh
}

/// One desired tile: `(placement_key, (canonical_key, rect))` where the rect is
/// `(centre_x, centre_y, width, height)` in Geographic world units.
type DesiredTile = HashMap<(i64, u32), ((u32, u32, u32), (f32, f32, f32, f32))>;

/// Maintain the flat-map imagery layer for the current viewport: reconcile the
/// set of visible tiles (spawn/despawn quads), kick off downloads for tiles we
/// don't yet have, and paint cached textures onto entities as they land.
///
/// The placeholder grid underneath (layer 1) always covers the frame, so tiles
/// start `Hidden` and are revealed only once their texture is ready — no white
/// flash, no holes while streaming.
#[allow(clippy::too_many_arguments)]
fn update_map2d_tiles(
    mut commands: Commands,
    mut tiler: ResMut<Map2dTiler>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
    cams: Query<(&Camera, &GlobalTransform, &Map2dCam), With<Map2dCamera>>,
    mut live: Query<(&Map2dTile, &MeshMaterial3d<StandardMaterial>, &mut Visibility)>,
) {
    // 1. Drain finished downloads into the texture cache.
    let drained: Vec<Map2dTileImg> = {
        let rx = tiler.rx.lock().unwrap();
        let mut v = Vec::new();
        while let Ok(t) = rx.try_recv() {
            v.push(t);
        }
        v
    };
    for t in drained {
        tiler.in_flight.remove(&(t.x, t.y, t.z));
        let img = Image::new(
            Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            t.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        tiler.cache.insert((t.x, t.y, t.z), images.add(img));
    }

    // 2. Read the viewport as a Geographic world rectangle.
    let Ok(win) = windows.get_single() else { return };
    let (Ok(cam), Ok(ct), Ok(mc)) = (
        cams.get_single().map(|c| c.0),
        cams.get_single().map(|c| c.1),
        cams.get_single().map(|c| c.2),
    ) else {
        return;
    };
    let Ok(tl) = cam.viewport_to_world_2d(ct, Vec2::ZERO) else { return };
    let Ok(br) = cam.viewport_to_world_2d(ct, Vec2::new(win.width(), win.height())) else {
        return;
    };

    // 3. Resolve tile level; a level change invalidates every quad.
    let z = tile_zoom_for(mc.zoom);
    if z != tiler.cur_z {
        for (_, e) in tiler.live.drain() {
            commands.entity(e).despawn();
        }
        tiler.cur_z = z;
    }
    let ni = 1i64 << z;
    let n = ni as f64;

    // 4. Enumerate the visible (raw-col, row) rectangle. Columns run past the
    //    ±π edges and wrap via `rem_euclid`; rows are clamped to [0, n).
    let col_start = ((f64::from(tl.x) + PI6) / TAU6 * n).floor() as i64;
    let col_end = ((f64::from(br.x) + PI6) / TAU6 * n).floor() as i64;
    let row_start = lat_to_row(f64::from(tl.y), n).floor().max(0.0) as i64;
    let row_end = lat_to_row(f64::from(br.y), n)
        .ceil()
        .min((ni - 1) as f64) as i64;

    let mut desired: DesiredTile = HashMap::new();
    for row in row_start..=row_end {
        for col in col_start..=col_end {
            let canon = (col.rem_euclid(ni) as u32, row as u32, z);
            let rect = tile_rect(col as f64, row as f64, n);
            desired.insert((col, row as u32), (canon, rect));
        }
    }

    // 5. Despawn tiles that left the viewport.
    let gone: Vec<(i64, u32)> = tiler
        .live
        .keys()
        .filter(|pk| !desired.contains_key(pk))
        .copied()
        .collect();
    for pk in gone {
        if let Some(e) = tiler.live.remove(&pk) {
            commands.entity(e).despawn();
        }
    }

    // 6. Ensure the shared unit-quad mesh exists.
    if tiler.unit_quad.is_none() {
        tiler.unit_quad = Some(meshes.add(build_unit_quad()));
    }
    let quad = tiler.unit_quad.clone().unwrap();

    // 7. Spawn new tiles (textured straight from cache when possible).
    for (pk, (canon, (cx, cy, w, h))) in &desired {
        if tiler.live.contains_key(pk) {
            continue;
        }
        let tex = tiler.cache.get(canon).cloned();
        let mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: tex.clone(),
            unlit: true,
            ..default()
        });
        let vis = if tex.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let e = commands
            .spawn((
                Map2dTile { key: *canon },
                Mesh3d(quad.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(*cx, *cy, TILE_Z_ELEV).with_scale(Vec3::new(*w, *h, 1.0)),
                RenderLayers::layer(1),
                vis,
            ))
            .id();
        tiler.live.insert(*pk, e);
    }

    // 8. Queue downloads for anything not cached / already in flight.
    let mut wants: Vec<(u32, u32, u32)> = desired.values().map(|(c, _)| *c).collect();
    wants.sort_unstable();
    wants.dedup();
    for key in wants {
        if !tiler.cache.contains_key(&key) && tiler.in_flight.insert(key) {
            let _ = tiler.job_tx.send(key);
        }
    }

    // 9. Paint cached textures onto already-live entities that are still blank
    //    (covers the one-frame race where a tile is spawned before its image).
    for (tile, mat_handle, mut vis) in live.iter_mut() {
        if let Some(h) = tiler.cache.get(&tile.key) {
            if let Some(m) = materials.get_mut(mat_handle) {
                if m.base_color_texture.is_none() {
                    m.base_color_texture = Some(h.clone());
                }
            }
            if *vis != Visibility::Visible {
                *vis = Visibility::Visible;
            }
        }
    }
}

/// A 2D download worker: fetches Bing Aerial tiles over the network (or reads
/// `{root}/{z}/{x}/{y}.png` from disk when `OFFLINE_IMAGERY_ROOT` is set),
/// decodes to RGBA and ships results back. One agent per thread keeps the
/// connection pool warm across fetches.
fn map2d_worker(job_rx: Arc<Mutex<mpsc::Receiver<(u32, u32, u32)>>>, tx: mpsc::Sender<Map2dTileImg>) {
    let offline_root = crate::feature_flags::offline_imagery_root();
    let agent = ureq::AgentBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) CesiumRust/0.1")
        .timeout(std::time::Duration::from_secs(15))
        .build();
    loop {
        let job = job_rx.lock().unwrap().recv();
        let Ok((x, y, z)) = job else { return };

        let bytes: Option<Vec<u8>> = if let Some(root) = &offline_root {
            let path = root
                .join(z.to_string())
                .join(x.to_string())
                .join(format!("{y}.png"));
            std::fs::read(path).ok()
        } else {
            let qk = quadkey(x, y, z);
            let sub = (x + y) % 8;
            let url = format!(
                "https://ecn.t{sub}.tiles.virtualearth.net/tiles/a{qk}.jpeg?g=14393"
            );
            match agent.get(&url).call() {
                Ok(resp) => {
                    let mut buf = Vec::new();
                    match resp.into_reader().read_to_end(&mut buf) {
                        Ok(_) => Some(buf),
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            }
        };

        if let Some(data) = bytes {
            if let Ok(img) = image::load_from_memory(&data) {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let _ = tx.send(Map2dTileImg {
                    x,
                    y,
                    z,
                    rgba: rgba.into_raw(),
                    width,
                    height,
                });
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod map2d_tests {
    use super::*;

    #[test]
    fn wrap_x_is_periodic_and_bounded() {
        let pi = std::f32::consts::PI;
        // Just past the antimeridian wraps to the far west, seamless.
        assert!((wrap_x(pi + 0.1) - (-pi + 0.1)).abs() < 1e-4);
        assert!((wrap_x(-pi - 0.1) - (pi - 0.1)).abs() < 1e-4);
        for k in -6..=6 {
            let x = wrap_x(k as f32 * 1.3);
            assert!(x >= -pi - 1e-3 && x <= pi + 1e-3, "out of range: {x}");
        }
    }

    #[test]
    fn wrap_lon_deg_bounds() {
        assert!((wrap_lon_deg(190.0) - (-170.0)).abs() < 1e-4);
        assert!((wrap_lon_deg(-370.0) - (-10.0)).abs() < 1e-4);
        assert!((wrap_lon_deg(45.0) - 45.0).abs() < 1e-4);
    }

    #[test]
    fn zoom_level_monotonic_and_reasonable() {
        let low = approx_zoom_level(ZOOM_MIN);
        let high = approx_zoom_level(ZOOM_MAX);
        assert!(high > low, "level must grow with zoom ({low} -> {high})");
        assert!(low >= 0);
    }

    #[test]
    fn row_lat_is_monotonic_and_symmetric() {
        let n = 256.0;
        // Row 0 is the far north, the middle row is the equator, the last is south.
        assert!(row_to_lat(0.0, n) > 1.4, "top row near north pole");
        assert!(row_to_lat(n / 2.0, n).abs() < 1e-9, "middle row is the equator");
        assert!(row_to_lat(n, n) < -1.4, "bottom row near south pole");
        // Row <-> lat round-trip.
        for row in [3.0, 40.0, 128.0, 200.0] {
            let lat = row_to_lat(row, n);
            assert!((lat_to_row(lat, n) - row).abs() < 1e-6, "round trip {row}");
        }
    }

    #[test]
    fn tile_rect_tiles_the_world_exactly() {
        let n = 16.0;
        let (c0, _, w, _) = tile_rect(0.0, 0.0, n);
        let (c1, _, w1, _) = tile_rect(1.0, 0.0, n);
        // All columns share one width == the world period / n.
        let world_w = (TAU6 / n) as f32;
        assert!((w - world_w).abs() < 1e-3);
        assert!((w - w1).abs() < 1e-6, "uniform columns");
        // Adjacent column centres sit exactly one width apart (seamless).
        assert!((c1 - c0 - w).abs() < 1e-3, "column pitch == width");
        // West edge of column 0 is -pi; west edge of column n is +pi (one world).
        assert!((c0 - w / 2.0 + PI6 as f32).abs() < 1e-2, "col 0 west == -pi");
        let (cn, _, wn, _) = tile_rect(n, 0.0, n);
        assert!((cn - wn / 2.0 - PI6 as f32).abs() < 1e-2, "col n west == +pi");
        // Rows abut: south edge of row 4 == north edge of row 5.
        let (_, y4, _, h4) = tile_rect(0.0, 4.0, n);
        let (_, y5, _, h5) = tile_rect(0.0, 5.0, n);
        assert!(
            (y4 - h4 / 2.0 - (y5 + h5 / 2.0)).abs() < 1e-4,
            "rows abut"
        );
    }

    #[test]
    fn quadkey_known_value() {
        // x=3, y=5, z=3 -> "213" (standard Bing/OSM quadkey).
        assert_eq!(quadkey(3, 5, 3), "213");
        assert_eq!(quadkey(0, 0, 0), "");
        assert_eq!(quadkey(1, 1, 1), "3");
    }

    #[test]
    fn tile_zoom_grows_with_zoom_and_stays_bounded() {
        assert!(tile_zoom_for(ZOOM_MAX) > tile_zoom_for(ZOOM_MIN));
        for z in [ZOOM_MIN, 500.0, 2000.0, ZOOM_MAX] {
            let t = tile_zoom_for(z);
            assert!((TILE_Z_MIN as u32..=TILE_Z_MAX as u32).contains(&t), "z {t}");
        }
    }

    #[test]
    fn clamp_center_pins_world_and_bounds_poles() {
        // Zoomed out so the whole world fits vertically -> pin to the equator,
        // longitude untouched.
        let c = clamp_center_y(Vec2::new(0.5, 1.4), 100.0, 700.0);
        assert_eq!(c.y, 0.0);
        assert_eq!(c.x, 0.5);
        // Zoomed in -> keep the band inside ±90 so a pole never shows empty.
        let lim = LAT_MAX - 700.0 * 0.5 / 2000.0;
        let north = clamp_center_y(Vec2::new(0.0, 5.0), 2000.0, 700.0);
        assert!((north.y - lim).abs() < 1e-4, "north clamp");
        let south = clamp_center_y(Vec2::new(0.0, -5.0), 2000.0, 700.0);
        assert!((south.y + lim).abs() < 1e-4, "south clamp");
    }

    #[test]
    fn min_zoom_fills_frame_and_respects_floor() {
        // A 727px-tall canvas needs ~727/π px/unit for the π-tall world to fill.
        let m = min_zoom_for(727.0);
        assert!(m >= ZOOM_MIN);
        assert!((m - 727.0 / std::f32::consts::PI).abs() < 1.0, "m = {m}");
        // A tiny canvas falls back to the fixed floor.
        assert_eq!(min_zoom_for(100.0), ZOOM_MIN);
    }
}
