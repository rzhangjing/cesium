//! Dynamic view-dependent globe tiling with a budgeted async pipeline.
//!
//! Key features (CesiumJS-inspired):
//! - Screen-space-error quadtree LOD (the CesiumJS `QuadtreePrimitive`
//!   strategy): every visible tile is subdivided while its projected screen
//!   footprint exceeds the SSE budget, so the view center is always crisp and
//!   resolution falls off continuously toward the limb
//! - Trilinear-mipmap + anisotropic texture sampling so oblique and minified
//!   tiles stay crisp instead of shimmering/blurring
//! - Parallel downloads (8 threads) and parallel mesh generation (8 threads)
//! - Priority request scheduling: tiles with the largest screen footprint are
//!   downloaded/spawned first (CesiumJS tile request scheduler role)
//! - Per-frame budgets: mesh uploads / entity spawns / texture uploads are
//!   spread across frames so loading never stalls interaction (this is the
//!   same role CesiumJS's per-frame tile processing budget plays)
//! - GPU handle caches (mesh / material / texture): revisiting an area
//!   reuses existing GPU resources instead of re-uploading everything
//! - Progressive cleanup: stale tiles stay as fallback coverage until the
//!   new visible set is sufficiently textured
//! - Zoom levels 3-19: from global overview to street-level detail

use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::orbit_camera::{OrbitState, CAMERA_FOV_Y};
use crate::tile_mesh::{create_tile_mesh, create_tile_mesh_uv, render_scale, GlobeTile};
use cesium_bevy_render::CesiumGlobe;

// ── Configuration ─────────────────────────────────────────────────

const MIN_ZOOM: u32 = 3;
const MAX_ZOOM: u32 = 19;
const BASE_SEGMENTS: u32 = 48;
/// WGS84 semi-major axis (meters), for screen-space-error math.
const EARTH_RADIUS_M: f64 = 6378137.0;
/// A 256-px tile may cover at most this many screen pixels before the
/// quadtree subdivides it — the CesiumJS `maximumScreenSpaceError = 2`
/// budget (2 px of geometric error per texel).
/// Number of parallel download / mesh-build worker threads.
const DOWNLOAD_THREADS: usize = 16;

/// Per-frame budgets. Asset work (GPU buffer/texture uploads, entity spawns)
/// is sliced across frames at these rates so a zoom or pan never produces a
/// multi-hundred-millisecond hitch; tiles ramp in over a few frames instead.
const MAX_MESH_UPLOADS_PER_FRAME: usize = 12;
const MAX_SPAWNS_PER_FRAME: usize = 16;
const MAX_TEXTURE_UPLOADS_PER_FRAME: usize = 16;
/// Despawn budget: entity removal costs real CPU (component drops + render
/// extraction), so bulk cleanup is sliced across frames.
const MAX_DESPAWNS_PER_FRAME: usize = 24;
/// Hard cap on live tile entities. Beyond this the draw-call count itself
/// becomes the bottleneck, so the oldest surplus tiles are evicted even if
/// still "visible" (their GPU handles stay cached for cheap re-spawn).
/// Sized above the typical visible-set peak (~750 for the concentric LOD
/// layout) so a settled view never loses coverage to eviction.
const MAX_TILE_ENTITIES: usize = 1100;
/// Coarsest levels kept resident as a permanent global fallback layer
/// (CesiumJS's base imagery layer role): downloaded once at startup, never
/// despawned/evicted, so fast pans into never-visited regions show blurry
/// imagery instead of the black base sphere while fine tiles load.
const BASE_LAYER_ZOOM: u32 = 3;
/// Upper bound for the per-tile GPU handle caches (textures dominate at
/// 256 KB each); oldest entries are evicted FIFO.
const MAX_GPU_CACHE_ENTRIES: usize = 3000;

const MAX_TILE_SCREEN_PX: f64 = 512.0;

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
    textured_tiles: HashSet<TileKey>,
    current_zoom: u32,
    last_center_x: f64,
    last_center_y: f64,
    /// Camera distance (globe radii) at the last visible-set recompute;
    /// a relative change re-triggers the SSE traversal.
    last_distance: f32,
    initialized: bool,
    /// The last computed visible tile set (all LOD levels). Spawned tiles
    /// outside this set are stale and get cleaned up progressively.
    visible_set: HashSet<TileKey>,
    /// Tiles waiting for mesh build + spawn.
    spawn_queue: VecDeque<TileKey>,
    /// Mirror of spawn_queue for O(1) duplicate checks.
    queued: HashSet<TileKey>,
    /// Reusable GPU handles so revisited tiles never re-upload:
    /// mesh geometry, material and texture are each created once per tile.
    gpu_meshes: HashMap<TileKey, Handle<Mesh>>,
    gpu_materials: HashMap<TileKey, Handle<StandardMaterial>>,
    gpu_textures: HashMap<TileKey, Handle<Image>>,
    /// Tiles whose imagery is permanently unavailable (Bing no-imagery
    /// placeholder or repeated download failure). They inherit the nearest
    /// textured ancestor's image (UV-remapped mesh) or a solid ocean color,
    /// so the globe never shows a hole down to the base sphere.
    no_data: HashSet<TileKey>,
    /// UV-remapped meshes for no-data tiles inheriting an ancestor texture.
    gpu_fb_meshes: HashMap<TileKey, Handle<Mesh>>,
    /// The texture a tile actually displays (its own or an inherited
    /// ancestor's); doubles as the ancestor-lookup table for deeper no-data
    /// tiles.
    effective_tex: HashMap<TileKey, Handle<Image>>,
    /// No-data tiles with no textured ancestor: rendered as solid ocean.
    solid_tiles: HashSet<TileKey>,
    /// Insertion order of gpu_textures (FIFO eviction order).
    gpu_tex_order: VecDeque<TileKey>,
    /// Source resolution (width in px) each GPU texture was created from,
    /// so horizon-filler tiles fetched at 128 px can be re-requested at
    /// full res once they are projected large.
    gpu_tex_size: HashMap<TileKey, u32>,
    /// Tiles with a full-res re-download in flight (dedupe guard).
    reupload: HashSet<TileKey>,
    /// Tiles with any download in flight; dedupes fetches and, together with
    /// coverage repair, guarantees a queued visible tile can never end up
    /// without a download backing it.
    in_flight: HashSet<TileKey>,
    /// Insertion order of spawned entities (FIFO eviction order).
    spawn_order: VecDeque<TileKey>,
    /// Set when `view_dependent_update` recomputed the visible set this
    /// frame; cleanup only runs on stable frames so a drag doesn't despawn
    /// tiles that leave and re-enter the visible set frame to frame.
    view_changed_this_frame: bool,
}

impl Default for TileManager {
    fn default() -> Self {
        Self {
            tile_entities: HashMap::new(),
            textured_tiles: HashSet::new(),
            current_zoom: MIN_ZOOM,
            last_center_x: 0.0,
            last_center_y: 0.0,
            last_distance: 0.0,
            initialized: false,
            visible_set: HashSet::new(),
            spawn_queue: VecDeque::new(),
            queued: HashSet::new(),
            gpu_meshes: HashMap::new(),
            gpu_materials: HashMap::new(),
            gpu_textures: HashMap::new(),
            no_data: HashSet::new(),
            gpu_fb_meshes: HashMap::new(),
            effective_tex: HashMap::new(),
            solid_tiles: HashSet::new(),
            gpu_tex_order: VecDeque::new(),
            gpu_tex_size: HashMap::new(),
            reupload: HashSet::new(),
            in_flight: HashSet::new(),
            spawn_order: VecDeque::new(),
            view_changed_this_frame: false,
        }
    }
}

#[derive(Resource)]
struct TextureReceiver {
    tx: mpsc::Sender<TileDownloadResult>,
    rx: Mutex<mpsc::Receiver<TileDownloadResult>>,
    /// Persistent cache of downloaded tile image data (finest level only).
    cache: Arc<Mutex<HashMap<TileKey, CachedTexture>>>,
    /// Tiles still worth downloading (queued / visible / spawned). Workers
    /// check this before every fetch so batches made stale by a fast pan
    /// release their bandwidth to the current view instead of finishing
    /// downloads nobody will look at.
    wanted: Arc<Mutex<HashSet<TileKey>>>,
}

impl Default for TextureReceiver {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(rx),
            cache: Arc::new(Mutex::new(HashMap::new())),
            wanted: Arc::new(Mutex::new(HashSet::new())),
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
    /// True when the tile has no usable imagery (Bing placeholder or all
    /// retries failed); the main thread then inherits ancestor coverage.
    placeholder: bool,
    /// True when the worker skipped the fetch because the tile left the
    /// wanted set (view moved on); the main thread only clears `in_flight`.
    aborted: bool,
}

/// Background mesh-generation pipeline: workers build `Mesh` values off the
/// main thread (the trig-heavy ellipsoid tessellation never blocks rendering);
/// the main thread only adds the finished meshes to the GPU within budget.
#[derive(Resource)]
struct MeshPipeline {
    tx: mpsc::Sender<(TileKey, Mesh, bool)>,
    rx: Mutex<mpsc::Receiver<(TileKey, Mesh, bool)>>,
    /// Finished meshes not yet uploaded to the GPU (budget overflow).
    /// The bool marks UV-remapped fallback meshes (no-data tiles).
    backlog: VecDeque<(TileKey, Mesh, bool)>,
}

impl Default for MeshPipeline {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(rx),
            backlog: VecDeque::new(),
        }
    }
}

// ── Plugin ─────────────────────────────────────────────────────────

pub struct DynamicGlobePlugin;

impl Plugin for DynamicGlobePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileManager>()
            .init_resource::<TextureReceiver>()
            .init_resource::<MeshPipeline>()
            .add_systems(Startup, initial_spawn)
            .add_systems(Update, (view_dependent_update, process_pipeline, smooth_tuck).chain());
    }
}

// ── Startup ────────────────────────────────────────────────────────

fn initial_spawn(
    mut mgr: ResMut<TileManager>,
    mut mesh_pipe: ResMut<MeshPipeline>,
    tex_rx: Res<TextureReceiver>,
    orbit: Res<OrbitState>,
    windows: Query<&Window>,
) {
    // The SSE traversal directly yields the settled LOD layout, so startup
    // never wastes bandwidth on intermediate zoom levels that would be
    // replaced immediately.
    let (lat_rad, lon_rad) = compute_sub_camera_point(&orbit);
    let visible = compute_visible_tiles(
        lat_rad,
        lon_rad,
        orbit.distance as f64,
        focal_pixels(&windows),
    );
    let finest = visible.iter().map(|t| t.0 .2).max().unwrap_or(MIN_ZOOM);
    mgr.visible_set = visible.iter().map(|&(k, _)| k).collect();

    enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &visible);

    // Permanent coarse fallback layer: every tile up to BASE_LAYER_ZOOM,
    // fetched downscaled (prio < 160) so the whole globe costs ~5 MB.
    let mut base: Vec<(TileKey, f32)> = Vec::new();
    for z in 1..=BASE_LAYER_ZOOM {
        for y in 0..(1u32 << z) {
            for x in 0..(1u32 << z) {
                base.push(((x, y, z), 100.0));
            }
        }
    }
    enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &base);

    mgr.current_zoom = finest;
    mgr.last_distance = orbit.distance;
    let (center_tx, center_ty) = geo_to_tile(lat_rad, lon_rad, finest);
    mgr.last_center_x = center_tx as f64;
    mgr.last_center_y = center_ty as f64;
    mgr.initialized = true;
    println!(
        "[DynGlobe] Initial: {} tiles queued, finest zoom {}",
        visible.len(),
        finest
    );
}

/// Queue mesh builds + downloads for tiles that need them, highest screen
/// footprint first (CesiumJS request-scheduler role: the view center
/// sharpens before horizon filler arrives). Entities are spawned later by
/// `process_pipeline` within the per-frame budget.
fn enqueue_tiles(
    mgr: &mut TileManager,
    mesh_pipe: &mut MeshPipeline,
    tex_rx: &TextureReceiver,
    tiles: &[(TileKey, f32)],
) {
    let mut mesh_jobs: Vec<(TileKey, u32, Option<[f32; 4]>)> = Vec::new();
    let mut downloads: Vec<(TileKey, bool, f32)> = Vec::new();
    let mut to_spawn: Vec<(TileKey, f32)> = Vec::new();

    {
        let cache = tex_rx.cache.lock().unwrap();
        for &(key, prio) in tiles {
            // Resolution upgrade: a tile first fetched as a small horizon
            // filler (128 px) but now projected large must be re-fetched at
            // full res, otherwise its stretched texels read as a smeared
            // comb band right next to crisp tiles at LOD junctions.
            if !mgr.no_data.contains(&key)
                && !mgr.reupload.contains(&key)
                && prio > 192.0
                && matches!(mgr.gpu_tex_size.get(&key), Some(&sz) if sz < 256)
            {
                downloads.push((key, false, prio));
                mgr.reupload.insert(key);
                mgr.in_flight.insert(key);
            }
            if mgr.tile_entities.contains_key(&key) || mgr.queued.contains(&key) {
                continue;
            }
            to_spawn.push((key, prio));

            if !mgr.gpu_meshes.contains_key(&key) {
                mesh_jobs.push((key, compute_segments(key.2), None));
            }
            if !cache.contains_key(&key)
                && !mgr.gpu_textures.contains_key(&key)
                && !mgr.no_data.contains(&key)
                && !mgr.in_flight.contains(&key)
            {
                // Tiles projected far below native texel size are pure
                // horizon filler: downscale on the worker thread so they
                // never pay full-res decode/upload costs. Threshold stays
                // well below 256 so a downscaled tile never sits right next
                // to a full-res one at comparable screen size.
                downloads.push((key, prio < 160.0, prio));
                mgr.in_flight.insert(key);
            }
        }
    }

    to_spawn.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (key, _) in to_spawn {
        mgr.spawn_queue.push_back(key);
        mgr.queued.insert(key);
    }

    downloads.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let downloads: Vec<(TileKey, bool)> =
        downloads.into_iter().map(|(k, d, _)| (k, d)).collect();

    // Immediately mark the fresh queue as worth downloading so workers that
    // start before the next end-of-frame `wanted` refresh don't skip it.
    {
        let mut w = tex_rx.wanted.lock().unwrap();
        w.extend(mgr.queued.iter().copied());
    }

    if !mesh_jobs.is_empty() {
        start_mesh_builds(mesh_pipe, mesh_jobs);
    }
    if !downloads.is_empty() {
        start_downloads(tex_rx, &downloads);
    }
}

// ── View-dependent update ──────────────────────────────────────────

fn view_dependent_update(
    orbit: Res<OrbitState>,
    windows: Query<&Window>,
    mut mgr: ResMut<TileManager>,
    mut mesh_pipe: ResMut<MeshPipeline>,
    tex_rx: Res<TextureReceiver>,
    mut commands: Commands,
) {
    if !mgr.initialized {
        return;
    }

    let (lat_rad, lon_rad) = compute_sub_camera_point(&orbit);

    // Recompute trigger: sub-camera point moved ~1 tile at a mid reference
    // zoom, or the camera distance changed enough (>3%) to flip SSE
    // subdivisions. Reference zoom is clamped so global views refresh often
    // enough while close-in views simply recompute every drag frame (the
    // traversal is cheap and GPU caches absorb any churn).
    let ref_z = mgr.current_zoom.clamp(4, 12);
    let (center_tx, center_ty) = geo_to_tile(lat_rad, lon_rad, ref_z);
    let dx = (center_tx as f64) - mgr.last_center_x;
    let dy = (center_ty as f64) - mgr.last_center_y;
    let moved = (dx * dx + dy * dy).sqrt();
    let dist_changed =
        (orbit.distance - mgr.last_distance).abs() > mgr.last_distance * 0.03;

    if moved <= 0.5 && !dist_changed {
        return;
    }

    let new_visible = compute_visible_tiles(
        lat_rad,
        lon_rad,
        orbit.distance as f64,
        focal_pixels(&windows),
    );
    let finest = new_visible
        .iter()
        .map(|t| t.0 .2)
        .max()
        .unwrap_or(mgr.current_zoom);
    let finest_changed = finest != mgr.current_zoom;
    let new_set: HashSet<TileKey> = new_visible.iter().map(|&(k, _)| k).collect();
    mgr.visible_set = new_set;

    // Stale tiles are NOT despawned here: they keep covering the globe while
    // their replacements load (no holes / no spawn-despawn churn during
    // drags). `process_pipeline` cleans them up progressively.
    enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &new_visible);

    mgr.current_zoom = finest;
    mgr.last_distance = orbit.distance;
    mgr.last_center_x = center_tx as f64;
    mgr.last_center_y = center_ty as f64;
    mgr.view_changed_this_frame = true;

    if finest_changed {
        // Drop tiles far from the new LOD range right away (budgeted):
        // otherwise a multi-level cascade leaves several full zoom levels of
        // overlapping entities alive and inflates the draw-call count.
        let prot = protected_ancestors(&mgr);
        let mut evicted = 0;
        let far: Vec<TileKey> = mgr
            .tile_entities
            .keys()
            .filter(|k| {
                // Never despawn tiles the current view still needs: opening
                // a hole at the limb exposes hanging skirt walls as stripe
                // fins while the replacement loads. Likewise keep every
                // ancestor of a not-yet-spawned visible tile (fast zooms
                // jump several LOD levels at once), otherwise the hole
                // falls through to the blue base sphere mid-zoom.
                !mgr.visible_set.contains(k)
                    && !prot.contains(k)
                    && k.2 > BASE_LAYER_ZOOM
                    && k.2.abs_diff(finest) > 2
            })
            .copied()
            .collect();
        for key in far {
            if evicted >= MAX_DESPAWNS_PER_FRAME {
                break;
            }
            mgr.despawn_tile(&key, &mut commands);
            evicted += 1;
        }
    }

    // Radial tuck is re-derived continuously from the camera by
    // `smooth_tuck` every frame; nothing to do here.
}

impl TileManager {
    /// Despawn one tile entity; GPU handles stay cached for cheap re-spawn.
    fn despawn_tile(&mut self, key: &TileKey, commands: &mut Commands) {
        if let Some(entity) = self.tile_entities.remove(key) {
            commands.entity(entity).despawn();
            self.textured_tiles.remove(key);
            self.spawn_order.retain(|k| k != key);
        }
    }
}

/// Ancestor keys of every visible tile that has no spawned entity yet.
/// Fast zooms can jump several LOD levels in one recompute, so protection
/// must cover the ENTIRE ancestor chain of each pending leaf, not just the
/// direct parent: any tile in this set is still the only coverage for part
/// of the view and must not be despawned (removing it opens a hole down to
/// the blue base sphere).
fn protected_ancestors(mgr: &TileManager) -> HashSet<TileKey> {
    let mut prot: HashSet<TileKey> = HashSet::new();
    for v in mgr
        .visible_set
        .iter()
        .filter(|v| !mgr.tile_entities.contains_key(v))
    {
        let (mut x, mut y, mut z) = *v;
        while z > 0 {
            x >>= 1;
            y >>= 1;
            z -= 1;
            if !prot.insert((x, y, z)) {
                break; // higher ancestors were already registered
            }
        }
    }
    prot
}

// ── Budgeted asset pipeline (mesh upload → spawn → texture → cleanup) ──

fn process_pipeline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut mgr: ResMut<TileManager>,
    mut mesh_pipe: ResMut<MeshPipeline>,
    tex_rx: Res<TextureReceiver>,
    orbit: Res<OrbitState>,
    windows: Query<&Window>,
) {
    if !mgr.initialized {
        return;
    }
    let view_changed = mgr.view_changed_this_frame;
    mgr.view_changed_this_frame = false;

    // 1) Collect finished background meshes into the backlog.
    let drained: Vec<(TileKey, Mesh, bool)> = {
        let rx = mesh_pipe.rx.lock().unwrap();
        let mut v = Vec::new();
        while let Ok(item) = rx.try_recv() {
            v.push(item);
        }
        v
    };
    for item in drained {
        mesh_pipe.backlog.push_back(item);
    }

    // 2) Upload backlog meshes to the GPU within budget.
    let mut mesh_uploads = 0;
    while mesh_uploads < MAX_MESH_UPLOADS_PER_FRAME {
        let Some((key, mesh, is_fb)) = mesh_pipe.backlog.pop_front() else {
            break;
        };
        if (is_fb && mgr.gpu_fb_meshes.contains_key(&key))
            || (!is_fb && mgr.gpu_meshes.contains_key(&key))
        {
            continue;
        }
        // Only spend GPU memory on tiles still wanted.
        if !mgr.visible_set.contains(&key) && !mgr.queued.contains(&key) {
            continue;
        }
        if is_fb {
            mgr.gpu_fb_meshes.insert(key, meshes.add(mesh));
            evict_gpu_cache(&mut mgr);
        } else {
            mgr.gpu_meshes.insert(key, meshes.add(mesh));
            evict_gpu_cache(&mut mgr);
        }
        mesh_uploads += 1;
    }

    // 3) Spawn entities whose mesh handle is ready, within budget.
    let scale = render_scale();
    let zf = float_zoom(focal_pixels(&windows), orbit.distance);
    let step = adaptive_tuck_step(orbit.distance);
    let pending: Vec<TileKey> = mgr.spawn_queue.drain(..).collect();
    let mut still_queued: Vec<TileKey> = Vec::new();
    let mut spawns = 0;

    let cache = tex_rx.cache.lock().unwrap();
    for key in pending {
        let already = mgr.tile_entities.contains_key(&key);

        if already || !mgr.queued.remove(&key) {
            continue;
        }

        let nodata = mgr.no_data.contains(&key);
        let mesh_ready = if nodata {
            if mgr.solid_tiles.contains(&key) {
                mgr.gpu_meshes.contains_key(&key)
            } else {
                mgr.gpu_fb_meshes.contains_key(&key)
            }
        } else {
            mgr.gpu_meshes.contains_key(&key)
        };

        if !mesh_ready || spawns >= MAX_SPAWNS_PER_FRAME {
            still_queued.push(key);
            mgr.queued.insert(key);
            continue;
        }

        // Only spawn tiles that are still part of the visible set.
        if !mgr.visible_set.contains(&key) {
            continue;
        }

        if nodata {
            // No imagery exists for this tile (Bing placeholder or download
            // failure): render it with the nearest ancestor's texture
            // upsampled through a UV-remapped mesh (CesiumJS upsampling
            // fallback). A solid-blue quad is NEVER spawned while a live
            // ancestor entity still covers the region: the parent simply
            // keeps showing until something real replaces it (coverage
            // repair re-checks this tile every stable frame).
            if mgr.solid_tiles.contains(&key) && has_live_ancestor(&mgr, &key) {
                continue;
            }
            let mesh_handle = if mgr.solid_tiles.contains(&key) {
                mgr.gpu_meshes[&key].clone()
            } else {
                mgr.gpu_fb_meshes[&key].clone()
            };
            let material = if let Some(mat) = mgr.gpu_materials.get(&key) {
                mat.clone()
            } else if let Some(tex) = mgr.effective_tex.get(&key).cloned() {
                let mat = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(tex),
                    perceptual_roughness: 0.9,
                    // Double-sided so the hanging skirt wall is visible
                    // through cracks from either side.
                    cull_mode: None,
                    ..default()
                });
                mgr.gpu_materials.insert(key, mat.clone());
                mat
            } else {
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.01, 0.05, 0.10),
                    perceptual_roughness: 1.0,
                    cull_mode: None,
                    ..default()
                });
                mgr.gpu_materials.insert(key, mat.clone());
                mat
            };

            let entity = commands
                .spawn((
                    CesiumGlobe,
                    GlobeTile {
                        x: key.0,
                        y: key.1,
                        z: key.2,
                    },
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material),
                    Transform::from_scale(Vec3::splat(
                        scale * level_tuck(key.2, zf, step) as f32,
                    )),
                ))
                .id();

            mgr.tile_entities.insert(key, entity);
            mgr.spawn_order.push_back(key);
            mgr.textured_tiles.insert(key);
            spawns += 1;
            continue;
        }

        // Real imagery path. Texture: GPU handle > CPU rgba cache. A tile
        // whose imagery is not ready yet stays HIDDEN (re-queued) instead of
        // spawning as a blue placeholder: an opaque blue quad would paint a
        // hole over the already-textured coarser parent, which reads far
        // worse than letting the parent show through until the child is
        // ready (the same rule CesiumJS follows for not-yet-loaded children).
        let cached_tex = mgr
            .gpu_textures
            .get(&key)
            .cloned()
            .or_else(|| {
                cache.get(&key).map(|c| {
                    make_image(&mut images, c.rgba_data.clone(), c.width, c.height)
                })
            });
        let Some(tex) = cached_tex else {
            still_queued.push(key);
            mgr.queued.insert(key);
            continue;
        };
        mgr.effective_tex.insert(key, tex.clone());

        let mesh_handle = mgr.gpu_meshes[&key].clone();

        let material = if let Some(mat) = mgr.gpu_materials.get(&key) {
            mat.clone()
        } else {
            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(tex.clone()),
                perceptual_roughness: 0.9,
                cull_mode: None,
                ..default()
            });
            mgr.gpu_materials.insert(key, mat.clone());
            mat
        };

        if !mgr.gpu_textures.contains_key(&key) {
            mgr.gpu_tex_order.push_back(key);
        }
        mgr.gpu_textures.insert(key, tex.clone());
        evict_gpu_cache(&mut mgr);

        let entity = commands
            .spawn((
                CesiumGlobe,
                GlobeTile {
                    x: key.0,
                    y: key.1,
                    z: key.2,
                },
                Mesh3d(mesh_handle),
                MeshMaterial3d(material),
                Transform::from_scale(Vec3::splat(
                    scale * level_tuck(key.2, zf, step) as f32,
                )),
            ))
            .id();

        mgr.tile_entities.insert(key, entity);
        mgr.spawn_order.push_back(key);
        mgr.textured_tiles.insert(key);
        spawns += 1;
    }
    drop(cache);

    mgr.spawn_queue = still_queued.into();

    // 4) Apply downloaded textures within budget (rest stays in the channel).
    let mut tex_uploads = 0;
    while tex_uploads < MAX_TEXTURE_UPLOADS_PER_FRAME {
        let Ok(result) = tex_rx.rx.lock().unwrap().try_recv() else {
            break;
        };
        let key = (result.x, result.y, result.z);
        mgr.in_flight.remove(&key);
        if result.aborted {
            // Worker skipped the fetch (tile left the wanted set mid-flight);
            // nothing to apply, the slot is simply free again.
            continue;
        }
        if result.placeholder {
            mgr.reupload.remove(&key);
            if mgr.gpu_textures.contains_key(&key) {
                // Already displaying real (low-res) imagery for this tile;
                // a placeholder verdict on the full-res re-fetch must not
                // flip it to no-data (that would stamp ancestor coverage
                // over good pixels).
                tex_uploads += 1;
                continue;
            }
            // Permanently no-data tile: inherit the nearest textured
            // ancestor (UV-upsampled) or fall back to solid ocean, so the
            // region never renders as a hole down to the base sphere.
            if mgr.no_data.insert(key) {
                let (mut ax, mut ay, mut az) = key;
                let mut ancestor: Option<(TileKey, Handle<Image>)> = None;
                while az > 0 {
                    ax >>= 1;
                    ay >>= 1;
                    az -= 1;
                    if let Some(t) = mgr.effective_tex.get(&(ax, ay, az)) {
                        ancestor = Some(((ax, ay, az), t.clone()));
                        break;
                    }
                }
                if let Some((anc, tex)) = ancestor {
                    let dz = key.2 - anc.2;
                    let side = (1u32 << dz) as f32;
                    let rx = (key.0 % (1u32 << dz)) as f32;
                    let ry = (key.1 % (1u32 << dz)) as f32;
                    let uv = [
                        rx / side,
                        ry / side,
                        (rx + 1.0) / side,
                        (ry + 1.0) / side,
                    ];
                    mgr.effective_tex.insert(key, tex);
                    if !mgr.gpu_fb_meshes.contains_key(&key) {
                        start_mesh_builds(
                            &mut mesh_pipe,
                            vec![(key, compute_segments(key.2), Some(uv))],
                        );
                    }
                } else {
                    // No textured ancestor: inherit whatever the closest
                    // LIVE ancestor entity displays (even a solid-ocean
                    // tile) so this tile never paints a fresh blue block.
                    let mut ix = key.0;
                    let mut iy = key.1;
                    let mut iz = key.2;
                    let mut inherited = false;
                    while iz > 0 {
                        ix >>= 1;
                        iy >>= 1;
                        iz -= 1;
                        if mgr.tile_entities.contains_key(&(ix, iy, iz)) {
                            if let Some(mat) =
                                mgr.gpu_materials.get(&(ix, iy, iz)).cloned()
                            {
                                mgr.gpu_materials.insert(key, mat);
                                inherited = true;
                            }
                            break;
                        }
                    }
                    if !inherited {
                        mgr.solid_tiles.insert(key);
                    }
                }
            }
            // Make sure the tile (re)enters the spawn queue: it may have
            // been dropped from it earlier (e.g. momentarily outside the
            // visible set), and without a spawn the region would remain a
            // hole down to the base sphere.
            if !mgr.tile_entities.contains_key(&key) && !mgr.queued.contains(&key) {
                mgr.spawn_queue.push_back(key);
                mgr.queued.insert(key);
            }
            tex_uploads += 1;
            continue;
        }
        // No CPU-side rgba cache: the GPU texture cache below survives
        // despawn, so keeping full-res bytes in RAM would be pure waste.

        let tex_handle = if matches!(
            (mgr.gpu_textures.get(&key), mgr.gpu_tex_size.get(&key)),
            (Some(_), Some(&sz)) if sz >= result.width
        ) {
            mgr.gpu_textures[&key].clone()
        } else {
            // New tile, or a full-res re-download replacing a downscaled
            // horizon-filler texture: swap the handle so the live entity
            // sharpens in place without a respawn.
            let h = make_image(&mut images, result.rgba_data, result.width, result.height);
            if !mgr.gpu_textures.contains_key(&key) {
                mgr.gpu_tex_order.push_back(key);
            }
            mgr.gpu_textures.insert(key, h.clone());
            mgr.gpu_tex_size.insert(key, result.width);
            evict_gpu_cache(&mut mgr);
            h
        };
        mgr.reupload.remove(&key);
        mgr.effective_tex.insert(key, tex_handle.clone());

        if let Some(mat_handle) = mgr.gpu_materials.get(&key) {
            if let Some(mat) = materials.get_mut(mat_handle) {
                mat.base_color_texture = Some(tex_handle);
                mat.base_color = Color::WHITE;
            }
        }
        if mgr.tile_entities.contains_key(&key) {
            mgr.textured_tiles.insert(key);
        }
        tex_uploads += 1;
    }

    // 5) Budgeted cleanup. Two rules keep this cheap and churn-free:
    //    - it only runs on STABLE frames (no visible-set recompute this
    //      frame), so tiles that briefly leave the set during a drag are not
    //      despawned only to be re-downloaded a moment later;
    //    - despawns are capped per frame so removing hundreds of entities
    //      never produces a CPU spike mid-drag.
    let finest = mgr.current_zoom;
    let fine_total = mgr
        .visible_set
        .iter()
        .filter(|(_, _, z)| *z == finest)
        .count();
    let fine_textured = mgr
        .visible_set
        .iter()
        .filter(|k| k.2 == finest && mgr.textured_tiles.contains(k))
        .count();
    let stable = !view_changed;
    let warmed_up = fine_total > 0 && fine_textured > fine_total / 5;
    let over_cap = mgr.tile_entities.len() > MAX_TILE_ENTITIES;

    if over_cap || (stable && warmed_up) {
        // (insertion position, key) pairs, oldest first. Sorted inside a
        // block so the borrow of `mgr` ends before despawning begins.
        let mut stale: Vec<(usize, TileKey)> = {
            let pos = |k: &TileKey| {
                mgr.spawn_order
                    .iter()
                    .position(|o| o == k)
                    .unwrap_or(usize::MAX)
            };
            let prot = protected_ancestors(&mgr);
            let mut v: Vec<(usize, TileKey)> = mgr
                .tile_entities
                .keys()
                .copied()
                .filter(|k| !mgr.visible_set.contains(k) && !prot.contains(k) && k.2 > BASE_LAYER_ZOOM)
                .map(|k| (pos(&k), k))
                .collect();
            v.sort();
            v
        };

        let mut removed = 0;
        for (_, key) in stale.drain(..) {
            if removed >= MAX_DESPAWNS_PER_FRAME {
                break;
            }
            mgr.despawn_tile(&key, &mut commands);
            removed += 1;
        }

        // Still over the hard cap: evict the oldest surplus entities even if
        // visible (draw-call pressure beats coverage; GPU handles stay
        // cached so they re-spawn instantly if the view returns). Tiles
        // with a live ancestor are evicted first: their region stays
        // covered by the parent, so no blue hole opens.
        if mgr.tile_entities.len() > MAX_TILE_ENTITIES {
            let mut extras: Vec<(usize, TileKey)> = {
                let pos = |k: &TileKey| {
                    mgr.spawn_order
                        .iter()
                        .position(|o| o == k)
                        .unwrap_or(usize::MAX)
                };
                let mut v: Vec<(usize, TileKey)> = mgr
                    .tile_entities
                    .keys()
                    .copied()
                    .filter(|k| k.2 > BASE_LAYER_ZOOM && has_live_ancestor(&mgr, k))
                    .map(|k| (pos(&k), k))
                    .collect();
                v.sort();
                v
            };
            for (_, key) in extras.drain(..) {
                if mgr.tile_entities.len() <= MAX_TILE_ENTITIES
                    || removed >= MAX_DESPAWNS_PER_FRAME * 2
                {
                    break;
                }
                mgr.despawn_tile(&key, &mut commands);
                removed += 1;
            }
        }

        if removed > 0 {
            println!(
                "[DynGlobe] Cleaned {} tiles, {} remaining",
                removed,
                mgr.tile_entities.len()
            );
        }
    }

    // Coverage repair: visible tiles can fall out of the spawn queue through
    // assorted paths (momentarily outside the set mid-drag, over-cap
    // eviction of a still-visible tile). On stable frames re-enqueue them so
    // coverage always converges back to hole-free. Re-enqueueing alone is not
    // enough: tiles that re-enter the queue through this path (or whose
    // in-flight fetch was aborted by the wanted check) would otherwise wait
    // forever on a download that was never started, so missing mesh jobs and
    // downloads are issued here as well.
    if !view_changed {
        let missing: Vec<TileKey> = mgr
            .visible_set
            .iter()
            .filter(|k| !mgr.tile_entities.contains_key(k) && !mgr.queued.contains(k))
            .copied()
            .take(32)
            .collect();
        // Queued visible tiles that have no texture and no download in
        // flight either (starved by an aborted fetch): re-issue the download.
        let starved: Vec<TileKey> = {
            let cache = tex_rx.cache.lock().unwrap();
            mgr.visible_set
                .iter()
                .filter(|k| {
                    mgr.queued.contains(k)
                        && !mgr.tile_entities.contains_key(k)
                        && !mgr.in_flight.contains(k)
                        && !mgr.no_data.contains(k)
                        && !mgr.gpu_textures.contains_key(k)
                        && !cache.contains_key(k)
                })
                .copied()
                .collect()
        };
        if !missing.is_empty() || !starved.is_empty() {
            let mut repair_mesh: Vec<(TileKey, u32, Option<[f32; 4]>)> = Vec::new();
            let mut repair_dl: Vec<(TileKey, bool)> = Vec::new();
            {
                let cache = tex_rx.cache.lock().unwrap();
                for k in missing.iter().chain(starved.iter()) {
                    if !mgr.gpu_meshes.contains_key(k)
                        && !mgr.gpu_fb_meshes.contains_key(k)
                    {
                        repair_mesh.push((*k, compute_segments(k.2), None));
                    }
                    if !cache.contains_key(k)
                        && !mgr.gpu_textures.contains_key(k)
                        && !mgr.no_data.contains(k)
                        && !mgr.in_flight.contains(k)
                    {
                        repair_dl.push((*k, false));
                        mgr.in_flight.insert(*k);
                    }
                }
            }
            if !repair_mesh.is_empty() {
                start_mesh_builds(&mut mesh_pipe, repair_mesh);
            }
            if !repair_dl.is_empty() {
                start_downloads(&tex_rx, &repair_dl);
            }
            for k in missing {
                mgr.spawn_queue.push_back(k);
                mgr.queued.insert(k);
            }
        }
    }

    // Refresh the download-wanted set so workers can drop fetches made stale
    // by the view moving on; queued / visible / spawned tiles stay wanted.
    {
        let mut w = tex_rx.wanted.lock().unwrap();
        w.clear();
        w.extend(mgr.queued.iter().copied());
        w.extend(mgr.visible_set.iter().copied());
        w.extend(mgr.tile_entities.keys().copied());
    }
}

/// True when any ancestor of `key` has a spawned entity still covering its
/// region, so `key` can be safely skipped/evicted without opening a hole
/// down to the blue base sphere.
fn has_live_ancestor(mgr: &TileManager, key: &TileKey) -> bool {
    let (mut x, mut y, mut z) = *key;
    while z > 0 {
        x >>= 1;
        y >>= 1;
        z -= 1;
        if mgr.tile_entities.contains_key(&(x, y, z)) {
            return true;
        }
    }
    false
}

/// FIFO-evict the oldest cached GPU handles once over the cap, so a long
/// panning session can't grow the caches without bound. Handles of tiles
/// whose entity is STILL ALIVE are never evicted: freeing the underlying
/// `Assets` data while a spawned entity keeps referencing it corrupts the
/// mesh allocator's bookkeeping, and the tile renders as garbage horizontal
/// stripes (the 花屏 artifact seen during deep zooms). Such entries are
/// pushed to the back and re-checked later; termination is guaranteed
/// because spawned entities are capped at `MAX_TILE_ENTITIES`, far below
/// `MAX_GPU_CACHE_ENTRIES`, so evictable (dead) entries always exist.
fn evict_gpu_cache(mgr: &mut TileManager) {
    while mgr.gpu_tex_order.len() > MAX_GPU_CACHE_ENTRIES {
        let Some(old) = mgr.gpu_tex_order.pop_front() else {
            break;
        };
        if old.2 <= BASE_LAYER_ZOOM {
            // Permanent fallback layer: never evict its GPU handles.
            continue;
        }
        if mgr.tile_entities.contains_key(&old) {
            // Still rendered: defer eviction.
            mgr.gpu_tex_order.push_back(old);
            continue;
        }
        mgr.gpu_textures.remove(&old);
        mgr.gpu_materials.remove(&old);
        mgr.gpu_meshes.remove(&old);
        mgr.gpu_fb_meshes.remove(&old);
        mgr.effective_tex.remove(&old);
        mgr.gpu_tex_size.remove(&old);
    }
}

// ── Screen-space-error quadtree LOD (CesiumJS QuadtreePrimitive) ──

/// Vertical focal length in pixels: (H/2) / tan(fov/2).
fn focal_pixels(windows: &Query<&Window>) -> f64 {
    let h = windows
        .get_single()
        .map(|w| w.height() as f64)
        .unwrap_or(720.0);
    (h * 0.5) / ((CAMERA_FOV_Y as f64) * 0.5).tan()
}

/// CesiumJS-style quadtree traversal: start at the coarsest level and
/// subdivide every visible tile while its projected screen footprint
/// exceeds [`MAX_TILE_SCREEN_PX`] (the maximumScreenSpaceError = 2 budget
/// for 256-px tiles), culling tiles beyond the horizon cap. Returns the
/// selected tiles together with their screen footprint, which doubles as
/// request priority. Resolution thus falls off continuously from the view
/// center to the limb, exactly like CesiumJS globe LOD.
fn compute_visible_tiles(
    lat_rad: f64,
    lon_rad: f64,
    distance: f64,
    focal_px: f64,
) -> Vec<(TileKey, f32)> {
    let d = distance.max(1.001);
    let cx = lat_rad.cos() * lon_rad.cos();
    let cy = lat_rad.cos() * lon_rad.sin();
    let cz = lat_rad.sin();
    let cap = (1.0 / d).acos();

    let mut out = Vec::new();
    let n0 = 1u32 << MIN_ZOOM;
    for y in 0..n0 {
        for x in 0..n0 {
            visit_tile(x, y, MIN_ZOOM, cx, cy, cz, d, cap, focal_px, &mut out);
        }
    }
    out
}

fn visit_tile(
    x: u32,
    y: u32,
    z: u32,
    cx: f64,
    cy: f64,
    cz: f64,
    d: f64,
    cap: f64,
    focal_px: f64,
    out: &mut Vec<(TileKey, f32)>,
) {
    let n = 1u64 << z;
    // Tile center geographic coordinates (y row 0 = north, like mesh UVs).
    let lon = (x as f64 + 0.5) / n as f64 * 2.0 * std::f64::consts::PI
        - std::f64::consts::PI;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * (y as f64 + 0.5) / n as f64))
        .sinh()
        .atan()
        .clamp(-1.4844, 1.4844);

    let tx = lat.cos() * lon.cos();
    let ty = lat.cos() * lon.sin();
    let tz = lat.sin();

    // Horizon cull: tile center beyond the visible cap + one tile margin.
    let dot = (tx * cx + ty * cy + tz * cz).clamp(-1.0, 1.0);
    let theta = dot.acos();
    let margin = 2.0 * std::f64::consts::PI / n as f64;
    if theta > cap + margin {
        return;
    }

    // Camera→tile-center chord distance in meters (globe radius = 1 unit).
    let ex = tx - cx * d;
    let ey = ty - cy * d;
    let ez = tz - cz * d;
    let dist_m = (ex * ex + ey * ey + ez * ez).sqrt() * EARTH_RADIUS_M;

    // Projected screen footprint of the tile's Web Mercator width.
    let w_m = 2.0 * std::f64::consts::PI * EARTH_RADIUS_M / n as f64;
    let screen_px = w_m / dist_m * focal_px;

    if screen_px <= MAX_TILE_SCREEN_PX || z >= MAX_ZOOM {
        out.push(((x, y, z), screen_px as f32));
    } else {
        let (x2, y2, z1) = (x * 2, y * 2, z + 1);
        visit_tile(x2, y2, z1, cx, cy, cz, d, cap, focal_px, out);
        visit_tile(x2 + 1, y2, z1, cx, cy, cz, d, cap, focal_px, out);
        visit_tile(x2, y2 + 1, z1, cx, cy, cz, d, cap, focal_px, out);
        visit_tile(x2 + 1, y2 + 1, z1, cx, cy, cz, d, cap, focal_px, out);
    }
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

/// Per-LOD-level radial tuck against a CONTINUOUS zoom anchor: coarser
/// tiles sit `step` below per (fractional) level difference, finer stale
/// tiles `step` above (signed, so overlapping levels never render coplanar
/// and z-fight). `step` adapts to the camera's height above the surface
/// (~2% of it): the parent/child cliff then stays far below one screen
/// pixel at every altitude (no smeared skirt band at LOD junctions) while
/// remaining far above the depth-buffer precision. The anchor is the
/// fractional zoom level (`float_zoom`), so entity scales drift smoothly
/// with the camera instead of jumping at integer finest flips — the discrete
/// radial jumps read as tile "wobble" while zooming, which CesiumJS never
/// shows because it has no tuck at all. Applied via entity scale (NOT baked
/// into the mesh) so meshes stay reusable across zoom changes.
fn level_tuck(z: u32, z_float: f64, step: f64) -> f64 {
    1.0 - step * (z_float - z as f64)
}

/// Fractional LOD level the view center selects right now, mirroring the
/// `screen_px == MAX_TILE_SCREEN_PX` subdivision threshold of the quadtree
/// traversal: integer crossings coincide with `finest` flips while values
/// in between drift continuously with the camera altitude.
fn float_zoom(focal_px: f64, distance: f32) -> f64 {
    let alt = (distance as f64 - 1.0).max(1.0e-4);
    (2.0 * std::f64::consts::PI * focal_px / (MAX_TILE_SCREEN_PX * alt))
        .log2()
        .clamp(MIN_ZOOM as f64, MAX_ZOOM as f64)
}

/// Re-derive every live tile's radial tuck from the continuous zoom anchor
/// each frame, so scales follow the camera smoothly (no wobble). Writes are
/// skipped while the value is unchanged to avoid change-detection churn on
/// idle frames.
fn smooth_tuck(
    orbit: Res<OrbitState>,
    windows: Query<&Window>,
    mut tiles: Query<(&GlobeTile, &mut Transform)>,
) {
    let zf = float_zoom(focal_pixels(&windows), orbit.distance);
    let step = adaptive_tuck_step(orbit.distance);
    let scale = render_scale();
    for (tile, mut tf) in &mut tiles {
        let s = scale * level_tuck(tile.z, zf, step) as f32;
        if (tf.scale.x - s).abs() > s * 1.0e-6 {
            tf.scale = Vec3::splat(s);
        }
    }
}

/// Camera-adaptive tuck step: 2% of the height above the surface, clamped
/// so it never vanishes (deep zoom) nor exceeds a small fraction of the
/// coarsest tile (horizon).
fn adaptive_tuck_step(distance: f32) -> f64 {
    // Floor 2e-5: the inter-level separation (step * level-delta) must stay
    // several times above depth-buffer precision at every camera altitude,
    // otherwise overlapping LOD levels z-fight as diamond patches; 2e-5 is
    // still sub-pixel (cliff < 0.5 px) down to ~30 km altitude.
    (2.0e-5 * (distance as f64 - 1.0)).clamp(2.0e-5, 2.0e-4)
}

/// Smoothness + palette metric: average / max RGB difference between
/// sampled pixel pairs 4px apart, plus mean channel levels. Real imagery is
/// textured (or at least dark ocean / warm salt flats); Bing's no-imagery
/// placeholder is a smooth BRIGHT COOL-TINTED gradient (measured:
/// center rgb ~(223,230,238), avg ~0.7, max ~9), while real smooth ocean is
/// dark teal (rgb ~(6,36,47)) and salt flats are warm white (R > B).
fn smoothness_stats(rgba: &image::RgbaImage) -> (f64, u32, u32, u32, u32) {
    let (w, h) = rgba.dimensions();
    let mut sum: f64 = 0.0;
    let mut maxd: u32 = 0;
    let mut count: u32 = 0;
    let mut acc_r: u64 = 0;
    let mut acc_g: u64 = 0;
    let mut acc_b: u64 = 0;
    let mut x = 0;
    while x + 4 < w {
        for y in (0..h).step_by(8) {
            let p = rgba.get_pixel(x, y).0;
            let q = rgba.get_pixel(x + 4, y).0;
            let d = ((p[0] as i32 - q[0] as i32).abs()
                + (p[1] as i32 - q[1] as i32).abs()
                + (p[2] as i32 - q[2] as i32).abs()) as u32;
            sum += d as f64;
            if d > maxd {
                maxd = d;
            }
            acc_r += p[0] as u64;
            acc_g += p[1] as u64;
            acc_b += p[2] as u64;
            count += 1;
        }
        x += 8;
    }
    if count == 0 {
        return (f64::MAX, u32::MAX, 0, 0, 0);
    }
    (
        sum / count as f64,
        maxd,
        (acc_r / count as u64) as u32,
        (acc_g / count as u64) as u32,
        (acc_b / count as u64) as u32,
    )
}

/// True when the decoded image is Bing's smooth bright cool-tinted no-imagery
/// placeholder rather than real satellite imagery. Smoothness alone is not
/// enough: real ocean is darker than the placeholder and salt flats / pale
/// sand are warm-tinted, so requiring bright + blue-dominant pixels keeps
/// those tiles on the real-imagery path (misclassifying them used to stamp
/// upsampled ancestor coverage over good terrain, reading as smeared bands
/// at LOD junctions).
fn is_placeholder_tile(rgba: &image::RgbaImage) -> (bool, f64, u32) {
    let (w, h) = rgba.dimensions();
    if w < 16 || h < 16 {
        return (false, f64::MAX, u32::MAX);
    }
    let (avg, maxd, r, _g, b) = smoothness_stats(rgba);
    let bright = (r + b) / 2 > 170;
    let cool = b >= r;
    (avg < 1.5 && maxd <= 12 && bright && cool, avg, maxd)
}

/// Append a box-filtered mip chain (2x2 average per level) to the base
/// level, returning the full data blob and the mip level count.
fn build_mip_chain(base: Vec<u8>, width: u32, height: u32) -> (Vec<u8>, u32) {
    let mut data = base;
    let mut levels = 1u32;
    let (mut cw, mut ch) = (width, height);
    let mut src_off = 0usize;
    while cw > 1 || ch > 1 {
        let nw = (cw / 2).max(1);
        let nh = (ch / 2).max(1);
        let mut mip = vec![0u8; (nw * nh * 4) as usize];
        for ry in 0..nh {
            for rx in 0..nw {
                let mut acc = [0u32; 4];
                for dy in 0..2u32 {
                    for dx in 0..2u32 {
                        let sx = ((rx * 2 + dx).min(cw - 1)) as usize;
                        let sy = ((ry * 2 + dy).min(ch - 1)) as usize;
                        let i = src_off + (sy * cw as usize + sx) * 4;
                        for c in 0..4 {
                            acc[c] += data[i + c] as u32;
                        }
                    }
                }
                let o = ((ry * nw + rx) * 4) as usize;
                for c in 0..4 {
                    mip[o + c] = (acc[c] / 4) as u8;
                }
            }
        }
        src_off += (cw * ch) as usize * 4;
        data.extend_from_slice(&mip);
        cw = nw;
        ch = nh;
        levels += 1;
    }
    (data, levels)
}

/// Create a GPU texture from raw RGBA data with a full mip chain and the
/// CesiumJS imagery sampler (trilinear mipmap + anisotropic filtering), so
/// minified horizon tiles don't shimmer and oblique tiles stay crisp.
fn make_image(
    images: &mut Assets<Image>,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
) -> Handle<Image> {
    let (data, levels) = build_mip_chain(rgba, width, height);
    // Image::new asserts data == base extent size, so construct with the
    // base level only and swap in the full mip chain afterwards (the GPU
    // upload path honors texture_descriptor.mip_level_count).
    let base_len = (width * height * 4) as usize;
    let mut img = Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data[..base_len].to_vec(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    img.data = data;
    img.texture_descriptor.mip_level_count = levels;
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    images.add(img)
}

// ── Tile helpers ───────────────────────────────────────────────────

fn compute_segments(zoom: u32) -> u32 {
    // Coarse tiles still need enough subdivisions to keep the limb
    // silhouette smooth where they meet the horizon.
    (BASE_SEGMENTS >> zoom.saturating_sub(MIN_ZOOM)).max(8)
}

// ── Background mesh builds (parallel) ──────────────────────────────

/// Build tile meshes on worker threads; results flow back through the
/// pipeline channel and are uploaded to the GPU within the frame budget.
fn start_mesh_builds(pipe: &mut MeshPipeline, jobs: Vec<(TileKey, u32, Option<[f32; 4]>)>) {
    let tx = pipe.tx.clone();

    std::thread::spawn(move || {
        let chunks: Vec<Vec<(TileKey, u32, Option<[f32; 4]>)>> = {
            let mut c: Vec<Vec<(TileKey, u32, Option<[f32; 4]>)>> =
                (0..DOWNLOAD_THREADS).map(|_| Vec::new()).collect();
            for (i, job) in jobs.into_iter().enumerate() {
                c[i % DOWNLOAD_THREADS].push(job);
            }
            c
        };

        let mut handles = Vec::new();
        for chunk in chunks {
            let tx = tx.clone();
            handles.push(std::thread::spawn(move || {
                for (key, segments, uv) in chunk {
                    let mesh = match uv {
                        Some(rect) => create_tile_mesh_uv(key.0, key.1, key.2, segments, rect),
                        None => create_tile_mesh(key.0, key.1, key.2, segments),
                    };
                    if tx.send((key, mesh, uv.is_some())).is_err() {
                        return;
                    }
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
    });
}

// ── Bing Maps downloads (parallel) ─────────────────────────────────

fn tile_to_quadkey(x: u32, y: u32, level: u32) -> String {
    let mut qk = String::with_capacity(level as usize);
    for i in (0..level).rev() {
        let mut d = 0u8;
        let mask = 1 << i;
        if (x & mask) != 0 {
            d |= 1;
        }
        if (y & mask) != 0 {
            d |= 2;
        }
        qk.push_str(&d.to_string());
    }
    qk
}

/// Start PARALLEL background downloads using a thread pool. When `downscale`
/// is set the worker also shrinks the image to 128x128 so coarse filler tiles
/// never pay full-resolution decode/upload costs.
fn start_downloads(tex_rx: &TextureReceiver, tiles: &[(TileKey, bool)]) {
    let tiles_owned: Vec<(TileKey, bool)> = tiles.to_vec();
    let tx = tex_rx.tx.clone();
    let wanted = tex_rx.wanted.clone();

    std::thread::spawn(move || {
        let total = tiles_owned.len();

        // Split tiles across DOWNLOAD_THREADS workers
        let chunks: Vec<Vec<(TileKey, bool)>> = {
            let mut c: Vec<Vec<(TileKey, bool)>> =
                (0..DOWNLOAD_THREADS).map(|_| Vec::new()).collect();
            for (i, tile) in tiles_owned.into_iter().enumerate() {
                c[i % DOWNLOAD_THREADS].push(tile);
            }
            c
        };

        let mut handles = Vec::new();
        for chunk in chunks {
            let tx = tx.clone();
            let wanted = wanted.clone();
            handles.push(std::thread::spawn(move || {
                let agent = ureq::AgentBuilder::new()
                    .user_agent("Mozilla/5.0 CesiumRust/0.1")
                    .timeout(std::time::Duration::from_secs(10))
                    .build();

                for &((px, py, pz), downscale) in &chunk {
                    // Fast pans make whole batches stale within a few
                    // frames; skip fetches nobody will look at so the
                    // workers and the server connection budget go to the
                    // current view instead.
                    if !wanted.lock().unwrap().contains(&(px, py, pz)) {
                        let _ = tx.send(TileDownloadResult {
                            x: px,
                            y: py,
                            z: pz,
                            rgba_data: Vec::new(),
                            width: 0,
                            height: 0,
                            placeholder: false,
                            aborted: true,
                        });
                        continue;
                    }
                    let qk = tile_to_quadkey(px, py, pz);
                    let sub = (px + py) % 8;
                    let url = format!(
                        "https://ecn.t{}.tiles.virtualearth.net/tiles/a{}.jpeg?g=14393",
                        sub, qk
                    );
                    // Retry with backoff: tile servers throttle bursty
                    // clients (403/429/timeouts); a permanently failed tile
                    // would otherwise never appear.
                    let mut delivered = false;
                    for attempt in 0..3u32 {
                        if attempt > 0 {
                            std::thread::sleep(std::time::Duration::from_millis(
                                250u64 << attempt,
                            ));
                        }
                        let fetched = match agent.get(&url).call() {
                            Ok(resp) => {
                                let mut reader = resp.into_reader();
                                let mut data = Vec::new();
                                if reader.read_to_end(&mut data).is_ok() {
                                    image::load_from_memory(&data).ok()
                                } else {
                                    None
                                }
                            }
                            Err(e) => {
                                eprintln!("[DL] ({},{},{}): {}", px, py, pz, e);
                                None
                            }
                        };
                        if let Some(img) = fetched {
                            let rgba = img.to_rgba8();
                            // Bing serves smooth gradient placeholder JPEGs
                            // for tiles without imagery; pasting one would
                            // stamp an opaque blue hole over the good parent
                            // coverage, so such tiles are treated as no-data
                            // and inherit ancestor coverage on the main
                            // thread instead.
                            if {
                                let (ph, avg, maxd) = is_placeholder_tile(&rgba);
                                if ph {
                                    eprintln!(
                                        "[DL] placeholder ({},{},{}) avg={:.2} max={}",
                                        px, py, pz, avg, maxd
                                    );
                                }
                                ph
                            } {
                                let _ = tx.send(TileDownloadResult {
                                    x: px,
                                    y: py,
                                    z: pz,
                                    rgba_data: Vec::new(),
                                    width: 0,
                                    height: 0,
                                    placeholder: true,
                                    aborted: false,
                                });
                                delivered = true;
                                break;
                            }
                            let (rgba, w, h) = if downscale && rgba.width() > 128 {
                                let small = image::DynamicImage::ImageRgba8(rgba)
                                    .resize(
                                        128,
                                        128,
                                        image::imageops::FilterType::Triangle,
                                    );
                                let r = small.to_rgba8();
                                let (w, h) = r.dimensions();
                                (r, w, h)
                            } else {
                                let (w, h) = rgba.dimensions();
                                (rgba, w, h)
                            };
                            let _ = tx.send(TileDownloadResult {
                                x: px,
                                y: py,
                                z: pz,
                                rgba_data: rgba.into_raw(),
                                width: w,
                                height: h,
                                placeholder: false,
                                aborted: false,
                            });
                            delivered = true;
                            break;
                        }
                    }
                    if !delivered {
                        // All retries failed: treat as no-data (inherit
                        // ancestor coverage) instead of leaving a hole.
                        let _ = tx.send(TileDownloadResult {
                            x: px,
                            y: py,
                            z: pz,
                            rgba_data: Vec::new(),
                            width: 0,
                            height: 0,
                            placeholder: true,
                            aborted: false,
                        });
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
