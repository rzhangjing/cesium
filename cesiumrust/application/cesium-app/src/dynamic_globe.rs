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
/// Hard cap on live tile entities (visible + hidden warm fallback). Hidden
/// tiles draw nothing (Visibility::Hidden), so the cap only bounds entity /
/// handle storage, not draw calls — the visible peak alone is ~750 for the
/// concentric LOD layout; the extra headroom keeps recently-hidden tiles
/// alive so a zoom-out re-partitions onto live coarse tiles instead of
/// flashing down to the base layer.
const MAX_TILE_ENTITIES: usize = 1800;
/// Coarsest levels kept resident as a permanent global fallback layer
/// (CesiumJS's base imagery layer role): downloaded once at startup, never
/// despawned/evicted, so fast pans into never-visited regions show blurry
/// imagery instead of the black base sphere while fine tiles load.
const BASE_LAYER_ZOOM: u32 = 3;
/// Upper bound for the per-tile GPU handle caches (textures dominate at
/// 256 KB each); oldest entries are evicted FIFO.
const MAX_GPU_CACHE_ENTRIES: usize = 3000;

const MAX_TILE_SCREEN_PX: f64 = 288.0;

type TileKey = (u32, u32, u32);

/// Cached tile image data (mip-chained RGBA bytes + dimensions).
struct CachedTexture {
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
    mip_levels: u32,
}

// ── Resources ──────────────────────────────────────────────────────

#[derive(Resource)]
struct TileManager {
    /// Currently-spawned tile entities.
    tile_entities: HashMap<TileKey, Entity>,
    /// Which of the spawned tiles have textures applied (no longer blue).
    textured_tiles: HashSet<TileKey>,
    current_zoom: u32,
    /// Camera distance (globe radii) at the last visible-set recompute;
    /// a relative change re-triggers the SSE traversal.
    last_distance: f32,
    initialized: bool,
    /// The last computed render partition (CesiumJS REPLACE refinement):
    /// exactly one tile per screen region, no parent/child overlap. Only
    /// these tiles are drawn (`sync_visibility` hides everything else).
    visible_set: HashSet<TileKey>,
    /// Pure quadtree candidate partition (before coarsening-KICK display
    /// retention); drives spawn/download work and the changed-frame check.
    partition_set: HashSet<TileKey>,
    /// Tiles that are still loading but NOT in the render partition:
    /// the four children of a KICK-blocked tile (CesiumJS "continue to load
    /// them" rule). They are downloaded/spawned here so the quadtree can
    /// swap the parent out for them as soon as all four are ready.
    load_set: HashSet<TileKey>,
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
    /// UV region of `effective_tex` this tile displays ([0,0,1,1] for tiles
    /// showing their own texture). Inheritance composes these regions so a
    /// grandchild upsampling through an already-upsampled parent samples the
    /// correct part of the grandparent's texture (no shifted-copy seams).
    effective_uv: HashMap<TileKey, [f32; 4]>,
    /// No-data tiles with no textured ancestor: rendered as solid ocean.
    solid_tiles: HashSet<TileKey>,
    /// Spawned tiles currently displaying an inherited ancestor texture
    /// through a UV-remapped mesh (their own full-res imagery is still
    /// missing or only a downscaled filler). Sharpened in place — mesh and
    /// texture swapped atomically — once the full-res download lands.
    upsampled: HashSet<TileKey>,
    /// UV-remap (fallback) mesh builds currently in flight: dedupe guard so
    /// spawn retries never spam duplicate jobs into the mesh pipeline.
    pending_fb_builds: HashSet<TileKey>,
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
    /// Cooldown deadline for tiles whose last download attempt failed
    /// transiently; fetches are re-issued only after it elapses so a
    /// throttled tile server is not hammered in a tight loop.
    retry_after: HashMap<TileKey, std::time::Instant>,
    /// Insertion order of spawned entities (FIFO eviction order).
    spawn_order: VecDeque<TileKey>,
    /// Hidden-tile LRU: tiles in the order they left the render partition.
    /// They stay alive (hidden, zero draw cost) as warm zoom-out fallback;
    /// cap pressure evicts the oldest-hidden first.
    hide_order: VecDeque<TileKey>,
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
            last_distance: 0.0,
            initialized: false,
            visible_set: HashSet::new(),
            partition_set: HashSet::new(),
            load_set: HashSet::new(),
            spawn_queue: VecDeque::new(),
            queued: HashSet::new(),
            gpu_meshes: HashMap::new(),
            gpu_materials: HashMap::new(),
            gpu_textures: HashMap::new(),
            no_data: HashSet::new(),
            gpu_fb_meshes: HashMap::new(),
            effective_tex: HashMap::new(),
            effective_uv: HashMap::new(),
            solid_tiles: HashSet::new(),
            upsampled: HashSet::new(),
            pending_fb_builds: HashSet::new(),
            gpu_tex_order: VecDeque::new(),
            gpu_tex_size: HashMap::new(),
            reupload: HashSet::new(),
            in_flight: HashSet::new(),
            retry_after: HashMap::new(),
            spawn_order: VecDeque::new(),
            hide_order: VecDeque::new(),
            view_changed_this_frame: false,
        }
    }
}

#[derive(Resource)]
struct TextureReceiver {
    rx: Mutex<mpsc::Receiver<TileDownloadResult>>,
    /// Job feed for the persistent download pool (tile key + downscale flag).
    job_tx: mpsc::Sender<(TileKey, bool)>,
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
        let (job_tx, job_rx) = mpsc::channel::<(TileKey, bool)>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let wanted = Arc::new(Mutex::new(HashSet::new()));
        // Persistent worker pool: threads and their ureq agents (including
        // the HTTP keep-alive connection pool) are created once, so bursty
        // zooms never spawn a thread storm that starves the frame thread,
        // and full-res re-fetches reuse warm server connections.
        for _ in 0..DOWNLOAD_THREADS {
            let job_rx = job_rx.clone();
            let tx = tx.clone();
            let wanted = wanted.clone();
            std::thread::spawn(move || download_worker(job_rx, tx, wanted));
        }
        Self {
            rx: Mutex::new(rx),
            job_tx,
            cache,
            wanted,
        }
    }
}

struct TileDownloadResult {
    x: u32,
    y: u32,
    z: u32,
    /// Base level + full mip chain (built on the worker thread).
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
    /// Number of mip levels packed into `rgba_data` (0 when no data).
    mip_levels: u32,
    /// True when the tile has no usable imagery (Bing placeholder or all
    /// retries failed); the main thread then inherits ancestor coverage.
    placeholder: bool,
    /// True when the worker skipped the fetch because the tile left the
    /// wanted set (view moved on); the main thread only clears `in_flight`.
    aborted: bool,
    /// True when every retry failed (transient throttle/timeout): the tile
    /// is NOT no-data; the main thread cools it down and re-issues the
    /// fetch later instead of stamping permanent ancestor coverage.
    failed: bool,
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
            .init_resource::<BaseSphereComposite>()
            .add_systems(Startup, initial_spawn)
            .add_systems(
                Update,
                (view_dependent_update, process_pipeline, sync_visibility).chain(),
            )
            .add_systems(Update, base_sphere_composite_system);
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
    let (visible, load) = compute_visible_tiles(
        lat_rad,
        lon_rad,
        orbit.distance as f64,
        focal_pixels(&windows),
        &mgr,
    );
    let finest = visible.iter().map(|t| t.0 .2).max().unwrap_or(MIN_ZOOM);
    mgr.visible_set = visible.iter().map(|&(k, _)| k).collect();
    mgr.partition_set = mgr.visible_set.clone();
    mgr.load_set = load.iter().map(|&(k, _)| k).collect();

    enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &visible);
    enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &load);

    // Permanent coarse fallback layer: every tile up to BASE_LAYER_ZOOM
    // (fetched at full res: the whole globe costs ~30 MB and stays crisp).
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
    mgr.initialized = true;
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
            // Upsampled tiles (displaying a stretched ancestor tint) are
            // exempt from the 192-px gate: horizon tiles never grow that
            // large, so without the exemption they would keep the ancestor
            // color forever and form a visible color-block ring at the
            // filler boundary (and pop-shift when a zoom-in crosses it).
            if !mgr.no_data.contains(&key)
                && !mgr.reupload.contains(&key)
                && (prio > 192.0 || mgr.upsampled.contains(&key))
                && mgr.gpu_tex_size.get(&key).map_or(true, |&sz| sz < 256)
                && mgr
                    .retry_after
                    .get(&key)
                    .map_or(true, |t| std::time::Instant::now() >= *t)
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
                && mgr
                    .retry_after
                    .get(&key)
                    .map_or(true, |t| std::time::Instant::now() >= *t)
            {
                // Only tiles projected far below native texel size are pure
                // horizon filler and get downscaled on the worker thread.
                // The threshold must stay far below the 192-px full-res
                // re-upload trigger: a tile fetched downscaled at 100-160 px
                // would cross the trigger after a couple of zoom notches and
                // sit blurry next to sharp neighbors until the re-fetch
                // lands (the smeared band / "tiles shifted" zoom artifact).
                downloads.push((key, prio < 64.0, prio));
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
) {
    if !mgr.initialized {
        return;
    }

    let (lat_rad, lon_rad) = compute_sub_camera_point(&orbit);

    // The quadtree re-runs EVERY frame (CesiumJS `QuadtreePrimitive` does
    // the same): a KICK-blocked parent must swap out the moment all four
    // children finish spawning, so a settled camera cannot freeze the
    // partition. The traversal is cheap arithmetic over a few hundred tiles
    // and the GPU caches absorb the churn.
    let (new_visible, new_load) = compute_visible_tiles(
        lat_rad,
        lon_rad,
        orbit.distance as f64,
        focal_pixels(&windows),
        &mgr,
    );
    let finest = new_visible
        .iter()
        .map(|t| t.0 .2)
        .max()
        .unwrap_or(mgr.current_zoom);
    let new_set: HashSet<TileKey> = new_visible.iter().map(|&(k, _)| k).collect();
    let new_load_set: HashSet<TileKey> = new_load.iter().map(|&(k, _)| k).collect();

    // CesiumJS-style render-list stability (`allAreRenderable`): the
    // displayed partition swaps per region ONLY when the replacement is
    // fully drawable with real pixels. Refinement holds the parent until
    // ALL covering descendants are ready (otherwise a moving mosaic of
    // soft/sharp blocks reads as tiles sliding during fast zooms);
    // coarsening holds sharp children until the coarse leaf is ready.
    // Not-ready replacements stay hidden (blocked) so parent and children
    // never overlap and never z-fight; regions without prior coverage
    // (pans into fresh territory) fill in as soon as their tiles spawn.
    let old_set = std::mem::take(&mut mgr.visible_set);
    let mut refine_cover: HashMap<TileKey, Vec<TileKey>> = HashMap::new();
    for n in &new_set {
        let (mut ax, mut ay, mut az) = *n;
        while az > 0 {
            ax >>= 1;
            ay >>= 1;
            az -= 1;
            let a = (ax, ay, az);
            if old_set.contains(&a) && !new_set.contains(&a) {
                refine_cover.entry(a).or_default().push(*n);
                break;
            }
        }
    }
    let mut display: HashSet<TileKey> = HashSet::new();
    let mut blocked: HashSet<TileKey> = HashSet::new();
    for old in old_set.iter() {
        if new_set.contains(old) {
            display.insert(*old);
            continue;
        }
        // Coarsening: nearest ancestor selected by the new partition.
        let (mut ax, mut ay, mut az) = *old;
        let mut ancestor: Option<TileKey> = None;
        while az > 0 {
            ax >>= 1;
            ay >>= 1;
            az -= 1;
            if new_set.contains(&(ax, ay, az)) {
                ancestor = Some((ax, ay, az));
                break;
            }
        }
        if let Some(a) = ancestor {
            if replacement_ready(&mgr, &a) {
                display.insert(a);
            } else {
                display.insert(*old);
                blocked.insert(a);
            }
            continue;
        }
        // Refinement: hold the parent until every covering descendant
        // (at any depth) is ready; then swap the whole region at once.
        if let Some(desc) = refine_cover.get(old) {
            if desc.iter().all(|d| replacement_ready(&mgr, d)) {
                display.extend(desc.iter().copied());
            } else {
                display.insert(*old);
                blocked.extend(desc.iter().copied());
            }
            continue;
        }
        // Region left the partition (horizon-culled) — drop it.
    }
    // Fresh region with no prior coverage: tiles appear once spawned.
    for n in &new_set {
        if blocked.contains(n) || display.contains(n) {
            continue;
        }
        let (mut ax, mut ay, mut az) = *n;
        let mut covered = old_set.contains(n);
        while !covered && az > 0 {
            ax >>= 1;
            ay >>= 1;
            az -= 1;
            covered = old_set.contains(&(ax, ay, az));
        }
        if !covered {
            display.insert(*n);
        }
    }
    let partition_changed = new_set != mgr.partition_set || new_load_set != mgr.load_set;
    mgr.partition_set = new_set.clone();
    mgr.visible_set = display;
    mgr.load_set = new_load_set;

    // Stale tiles are NOT despawned here: they stay alive but HIDDEN as warm
    // fallback (zero draw cost under strict partition rendering), so a
    // zoom-out re-partitions onto live coarse tiles instead of flashing down
    // to the base layer while replacements re-spawn. Only hard-cap pressure
    // evicts them, oldest-hidden first (see `process_pipeline`).
    enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &new_visible);
    enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &new_load);

    // Maintain the hidden-tile LRU: drop re-activated / despawned entries,
    // then append tiles that just left the partition (oldest-hidden first).
    let (vis, load, ents, hide) = {
        let m = &mut *mgr;
        (&m.visible_set, &m.load_set, &m.tile_entities, &mut m.hide_order)
    };
    hide.retain(|k| !vis.contains(k) && !load.contains(k) && ents.contains_key(k));
    let queued_hide: HashSet<TileKey> = hide.iter().copied().collect();
    for k in ents.keys() {
        if !vis.contains(k) && !load.contains(k) && !queued_hide.contains(k) {
            hide.push_back(*k);
        }
    }

    mgr.current_zoom = finest;
    mgr.last_distance = orbit.distance;
    mgr.view_changed_this_frame = partition_changed;
}

impl TileManager {
    /// Despawn one tile entity; GPU handles stay cached for cheap re-spawn.
    fn despawn_tile(&mut self, key: &TileKey, commands: &mut Commands) {
        if let Some(entity) = self.tile_entities.remove(key) {
            commands.entity(entity).despawn();
            self.textured_tiles.remove(key);
            self.upsampled.remove(key);
            self.pending_fb_builds.remove(key);
            self.retry_after.remove(key);
            self.spawn_order.retain(|k| k != key);
            self.hide_order.retain(|k| k != key);
        }
    }
}

/// Ancestor keys of every render/load-set tile that has no spawned entity
/// yet. Fast zooms can jump several LOD levels in one recompute, so
/// protection must cover the ENTIRE ancestor chain of each pending leaf,
/// not just the direct parent: any tile in this set is still the only
/// coverage for part of the view and must not be despawned (removing it
/// opens a hole down to the blue base sphere).
fn protected_ancestors(mgr: &TileManager) -> HashSet<TileKey> {
    let mut prot: HashSet<TileKey> = HashSet::new();
    for v in mgr
        .visible_set
        .iter()
        .chain(mgr.load_set.iter())
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
        if !mgr.visible_set.contains(&key)
            && !mgr.load_set.contains(&key)
            && !mgr.queued.contains(&key)
        {
            continue;
        }
        if is_fb {
            mgr.pending_fb_builds.remove(&key);
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
            // Full-res path needs the real mesh; the ancestor-upsample path
            // below displays through a UV-remapped fallback mesh instead.
            mgr.gpu_meshes.contains_key(&key) || mgr.gpu_fb_meshes.contains_key(&key)
        };

        if !mesh_ready || spawns >= MAX_SPAWNS_PER_FRAME {
            still_queued.push(key);
            mgr.queued.insert(key);
            continue;
        }

        // Only spawn tiles still in the render partition or the background
        // load set (children of a KICK-blocked parent). Blocked coarsening
        // ancestors are hidden from the display but must still spawn so
        // they can become ready and release the retained children.
        if !mgr.visible_set.contains(&key)
            && !mgr.load_set.contains(&key)
            && !mgr.partition_set.contains(&key)
        {
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
                    Transform::from_scale(Vec3::splat(scale)),
                ))
                .id();

            mgr.tile_entities.insert(key, entity);
            mgr.spawn_order.push_back(key);
            mgr.textured_tiles.insert(key);
            spawns += 1;
            continue;
        }

        // Real imagery path. A tile only ever displays its OWN texture once
        // it is full resolution (>= 256 px): with its own full-res texels it
        // spawns on the regular mesh. Until then it inherits the nearest
        // textured ancestor through a UV-remapped mesh (CesiumJS upsampling
        // fallback) — the child's pixels are then EXACTLY the parent's, so
        // the KICK partition swap is invisible and a zoom never shows pale
        // stretched-filler patches next to sharp tiles (the "color patches"
        // artifact) nor a blur->sharp pop at every level swap (the zoom
        // shake). The tile sharpens in place once — mesh and texture swapped
        // atomically — when its own full-res download lands. Base-layer
        // tiles are exempt: they are the KICK fallback roots and must always
        // draw (solid deep-ocean default until their download lands).
        let cached_tex = mgr
            .gpu_textures
            .get(&key)
            .cloned()
            .or_else(|| {
                cache.get(&key).map(|c| {
                    make_image(
                        &mut images,
                        c.rgba_data.clone(),
                        c.width,
                        c.height,
                        c.mip_levels,
                    )
                })
            });
        let own_width = mgr
            .gpu_tex_size
            .get(&key)
            .copied()
            .or_else(|| cache.get(&key).map(|c| c.width));
        let own_full_res = cached_tex.is_some()
            && (key.2 <= BASE_LAYER_ZOOM || own_width.map_or(false, |w| w >= 256));

        if !own_full_res {
            if key.2 <= BASE_LAYER_ZOOM {
                // Base root without imagery yet: fall through to the solid
                // deep-ocean spawn below.
            } else if mgr.upsampled.contains(&key) {
                // Already displaying an inherited ancestor texture.
                still_queued.push(key);
                mgr.queued.insert(key);
                continue;
            } else {
                // Inherit the nearest textured ancestor (same mechanism as
                // the no-data path).
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
                let Some((anc, tex)) = ancestor else {
                    // No textured ancestor yet (base layer still loading):
                    // stay hidden, coverage repair retries once it settles.
                    still_queued.push(key);
                    mgr.queued.insert(key);
                    continue;
                };
                let dz = key.2 - anc.2;
                let side = (1u32 << dz) as f32;
                let rx = (key.0 % (1u32 << dz)) as f32;
                let ry = (key.1 % (1u32 << dz)) as f32;
                // Compose with the ancestor's own UV region (it may itself
                // be upsampling a higher ancestor's texture).
                let [au0, av0, au1, av1] = mgr
                    .effective_uv
                    .get(&anc)
                    .copied()
                    .unwrap_or([0.0, 0.0, 1.0, 1.0]);
                let fu = (au1 - au0) / side;
                let fv = (av1 - av0) / side;
                let uv = [
                    au0 + rx * fu,
                    av0 + ry * fv,
                    au0 + (rx + 1.0) * fu,
                    av0 + (ry + 1.0) * fv,
                ];
                let mesh_handle = if let Some(h) = mgr.gpu_fb_meshes.get(&key) {
                    h.clone()
                } else {
                    if !mgr.pending_fb_builds.contains(&key) {
                        mgr.pending_fb_builds.insert(key);
                        start_mesh_builds(
                            &mut mesh_pipe,
                            vec![(key, compute_segments(key.2), Some(uv))],
                        );
                    }
                    still_queued.push(key);
                    mgr.queued.insert(key);
                    continue;
                };
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
                // Deeper children inherit through this tile from now on.
                mgr.effective_tex.insert(key, tex);
                mgr.effective_uv.insert(key, uv);

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
                        Transform::from_scale(Vec3::splat(scale)),
                    ))
                    .id();

                mgr.tile_entities.insert(key, entity);
                mgr.spawn_order.push_back(key);
                mgr.textured_tiles.insert(key);
                mgr.upsampled.insert(key);
                spawns += 1;
                continue;
            }
        }

        // Own full-res path (or base root): needs the regular mesh handle.
        let Some(mesh_handle) = mgr.gpu_meshes.get(&key).cloned() else {
            still_queued.push(key);
            mgr.queued.insert(key);
            continue;
        };

        if let Some(tex) = &cached_tex {
            mgr.effective_tex.insert(key, tex.clone());
            mgr.effective_uv.insert(key, [0.0, 0.0, 1.0, 1.0]);
        }

        let material = if let Some(mat) = mgr.gpu_materials.get(&key) {
            mat.clone()
        } else if let Some(tex) = cached_tex.clone() {
            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(tex),
                perceptual_roughness: 0.9,
                cull_mode: None,
                ..default()
            });
            mgr.gpu_materials.insert(key, mat.clone());
            mat
        } else {
            // Default image for a base tile whose download is still
            // missing: solid deep ocean, swapped for real imagery in place.
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.01, 0.05, 0.10),
                perceptual_roughness: 1.0,
                cull_mode: None,
                ..default()
            });
            mgr.gpu_materials.insert(key, mat.clone());
            mat
        };

        if let Some(tex) = cached_tex {
            if !mgr.gpu_textures.contains_key(&key) {
                mgr.gpu_tex_order.push_back(key);
            }
            mgr.gpu_textures.insert(key, tex);
            evict_gpu_cache(&mut mgr);
        }

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
                Transform::from_scale(Vec3::splat(scale)),
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
            // nothing to apply, the slot is simply free again. Clear the
            // reupload guard too, or a full-res re-fetch aborted once would
            // never be retried and the tile would stay blurry forever.
            mgr.reupload.remove(&key);
            continue;
        }
        if result.failed {
            // Transient throttle/timeout: cool down and let the normal
            // enqueue / coverage-repair paths re-issue the fetch later.
            // Never stamp no-data here, or the tile keeps the ancestor's
            // coarse tint forever (the rectangular color-block artifact).
            mgr.reupload.remove(&key);
            mgr.retry_after.insert(
                key,
                std::time::Instant::now() + std::time::Duration::from_secs(10),
            );
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
                    // Compose with the ancestor's own UV region: it may
                    // itself be upsampling a higher ancestor's texture.
                    let [au0, av0, au1, av1] = mgr
                        .effective_uv
                        .get(&anc)
                        .copied()
                        .unwrap_or([0.0, 0.0, 1.0, 1.0]);
                    let fu = (au1 - au0) / side;
                    let fv = (av1 - av0) / side;
                    let uv = [
                        au0 + rx * fu,
                        av0 + ry * fv,
                        au0 + (rx + 1.0) * fu,
                        av0 + (ry + 1.0) * fv,
                    ];
                    mgr.effective_tex.insert(key, tex);
                    mgr.effective_uv.insert(key, uv);
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
            let h = make_image(
                &mut images,
                result.rgba_data,
                result.width,
                result.height,
                result.mip_levels,
            );
            if !mgr.gpu_textures.contains_key(&key) {
                mgr.gpu_tex_order.push_back(key);
            }
            mgr.gpu_textures.insert(key, h.clone());
            mgr.gpu_tex_size.insert(key, result.width);
            evict_gpu_cache(&mut mgr);
            h
        };
        mgr.reupload.remove(&key);
        // A sub-256 filler must never repaint a tile currently upsampling an
        // ancestor: its stretched texels are exactly the pale-patch artifact.
        // The handle stays cached (gpu_tex_size records the resolution so the
        // full-res re-fetch triggers), but the displayed pixels only change
        // when full-res imagery arrives.
        let filler = result.width < 256 && key.2 > BASE_LAYER_ZOOM;
        if !filler {
            mgr.effective_tex.insert(key, tex_handle.clone());
            mgr.effective_uv.insert(key, [0.0, 0.0, 1.0, 1.0]);

            if let Some(mat_handle) = mgr.gpu_materials.get(&key) {
                if let Some(mat) = materials.get_mut(mat_handle) {
                    mat.base_color_texture = Some(tex_handle);
                    mat.base_color = Color::WHITE;
                }
            }
        }
        if mgr.tile_entities.contains_key(&key) {
            mgr.textured_tiles.insert(key);
        }
        tex_uploads += 1;
    }

    // Sharpen pass: an upsampled tile whose own full-res texture AND real
    // mesh have both arrived swaps mesh + texture atomically in one frame
    // (single blur->sharp pop in place, then the fallback UV-remap mesh is
    // dropped for the exact-ellipsoid geometry).
    let sharpen: Vec<TileKey> = mgr
        .upsampled
        .iter()
        .filter(|k| {
            mgr.tile_entities.contains_key(*k)
                && mgr.gpu_meshes.contains_key(*k)
                && matches!(mgr.gpu_tex_size.get(*k), Some(&s) if s >= 256)
        })
        .copied()
        .collect();
    for key in sharpen {
        let ent = mgr.tile_entities[&key];
        if let (Some(mesh), Some(mat)) = (
            mgr.gpu_meshes.get(&key).cloned(),
            mgr.gpu_materials.get(&key).cloned(),
        ) {
            commands.entity(ent).insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            mgr.upsampled.remove(&key);
        }
    }

    // 5) Budgeted cleanup. Hidden tiles stay alive as warm zoom-out fallback
    //    (they draw nothing under strict partition rendering), so entities
    //    are only removed when the hard cap is exceeded — and then
    //    oldest-hidden first (LRU), never touching the render partition, the
    //    load set, protected ancestors of pending leaves, or the permanent
    //    base layer. Despawns are capped per frame so bulk eviction never
    //    produces a CPU spike mid-drag.
    let mut removed = 0;
    if mgr.tile_entities.len() > MAX_TILE_ENTITIES {
        let prot = protected_ancestors(&mgr);
        // Evict hidden tiles FINE-LEVEL-FIRST (they cover little area and
        // re-load cheaply from the GPU cache), oldest-hidden first within a
        // level. A pure hide-time LRU would eat the coarse tiles first —
        // exactly the warm pyramid a widening view re-partitions onto —
        // opening straight-edged holes down to the base sphere mid-zoom
        // (the "tiles shifted" artifact). Keeping coarse/mid levels alive
        // makes zoom-out land on live tiles instead.
        while mgr.tile_entities.len() > MAX_TILE_ENTITIES
            && removed < MAX_DESPAWNS_PER_FRAME
        {
            let len = mgr.hide_order.len();
            let Some(key) = mgr
                .hide_order
                .iter()
                .enumerate()
                .filter(|(_, k)| {
                    mgr.tile_entities.contains_key(*k)
                        && !prot.contains(*k)
                        && !mgr.visible_set.contains(*k)
                        && !mgr.load_set.contains(*k)
                        && k.2 > BASE_LAYER_ZOOM
                })
                .max_by_key(|(idx, k)| (k.2, len - idx))
                .map(|(_, k)| *k)
            else {
                break;
            };
            mgr.despawn_tile(&key, &mut commands);
            removed += 1;
        }

        // Still over the hard cap: evict the oldest surplus entities even if
        // visible (draw-call pressure beats coverage; GPU handles stay
        // cached so they re-spawn instantly if the view returns). Tiles
        // with a live ancestor are evicted first: their region stays
        // covered by the parent, so no blue hole opens. NEVER evict tiles
        // the render partition or the load set still needs: with strict
        // partition rendering the parent is hidden, so evicting a
        // partition leaf would open a hole down to the base sphere.
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
                    .filter(|k| {
                        k.2 > BASE_LAYER_ZOOM
                            && !mgr.visible_set.contains(k)
                            && !mgr.load_set.contains(k)
                            && has_live_ancestor(&mgr, k)
                    })
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
            .chain(mgr.load_set.iter())
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
                        && mgr
                            .retry_after
                            .get(k)
                            .map_or(true, |t| std::time::Instant::now() >= *t)
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
    // by the view moving on; queued / visible / loaded / spawned tiles stay
    // wanted.
    {
        let mut w = tex_rx.wanted.lock().unwrap();
        w.clear();
        w.extend(mgr.queued.iter().copied());
        w.extend(mgr.visible_set.iter().copied());
        w.extend(mgr.load_set.iter().copied());
        w.extend(mgr.tile_entities.keys().copied());
    }
}

/// A partition leaf the display can safely hand over to: a live entity
/// showing real pixels (own full-res texture, or an intentional solid /
/// no-data inheritance) — never a stretched-ancestor upsample whose soft
/// texels would read as a slid/shifted block against the retained children.
fn replacement_ready(mgr: &TileManager, key: &TileKey) -> bool {
    if !mgr.tile_entities.contains_key(key) {
        return false;
    }
    if mgr.solid_tiles.contains(key) || mgr.no_data.contains(key) {
        return true;
    }
    if mgr.upsampled.contains(key) {
        return false;
    }
    matches!(mgr.gpu_tex_size.get(key), Some(&s) if s >= 256)
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

/// CesiumJS-style KICK-aware quadtree partition (REPLACE refinement,
/// `QuadtreePrimitive.visitTile` selection): subdivide every visible tile
/// whose projected screen footprint exceeds [`MAX_TILE_SCREEN_PX`] (~1.1x
/// native 256-px texel size, so a leaf never stretches enough to read as a
/// soft/bright block next to its subdivided neighbors — the color-block
/// seam artifact), culling tiles
/// beyond the horizon cap — but only when ALL four children are already
/// spawned. If any child is missing, the parent stays in the render
/// partition (all-or-nothing, the CesiumJS KICK rule) and the children
/// keep loading in the background. Returns (render partition, background
/// load set); only the render partition is drawn (`sync_visibility`), so
/// parents and children never overlap and never z-fight — which is what
/// lets tiles sit at their EXACT ellipsoid positions with no radial tuck.
fn compute_visible_tiles(
    lat_rad: f64,
    lon_rad: f64,
    distance: f64,
    focal_px: f64,
    mgr: &TileManager,
) -> (Vec<(TileKey, f32)>, Vec<(TileKey, f32)>) {
    let d = distance.max(1.001);
    let cx = lat_rad.cos() * lon_rad.cos();
    let cy = lat_rad.cos() * lon_rad.sin();
    let cz = lat_rad.sin();
    let cap = (1.0 / d).acos();

    let mut render = Vec::new();
    let mut load = Vec::new();
    let n0 = 1u32 << MIN_ZOOM;
    for y in 0..n0 {
        for x in 0..n0 {
            visit_tile(
                x, y, MIN_ZOOM, cx, cy, cz, d, cap, focal_px, mgr, &mut render, &mut load,
            );
        }
    }
    (render, load)
}

/// Traversal outcome of a quadtree subtree — the Rust mirror of CesiumJS
/// `TraversalDetails.allAreRenderable`. `Ready` means every SELECTED tile
/// (i.e. not horizon-culled) in the subtree has a live entity and was added
/// to the render list; `NotReady` means at least one selected tile is still
/// missing its entity; `Culled` means the whole subtree is beyond the
/// horizon cap. Culled subtrees are invisible to the parent's
/// all-or-nothing check (CesiumJS never visits invisible children), so a
/// child crossing the cull boundary can never bounce the partition between
/// parent and children.
enum Visit {
    Ready,
    NotReady,
    Culled,
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
    mgr: &TileManager,
    render: &mut Vec<(TileKey, f32)>,
    load: &mut Vec<(TileKey, f32)>,
) -> Visit {
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
        return Visit::Culled;
    }

    // Camera→tile-center chord distance in meters (globe radius = 1 unit).
    let ex = tx - cx * d;
    let ey = ty - cy * d;
    let ez = tz - cz * d;
    let dist_m = (ex * ex + ey * ey + ez * ez).sqrt() * EARTH_RADIUS_M;

    // Projected screen footprint of the tile's Web Mercator width.
    let w_m = 2.0 * std::f64::consts::PI * EARTH_RADIUS_M / n as f64;
    let screen_px = w_m / dist_m * focal_px;

    let has_ent = mgr.tile_entities.contains_key(&(x, y, z));

    if screen_px <= MAX_TILE_SCREEN_PX || z >= MAX_ZOOM {
        // Selected leaf. Like CesiumJS (which adds even not-yet-renderable
        // tiles to the render list so the tree can settle on the deepest
        // available coverage), the leaf is always added; the outcome only
        // reports whether it is drawable so the parent can KICK back to its
        // own level instead of drawing a hole. `sync_visibility` simply
        // skips unspawned tiles until they appear.
        render.push(((x, y, z), screen_px as f32));
        // A tile counts as ready the moment it has a live entity: a child
        // still upsampling its ancestor's texture is visually IDENTICAL to
        // the parent's pixels, so swapping the partition onto it is invisible
        // — no KICK hold-back needed (that only delayed the swap and popped
        // the whole screen between coarse levels).
        if has_ent {
            Visit::Ready
        } else {
            Visit::NotReady
        }
    } else {
        // SSE not good enough: refine into the four children, then apply the
        // CesiumJS all-or-nothing rule on the traversal OUTCOMES — not on
        // how many render entries the children produced (a child that
        // successfully refined into its own grandchildren added four
        // entries and still counts as ready).
        let start = render.len();
        let (x2, y2, z1) = (x * 2, y * 2, z + 1);
        let children = [
            (x2, y2, z1),
            (x2 + 1, y2, z1),
            (x2, y2 + 1, z1),
            (x2 + 1, y2 + 1, z1),
        ];
        let outcomes = [
            visit_tile(
                x2, y2, z1, cx, cy, cz, d, cap, focal_px, mgr, render, load,
            ),
            visit_tile(
                x2 + 1, y2, z1, cx, cy, cz, d, cap, focal_px, mgr, render, load,
            ),
            visit_tile(
                x2, y2 + 1, z1, cx, cy, cz, d, cap, focal_px, mgr, render, load,
            ),
            visit_tile(
                x2 + 1, y2 + 1, z1, cx, cy, cz, d, cap, focal_px, mgr, render, load,
            ),
        ];
        let any_selected = outcomes
            .iter()
            .any(|o| matches!(o, Visit::Ready | Visit::NotReady));
        if !any_selected {
            // Every child was horizon-culled. CesiumJS renders nothing here
            // (the region sits at the limb); we conservatively render this
            // tile so a cull-boundary gap can never open down to the base
            // sphere.
            render.push(((x, y, z), screen_px as f32));
            return if mgr.tile_entities.contains_key(&(x, y, z)) {
                Visit::Ready
            } else {
                Visit::NotReady
            };
        }
        let all_ready = outcomes
            .iter()
            .all(|o| matches!(o, Visit::Ready | Visit::Culled));
        if !all_ready {
            // CesiumJS KICK rule: drop everything the children selected and
            // render this tile instead — but keep loading the missing
            // children so the partition swaps down the moment the last one
            // spawns ("continue to load them though!"). Culled children are
            // skipped: CesiumJS never visits invisible children, so they
            // neither block the swap nor consume download bandwidth.
            render.truncate(start);
            for (c, o) in children.iter().zip(outcomes.iter()) {
                if !matches!(o, Visit::Culled)
                    && (!mgr.tile_entities.contains_key(c)
                        || mgr
                            .gpu_tex_size
                            .get(c)
                            .map_or(false, |&s| s < 256))
                {
                    // Missing child, or child stuck on a downscaled horizon
                    // filler: keep it in the load set so the full-res
                    // (re)download is scheduled and the swap can happen.
                    load.push((*c, (screen_px * 0.5) as f32));
                }
            }
            render.push(((x, y, z), screen_px as f32));
            // Mirror of CesiumJS `allAreRenderable = tile.renderable` after
            // a KICK: as far as the parent is concerned this subtree now
            // reduces to this tile.
            if has_ent {
                Visit::Ready
            } else {
                Visit::NotReady
            }
        } else {
            Visit::Ready
        }
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

/// Strict partition rendering — the CesiumJS "only tiles on the per-frame
/// render list are drawn" rule: every spawned tile outside the render
/// partition is hidden the same frame the partition changes, so a parent
/// and its children NEVER draw together and NEVER z-fight. Hiding instead
/// of despawning keeps the entity + GPU handles hot, so re-entry when the
/// view returns is instant.
fn sync_visibility(
    mgr: Res<TileManager>,
    mut tiles: Query<(&GlobeTile, &mut Visibility)>,
) {
    for (tile, mut vis) in &mut tiles {
        let in_partition = mgr
            .visible_set
            .contains(&(tile.x, tile.y, tile.z));
        let target = if in_partition {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != target {
            *vis = target;
        }
    }
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

/// Create a GPU texture from worker-prepared RGBA data (base level + mip
/// chain already built off the frame thread) with the CesiumJS imagery
/// sampler (trilinear mipmap + anisotropic filtering), so minified horizon
/// tiles don't shimmer and oblique tiles stay crisp. Building the chain on
/// the download workers keeps fast-zoom frames free of mip CPU spikes.
fn make_image(
    images: &mut Assets<Image>,
    data: Vec<u8>,
    width: u32,
    height: u32,
    levels: u32,
) -> Handle<Image> {
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

// ── Base-sphere whole-globe composite ──────────────────────────────

/// Marker for the non-LOD base sphere so the composite system can find its
/// material and drape the baked whole-globe texture over it.
#[derive(Component)]
pub struct BaseSphereMarker;

const COMPOSITE_TILE: u32 = 128; // per-tile block size inside the composite
const COMPOSITE_SIZE: u32 = 8 * COMPOSITE_TILE; // z=3: 8x8 tiles = 1024 px

#[derive(Resource, Default)]
struct BaseSphereComposite {
    rx: Option<Mutex<mpsc::Receiver<(Vec<u8>, u32)>>>,
    done: bool,
}

/// Once every base-layer (z=3) tile has either a texture or a no-data
/// verdict, bake them into one 1024x1024 Mercator composite and drape it
/// over the base sphere. Any transient coverage hole or the horizon limb
/// then shows a blurry earth instead of the flat blue sphere color.
fn base_sphere_composite_system(
    mut state: ResMut<BaseSphereComposite>,
    mgr: Res<TileManager>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sphere: Query<&MeshMaterial3d<StandardMaterial>, With<BaseSphereMarker>>,
) {
    if state.done {
        return;
    }
    if let Some(rx) = &state.rx {
        let Ok((chain, levels)) = rx.lock().unwrap().try_recv() else {
            return;
        };
        let handle =
            make_clamped_image(&mut images, chain, COMPOSITE_SIZE, COMPOSITE_SIZE, levels);
        if let Ok(mat) = sphere.get_single() {
            if let Some(m) = materials.get_mut(&mat.0) {
                m.base_color = Color::WHITE;
                m.base_color_texture = Some(handle);
            }
        }
        state.rx = None;
        state.done = true;
        return;
    }

    let keys: Vec<TileKey> = (0..8u32)
        .flat_map(|x| (0..8u32).map(move |y| (x, y, BASE_LAYER_ZOOM)))
        .collect();
    if !keys
        .iter()
        .all(|k| mgr.gpu_textures.contains_key(k) || mgr.no_data.contains(k))
    {
        return;
    }

    // Collect 128-px blocks (box-downsampled full-res tiles); tiles with no
    // imagery become deep-ocean blue.
    let mut blocks: Vec<(u32, u32, Vec<u8>)> = Vec::with_capacity(keys.len());
    for k in keys {
        let block = mgr
            .gpu_textures
            .get(&k)
            .and_then(|h| images.get(h))
            .map(|img| {
                let w = img.texture_descriptor.size.width;
                let h = img.texture_descriptor.size.height;
                box_downsample(&img.data[..(w * h * 4) as usize], w, COMPOSITE_TILE)
            })
            .unwrap_or_else(ocean_block);
        blocks.push((k.0, k.1, block));
    }

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut comp = vec![0u8; (COMPOSITE_SIZE * COMPOSITE_SIZE * 4) as usize];
        for (bx, by, blk) in blocks {
            let x0 = bx * COMPOSITE_TILE;
            let y0 = by * COMPOSITE_TILE;
            for row in 0..COMPOSITE_TILE {
                let src = (row * COMPOSITE_TILE * 4) as usize;
                let dst = ((y0 + row) * COMPOSITE_SIZE * 4 + x0 * 4) as usize;
                comp[dst..dst + (COMPOSITE_TILE * 4) as usize]
                    .copy_from_slice(&blk[src..src + (COMPOSITE_TILE * 4) as usize]);
            }
        }
        let (chain, levels) = build_mip_chain(comp, COMPOSITE_SIZE, COMPOSITE_SIZE);
        let _ = tx.send((chain, levels));
    });
    state.rx = Some(Mutex::new(rx));
}

/// Box-downsample an RGBA tile of width `w` (power-of-two multiple of the
/// target) to `target` x `target`.
fn box_downsample(src: &[u8], w: u32, target: u32) -> Vec<u8> {
    let factor = (w / target).max(1);
    let mut out = vec![0u8; (target * target * 4) as usize];
    for y in 0..target {
        for x in 0..target {
            let mut acc = [0u32; 4];
            for dy in 0..factor {
                for dx in 0..factor {
                    let i = (((y * factor + dy) * w + (x * factor + dx)) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += src[i + c] as u32;
                    }
                }
            }
            let n = factor * factor;
            let o = ((y * target + x) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (acc[c] / n) as u8;
            }
        }
    }
    out
}

/// Raw (unlit) deep-ocean blue close to Bing imagery water pixels, for
/// composite blocks whose tile has no imagery.
fn ocean_block() -> Vec<u8> {
    let mut v = Vec::with_capacity((COMPOSITE_TILE * COMPOSITE_TILE * 4) as usize);
    for _ in 0..COMPOSITE_TILE * COMPOSITE_TILE {
        v.extend_from_slice(&[12, 28, 44, 255]);
    }
    v
}

/// Like `make_image` but clamped at the edges: the composite is a single
/// whole-globe Mercator image, not a repeating tile.
fn make_clamped_image(
    images: &mut Assets<Image>,
    data: Vec<u8>,
    width: u32,
    height: u32,
    levels: u32,
) -> Handle<Image> {
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
        address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
        address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
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
/// Feed download jobs to the persistent worker pool. Non-blocking: jobs queue
/// up and workers skip stale ones (not in `wanted`) at dequeue time.
fn start_downloads(tex_rx: &TextureReceiver, tiles: &[(TileKey, bool)]) {
    for &(key, downscale) in tiles {
        let _ = tex_rx.job_tx.send((key, downscale));
    }
}

/// Persistent download-pool worker: owns one ureq agent for its whole life
/// (its connection pool keeps tile-server connections warm across fetches,
/// so full-res re-uploads reuse TLS sessions instead of paying a fresh
/// handshake per tile) and pulls jobs until the feed closes.
fn download_worker(
    job_rx: Arc<Mutex<mpsc::Receiver<(TileKey, bool)>>>,
    tx: mpsc::Sender<TileDownloadResult>,
    wanted: Arc<Mutex<HashSet<TileKey>>>,
) {
    let agent = ureq::AgentBuilder::new()
        .user_agent("Mozilla/5.0 CesiumRust/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .build();

    loop {
        let job = job_rx.lock().unwrap().recv();
        let Ok(((px, py, pz), downscale)) = job else {
            return;
        };
        // Fast pans make whole batches stale within a few frames; skip
        // fetches nobody will look at so the server connection budget goes
        // to the current view instead.
        if !wanted.lock().unwrap().contains(&(px, py, pz)) {
            let _ = tx.send(TileDownloadResult {
                x: px,
                y: py,
                z: pz,
                rgba_data: Vec::new(),
                width: 0,
                height: 0,
                mip_levels: 0,
                placeholder: false,
                aborted: true,
                failed: false,
            });
            continue;
        }
        let qk = tile_to_quadkey(px, py, pz);
        let sub = (px + py) % 8;
        let url = format!(
            "https://ecn.t{}.tiles.virtualearth.net/tiles/a{}.jpeg?g=14393",
            sub, qk
        );
        // Retry with backoff: tile servers throttle bursty clients
        // (403/429/timeouts); a permanently failed tile would otherwise
        // never appear.
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
                Err(_) => None,
            };
            if let Some(img) = fetched {
                let rgba = img.to_rgba8();
                // Bing serves smooth gradient placeholder JPEGs for tiles
                // without imagery; pasting one would stamp an opaque blue
                // hole over the good parent coverage, so such tiles are
                // treated as no-data and inherit ancestor coverage on the
                // main thread instead.
                let (is_ph, ph_avg, ph_maxd) = is_placeholder_tile(&rgba);
                if is_ph {
                    eprintln!(
                        "[nodata] z{pz} x{px} y{py} verdict=placeholder avg={ph_avg:.2} maxd={ph_maxd}"
                    );
                    let _ = tx.send(TileDownloadResult {
                        x: px,
                        y: py,
                        z: pz,
                        rgba_data: Vec::new(),
                        width: 0,
                        height: 0,
                        mip_levels: 0,
                        placeholder: true,
                        aborted: false,
                        failed: false,
                    });
                    delivered = true;
                    break;
                }
                let (rgba, w, h) = if downscale && rgba.width() > 128 {
                    let small = image::DynamicImage::ImageRgba8(rgba).resize(
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
                // Build the mip chain on this worker thread: the frame
                // thread then only pays the GPU upload, keeping fast-zoom
                // frame pacing smooth (main-thread mip builds spiked frames
                // mid-zoom, reading as image wobble).
                let (chain, levels) = build_mip_chain(rgba.into_raw(), w, h);
                let _ = tx.send(TileDownloadResult {
                    x: px,
                    y: py,
                    z: pz,
                    rgba_data: chain,
                    width: w,
                    height: h,
                    mip_levels: levels,
                    placeholder: false,
                    aborted: false,
                    failed: false,
                });
                delivered = true;
                break;
            }
        }
        if !delivered {
            // All retries failed (transient throttle/timeout): NOT no-data.
            // The main thread cools the tile down and re-issues the fetch;
            // stamping permanent ancestor coverage here is what turned
            // throttled tiles into rectangular color blocks.
            eprintln!("[nodata] z{pz} x{px} y{py} verdict=retries-exhausted");
            let _ = tx.send(TileDownloadResult {
                x: px,
                y: py,
                z: pz,
                rgba_data: Vec::new(),
                width: 0,
                height: 0,
                mip_levels: 0,
                placeholder: false,
                aborted: false,
                failed: true,
            });
        }
    }
}
