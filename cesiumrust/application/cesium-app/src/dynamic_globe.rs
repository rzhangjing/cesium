//! View-dependent dynamic LOD globe tile manager.
//!
//! Key features (CesiumJS-inspired):
//! - View-dependent loading: only ~121 visible tiles loaded at any time
//! - Parallel downloads: 8 threads for fast texture loading
//! - Texture cache: previously downloaded tiles are cached in memory
//! - Progressive transition: old tiles stay visible until new tiles have textures
//! - Zoom levels 3-12: from global overview to street-level detail

use bevy::prelude::*;
use std::collections::HashMap;
use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::orbit_camera::OrbitState;
use crate::tile_mesh::{create_tile_mesh, render_scale, GlobeTile};
use cesium_bevy_render::CesiumGlobe;

// ── Configuration ──────────────────────────────────────────────────

const MIN_ZOOM: u32 = 3;
const MAX_ZOOM: u32 = 12;
const VIEW_HALF_WINDOW: i32 = 5;
const VIEW_REFRESH_THRESHOLD: f64 = 2.0;
const BASE_SEGMENTS: u32 = 48;
/// Multiplicative hysteresis band: zoom in below threshold*0.8, zoom out
/// above threshold*1.2 (absolute hysteresis would block high zoom levels
/// whose distance thresholds are smaller than the band).
/// Number of parallel download threads.
const DOWNLOAD_THREADS: usize = 8;

const ZOOM_IN_THRESHOLDS: [f32; 9] = [
    6.0, 2.5, 1.2, 0.6, 0.3, 0.15, 0.07, 0.035, 0.018,
];
const ZOOM_OUT_THRESHOLDS: [f32; 9] = [
    7.0, 3.0, 1.5, 0.8, 0.4, 0.2, 0.1, 0.05, 0.025,
];

type TileKey = (u32, u32, u32);

/// Cached tile image data (raw RGBA bytes + dimensions).
struct CachedTexture {
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
}

// ── Resources ──────────────────────────────────────────────────────

#[derive(Resource)]
struct TileManager {
    /// Currently-spawned tile entities.
    tile_entities: HashMap<TileKey, Entity>,
    /// Which of the spawned tiles have textures applied (no longer blue).
    textured_tiles: std::collections::HashSet<TileKey>,
    current_zoom: u32,
    last_center_x: f64,
    last_center_y: f64,
    initialized: bool,
    /// The last computed visible tile set (all LOD levels). Spawned tiles
    /// outside this set are stale and get cleaned up progressively.
    visible_set: std::collections::HashSet<TileKey>,
}

impl Default for TileManager {
    fn default() -> Self {
        Self {
            tile_entities: HashMap::new(),
            textured_tiles: std::collections::HashSet::new(),
            current_zoom: MIN_ZOOM,
            last_center_x: 0.0,
            last_center_y: 0.0,
            initialized: false,
            visible_set: std::collections::HashSet::new(),
        }
    }
}

#[derive(Resource)]
struct TextureReceiver {
    tx: mpsc::Sender<TileDownloadResult>,
    rx: Mutex<mpsc::Receiver<TileDownloadResult>>,
    /// Persistent cache of downloaded tile image data.
    cache: Arc<Mutex<HashMap<TileKey, CachedTexture>>>,
}

impl Default for TextureReceiver {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(rx),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

struct TileDownloadResult {
    x: u32,
    y: u32,
    z: u32,
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
}

// ── Plugin ─────────────────────────────────────────────────────────

pub struct DynamicGlobePlugin;

impl Plugin for DynamicGlobePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileManager>()
            .init_resource::<TextureReceiver>()
            .add_systems(Startup, initial_spawn)
            .add_systems(
                Update,
                (view_dependent_update, apply_textures_from_downloads),
            );
    }
}

// ── Startup ────────────────────────────────────────────────────────

fn initial_spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mgr: ResMut<TileManager>,
    tex_rx: Res<TextureReceiver>,
    orbit: Res<OrbitState>,
) {
    // Spawn directly at the zoom the current camera distance settles on, so
    // startup doesn't waste bandwidth downloading intermediate zoom levels
    // that would be replaced immediately.
    let surface_dist = (orbit.distance - 1.0_f32).max(0.0);
    let mut zoom = MIN_ZOOM;
    for _ in 0..(MAX_ZOOM - MIN_ZOOM) as usize {
        let next = compute_target_zoom(surface_dist, zoom);
        if next == zoom {
            break;
        }
        zoom = next;
    }

    let (lat_rad, lon_rad) = compute_sub_camera_point(&orbit);
    let (center_tx, center_ty) = geo_to_tile(lat_rad, lon_rad, zoom);
    let visible = compute_lod_tiles(zoom, lat_rad, lon_rad, orbit.distance);
    mgr.visible_set = visible.iter().copied().collect();
    spawn_tiles(&mut commands, &mut meshes, &mut materials, &mut mgr, &visible, zoom);
    start_downloads(&tex_rx, &visible);
    mgr.current_zoom = zoom;
    mgr.last_center_x = center_tx as f64;
    mgr.last_center_y = center_ty as f64;
    mgr.initialized = true;
    println!(
        "[DynGlobe] Initial: {} tiles at zoom {}",
        visible.len(),
        zoom
    );
}

// ── View-dependent update ──────────────────────────────────────────

fn view_dependent_update(
    orbit: Res<OrbitState>,
    mut mgr: ResMut<TileManager>,
    tex_rx: Res<TextureReceiver>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !mgr.initialized {
        return;
    }

    let surface_dist = (orbit.distance - 1.0_f32).max(0.0);
    let target_zoom = compute_target_zoom(surface_dist, mgr.current_zoom);
    let (lat_rad, lon_rad) = compute_sub_camera_point(&orbit);
    let (center_tx, center_ty) = geo_to_tile(lat_rad, lon_rad, target_zoom);

    let dx = (center_tx as f64) - mgr.last_center_x;
    let dy = (center_ty as f64) - mgr.last_center_y;
    let moved = (dx * dx + dy * dy).sqrt();

    let zoom_changed = target_zoom != mgr.current_zoom;
    let view_changed = moved > VIEW_REFRESH_THRESHOLD;

    if !zoom_changed && !view_changed {
        return;
    }

    if zoom_changed {
        println!(
            "[DynGlobe] Zoom {} → {} (dist={:.3})",
            mgr.current_zoom, target_zoom, surface_dist
        );
    }

    let new_visible =
        compute_lod_tiles(target_zoom, lat_rad, lon_rad, orbit.distance);
    let new_set: std::collections::HashSet<TileKey> =
        new_visible.iter().copied().collect();
    mgr.visible_set = new_set.clone();
    let old_set: std::collections::HashSet<TileKey> =
        mgr.tile_entities.keys().copied().collect();

    let to_add: Vec<TileKey> =
        new_set.difference(&old_set).copied().collect();
    let to_remove: Vec<TileKey> =
        old_set.difference(&new_set).copied().collect();

    // On plain pans, despawn tiles leaving the view immediately. On zoom
    // changes keep stale tiles as fallback coverage; the progressive cleanup
    // in `apply_textures_from_downloads` removes them once new tiles load.
    let immediate_remove: Vec<TileKey> = if zoom_changed {
        Vec::new()
    } else {
        to_remove
    };

    // Despawn removed tiles
    for key in &immediate_remove {
        if let Some(entity) = mgr.tile_entities.remove(key) {
            commands.entity(entity).despawn();
            mgr.textured_tiles.remove(key);
        }
    }

    // Spawn added tiles
    let scale = render_scale();
    let segments = compute_segments(target_zoom);

    // Check cache first: if tile is cached, apply texture immediately
    let cache = tex_rx.cache.lock().unwrap();

    for &(tx, ty, tz) in &to_add {
        let mesh = create_tile_mesh(tx, ty, tz, segments, level_tuck(tz, target_zoom));

        let (material, has_texture) = if let Some(cached) = cache.get(&(tx, ty, tz)) {
            let tex_handle = build_gpu_image(
                &mut images,
                cached.rgba_data.clone(),
                cached.width,
                cached.height,
                tz,
                target_zoom,
            );
            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(tex_handle),
                perceptual_roughness: 0.9,
                ..default()
            });
            (mat, true)
        } else {
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.04, 0.15, 0.4),
                perceptual_roughness: 0.9,
                ..default()
            });
            (mat, false)
        };

        let entity = commands
            .spawn((
                CesiumGlobe,
                GlobeTile {
                    x: tx,
                    y: ty,
                    z: tz,
                },
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::from_scale(Vec3::splat(scale)),
            ))
            .id();

        mgr.tile_entities.insert((tx, ty, tz), entity);
        if has_texture {
            mgr.textured_tiles.insert((tx, ty, tz));
        }
    }

    drop(cache);

    mgr.current_zoom = target_zoom;
    mgr.last_center_x = center_tx as f64;
    mgr.last_center_y = center_ty as f64;

    if !to_add.is_empty() || !immediate_remove.is_empty() {
        println!(
            "[DynGlobe] +{} -{} tiles, {} total",
            to_add.len(),
            immediate_remove.len(),
            mgr.tile_entities.len()
        );
    }

    // Filter out cached tiles from download list
    let need_download: Vec<TileKey> = {
        let cache = tex_rx.cache.lock().unwrap();
        to_add
            .iter()
            .filter(|k| !cache.contains_key(k))
            .copied()
            .collect()
    };

    if !need_download.is_empty() {
        start_downloads(&tex_rx, &need_download);
    }
}

// ── Zoom level computation ─────────────────────────────────────────

fn compute_target_zoom(surface_dist: f32, current_zoom: u32) -> u32 {
    let idx = current_zoom.saturating_sub(MIN_ZOOM) as usize;

    if current_zoom > MIN_ZOOM {
        let out_idx = (current_zoom - MIN_ZOOM - 1) as usize;
        if out_idx < ZOOM_OUT_THRESHOLDS.len()
            && surface_dist > ZOOM_OUT_THRESHOLDS[out_idx] * 1.2
        {
            return current_zoom - 1;
        }
    }

    if current_zoom < MAX_ZOOM {
        if idx < ZOOM_IN_THRESHOLDS.len()
            && surface_dist < ZOOM_IN_THRESHOLDS[idx] * 0.8
        {
            return current_zoom + 1;
        }
    }

    current_zoom
}

// ── Sub-camera point ───────────────────────────────────────────────

fn compute_sub_camera_point(orbit: &OrbitState) -> (f64, f64) {
    let cos_pitch = orbit.pitch.cos();
    let sin_pitch = orbit.pitch.sin();
    let dir_x = cos_pitch * orbit.heading.cos();
    let dir_y = cos_pitch * orbit.heading.sin();
    let dir_z = sin_pitch;

    let len = (dir_x * dir_x + dir_y * dir_y + dir_z * dir_z).sqrt();
    let lat = (dir_z / len).asin() as f64;
    let lon = (dir_y as f64).atan2(dir_x as f64);
    (lat, lon)
}

// ── Visible tile computation ───────────────────────────────────────

/// Float tile coordinates of a geographic point at `zoom` (tile centers at
/// integer + 0.5).
fn geo_to_tile_f(lat_rad: f64, lon_rad: f64, zoom: u32) -> (f64, f64) {
    let n = (1u64 << zoom) as f64;
    let tx = (lon_rad + std::f64::consts::PI) / (2.0 * std::f64::consts::PI) * n - 0.5;
    let lat_c = lat_rad.clamp(-1.4844, 1.4844);
    let ty = (1.0 - lat_c.tan().asinh() / std::f64::consts::PI) / 2.0 * n - 0.5;
    (tx, ty)
}

/// Concentric multi-resolution tile selection (CesiumJS-style quadtree LOD
/// approximation):
/// - the finest (target) zoom covers the central window;
/// - progressively coarser zooms fill rings out to the visible horizon cap,
///   skipping tiles fully covered by a finer window.
///
/// From distance `d` (globe radius = 1) the visible cap angular radius is
/// acos(1/d), so the limb shows imagery instead of the base sphere.
fn compute_lod_tiles(
    target_zoom: u32,
    lat_rad: f64,
    lon_rad: f64,
    distance: f32,
) -> Vec<TileKey> {
    let d = (distance as f64).max(1.001);
    let cap_deg = (1.0 / d).acos().to_degrees();

    let mut tiles: std::collections::HashSet<TileKey> = std::collections::HashSet::new();
    // Coverage rect of the finer window, expressed in current-level coords.
    let mut finer_rect: Option<(f64, f64, f64, f64)> = None;

    for z in (MIN_ZOOM..=target_zoom).rev() {
        let n = 1u32 << z;
        let tile_deg = 360.0 / n as f64;
        let need = (cap_deg / tile_deg).ceil() as i32 + 1;
        // Same clamp for all levels: coarse rings only fill the foreshortened
        // horizon band, where a half-window of 8 of the next-finer level
        // already reaches the limb at any camera distance.
        let hw = need.clamp(VIEW_HALF_WINDOW, 8);

        let (cxf, cyf) = geo_to_tile_f(lat_rad, lon_rad, z);
        let cx = cxf.round() as i32;
        let cy = cyf.round() as i32;

        for dy in -hw..=hw {
            let ty = cy + dy;
            if ty < 0 || ty >= n as i32 {
                continue;
            }
            for dx in -hw..=hw {
                let tx = cx + dx;
                // Skip tiles fully covered by the finer window.
                if let Some((fx0, fx1, fy0, fy1)) = finer_rect {
                    if (tx as f64) >= fx0
                        && (tx + 1) as f64 <= fx1
                        && (ty as f64) >= fy0
                        && (ty + 1) as f64 <= fy1
                    {
                        continue;
                    }
                }
                let tx_w = ((tx % n as i32) + n as i32) as u32 % n;
                tiles.insert((tx_w, ty as u32, z));
            }
        }

        // This level's window becomes the "finer rect" for the next coarser
        // level (coordinates halve per level).
        finer_rect = Some((
            (cx - hw) as f64 * 0.5,
            (cx + hw + 1) as f64 * 0.5,
            (cy - hw) as f64 * 0.5,
            (cy + hw + 1) as f64 * 0.5,
        ));
    }

    tiles.into_iter().collect()
}

/// Radial tuck per LOD level so coarser rings sit just below finer tiles,
/// preventing z-fighting where a coarse tile partially underlaps the finer
/// window.
fn level_tuck(z: u32, finest: u32) -> f64 {
    1.0 - 0.0006 * finest.saturating_sub(z) as f64
}

/// Create a GPU texture from raw RGBA data, downscaling to 128x128 for tiles
/// coarser than the current finest level (they only appear small on screen).
fn build_gpu_image(
    images: &mut Assets<Image>,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    tile_z: u32,
    finest: u32,
) -> Handle<Image> {
    let (data, w, h) = if tile_z < finest && width > 128 {
        let img =
            image::RgbaImage::from_raw(width, height, rgba).expect("rgba buffer size mismatch");
        let small = image::DynamicImage::ImageRgba8(img)
            .resize(128, 128, image::imageops::FilterType::Triangle);
        (small.to_rgba8().into_raw(), 128, 128)
    } else {
        (rgba, width, height)
    };
    images.add(Image::new(
        bevy::render::render_resource::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    ))
}

fn geo_to_tile(lat_rad: f64, lon_rad: f64, zoom: u32) -> (u32, u32) {
    let n = (1u64 << zoom) as f64;
    let tx = ((lon_rad + std::f64::consts::PI) / (2.0 * std::f64::consts::PI) * n)
        .floor()
        .clamp(0.0, n - 1.0) as u32;
    let lat_c = lat_rad.clamp(-1.4844, 1.4844);
    let ty = ((1.0 - lat_c.tan().asinh() / std::f64::consts::PI) / 2.0 * n)
        .floor()
        .clamp(0.0, n - 1.0) as u32;
    (tx, ty)
}

// ── Tile helpers ───────────────────────────────────────────────────

fn spawn_tiles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    mgr: &mut TileManager,
    tiles: &[TileKey],
    finest_zoom: u32,
) {
    let scale = render_scale();
    for &(tx, ty, tz) in tiles {
        let seg = compute_segments(tz);
        let mesh = create_tile_mesh(tx, ty, tz, seg, level_tuck(tz, finest_zoom));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.04, 0.15, 0.4),
            perceptual_roughness: 0.9,
            ..default()
        });
        let entity = commands
            .spawn((
                CesiumGlobe,
                GlobeTile { x: tx, y: ty, z: tz },
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::from_scale(Vec3::splat(scale)),
            ))
            .id();
        mgr.tile_entities.insert((tx, ty, tz), entity);
    }
}

fn compute_segments(zoom: u32) -> u32 {
    // Coarse tiles still need enough subdivisions to keep the limb
    // silhouette smooth where they meet the horizon.
    (BASE_SEGMENTS >> zoom.saturating_sub(MIN_ZOOM)).max(8)
}

// ── Bing Maps downloads (parallel) ─────────────────────────────────

fn tile_to_quadkey(x: u32, y: u32, level: u32) -> String {
    let mut qk = String::with_capacity(level as usize);
    for i in (0..level).rev() {
        let mut d = 0u8;
        let mask = 1 << i;
        if (x & mask) != 0 { d |= 1; }
        if (y & mask) != 0 { d |= 2; }
        qk.push_str(&d.to_string());
    }
    qk
}

/// Start PARALLEL background downloads using a thread pool.
fn start_downloads(tex_rx: &TextureReceiver, tiles: &[TileKey]) {
    let tiles_owned: Vec<TileKey> = tiles.to_vec();
    let tx = tex_rx.tx.clone();

    std::thread::spawn(move || {
        let total = tiles_owned.len();

        // Split tiles across DOWNLOAD_THREADS workers
        let chunks: Vec<Vec<TileKey>> = {
            let mut c: Vec<Vec<TileKey>> = (0..DOWNLOAD_THREADS).map(|_| Vec::new()).collect();
            for (i, &tile) in tiles_owned.iter().enumerate() {
                c[i % DOWNLOAD_THREADS].push(tile);
            }
            c
        };

        let mut handles = Vec::new();
        for chunk in chunks {
            let tx = tx.clone();
            handles.push(std::thread::spawn(move || {
                let agent = ureq::AgentBuilder::new()
                    .user_agent("Mozilla/5.0 CesiumRust/0.1")
                    .timeout(std::time::Duration::from_secs(10))
                    .build();

                for &(px, py, pz) in &chunk {
                    let qk = tile_to_quadkey(px, py, pz);
                    let sub = (px + py) % 8;
                    let url = format!(
                        "https://ecn.t{}.tiles.virtualearth.net/tiles/a{}.jpeg?g=14393",
                        sub, qk
                    );
                    match agent.get(&url).call() {
                        Ok(resp) => {
                            let mut reader = resp.into_reader();
                            let mut data = Vec::new();
                            if reader.read_to_end(&mut data).is_ok() {
                                if let Ok(img) = image::load_from_memory(&data) {
                                    let rgba = img.to_rgba8();
                                    let (w, h) = rgba.dimensions();
                                    let _ = tx.send(TileDownloadResult {
                                        x: px, y: py, z: pz,
                                        rgba_data: rgba.into_raw(),
                                        width: w, height: h,
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[DL] ({},{},{}): {}", px, py, pz, e);
                        }
                    }
                }
            }));
        }

        // Wait for all download threads
        for h in handles {
            let _ = h.join();
        }

        println!("[DL] Batch complete: {} tiles", total);
    });
}

// ── Texture application + cache storage ────────────────────────────

fn apply_textures_from_downloads(
    tex_rx: Res<TextureReceiver>,
    mut mgr: ResMut<TileManager>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tile_query: Query<(&GlobeTile, &MeshMaterial3d<StandardMaterial>)>,
    mut commands: Commands,
) {
    let mut results: Vec<TileDownloadResult> = Vec::new();
    {
        let rx = tex_rx.rx.lock().unwrap();
        while let Ok(r) = rx.try_recv() {
            results.push(r);
        }
    }

    if results.is_empty() {
        return;
    }

    let mut cache = tex_rx.cache.lock().unwrap();
    let finest = mgr.current_zoom;

    for result in results {
        // Only the finest level is worth caching at full resolution; coarser
        // tiles are transient filler rings.
        if result.z == finest {
            cache.insert(
                (result.x, result.y, result.z),
                CachedTexture {
                    rgba_data: result.rgba_data.clone(),
                    width: result.width,
                    height: result.height,
                },
            );
        }

        // Create Bevy texture (downscaled for coarse filler levels)
        let tex_handle = build_gpu_image(
            &mut images,
            result.rgba_data,
            result.width,
            result.height,
            result.z,
            finest,
        );

        // Apply to matching entity
        for (globe_tile, mat_handle) in tile_query.iter() {
            if globe_tile.x == result.x
                && globe_tile.y == result.y
                && globe_tile.z == result.z
            {
                if let Some(mat) = materials.get_mut(mat_handle) {
                    mat.base_color_texture = Some(tex_handle.clone());
                    mat.base_color = Color::WHITE;
                }
                mgr.textured_tiles
                    .insert((result.x, result.y, result.z));
                break;
            }
        }
    }

    // Progressive cleanup: once a fifth of the finest visible tiles have
    // textures, despawn spawned entities that are no longer in the visible
    // set (stale tiles kept as fallback coverage during zoom transitions).
    let current_zoom = mgr.current_zoom;
    let fine_total = mgr
        .visible_set
        .iter()
        .filter(|(_, _, z)| *z == current_zoom)
        .count();
    let fine_textured = mgr
        .visible_set
        .iter()
        .filter(|k| k.2 == current_zoom && mgr.textured_tiles.contains(k))
        .count();

    if fine_total > 0 && fine_textured > fine_total / 5 {
        let stale: Vec<TileKey> = mgr
            .tile_entities
            .keys()
            .filter(|k| !mgr.visible_set.contains(k))
            .copied()
            .collect();
        if !stale.is_empty() {
            let removed_count = stale.len();
            for key in stale {
                if let Some(entity) = mgr.tile_entities.remove(&key) {
                    commands.entity(entity).despawn();
                    mgr.textured_tiles.remove(&key);
                }
            }
            println!(
                "[DynGlobe] Cleaned {} stale tiles, {} remaining",
                removed_count,
                mgr.tile_entities.len()
            );
        }
    }
}
