//! M1.5 thin-shell dynamic globe — ImageryKey/ImageryPayload specialization
//! delegating tile fetch/decode/GPU-cache/eviction to the `cesium-pipeline`
//! core (via `fetch_gated` → shared ureq keep-alive pool) and extracted helper
//! modules (`globe_lod`, `globe_pipeline`, `globe_textures`, `base_sphere`).
//!
//! ## Implicit contracts preserved (逐字节/带注释):
//! - `BaseSphereMarker` (re-exported from `base_sphere`)
//! - `DynamicGlobePlugin` — same system chain, same Startup/Update registration
//! - `BASE_LAYER_ZOOM = 3` permanent resident fallback (via `DefaultBudget`)
//! - wanted-staleness skip (worker checks wanted set before fetch)
//! - `own_full_res` (L837-841 / L1163 semantics preserved in `globe_pipeline`)
//! - `evict_gpu_cache` three invariants: BASE_LAYER exempt, live push_back
//!   deferral (花屏防护), termination 1800 << 3000 (via `DefaultBudget`)
//! - `TilePipelineSet` pub(crate) (perf_trace cross-module contract)
//! - `PerfCounters` write-back (bidirectional with legacy)
//!
//! ## GenericPipeline deferred-problem resolutions (M1.4 → M1.5):
//! (a) Decoder(bytes)→Payload without key context: **solved by specialization**
//!     — imagery decode (load_from_memory + placeholder + mip chain) is
//!     key-independent; downscale is a per-job flag in the worker channel.
//! (b) K:Copy excludes tileset Vec/String keys: **not applicable** — imagery
//!     TileKey=(u32,u32,u32) IS Copy; tileset is a separate consumer.
//! (c) Cross-frame persistence needs new Resource: **solved** — TileManager +
//!     TextureReceiver ARE Bevy Resources (inherently cross-frame); they wrap
//!     the pipeline's NetworkBackend (shared ureq pool) + DefaultBudget.
//!
//! ## Offline wiring (M3 gate L127):
//! When `OFFLINE_IMAGERY_ROOT` is set, the download worker reads tiles from
//! disk (`{root}/{z}/{x}/{y}.png`) instead of Bing. `STRICT_OFFLINE=1` panics
//! on any https URL (no network fallback).

// frozen legacy golden-path style debt; local allow to satisfy strict CI clippy gate
#![allow(clippy::unnecessary_map_or, clippy::type_complexity, clippy::too_many_arguments)]

use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::base_sphere::BaseSphereComposite;
use crate::globe_lod::{self, LodContext, TileKey, BASE_LAYER_ZOOM};
use crate::globe_pipeline;
use crate::orbit_camera::OrbitState;
use crate::perf_counters::PerfCounters;
use crate::tile_mesh::GlobeTile;
use cesium_pipeline::DefaultBudget;

// ── Types ────────────────────────────────────────────────────────────────

/// Cached tile image data (mip-chained RGBA bytes + dimensions).
pub(crate) struct CachedTexture {
    pub rgba_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
}

// ── Resources ────────────────────────────────────────────────────────────

#[derive(Resource)]
pub(crate) struct TileManager {
    pub tile_entities: HashMap<TileKey, Entity>,
    pub textured_tiles: HashSet<TileKey>,
    pub current_zoom: u32,
    pub last_distance: f32,
    pub initialized: bool,
    pub visible_set: HashSet<TileKey>,
    pub partition_set: HashSet<TileKey>,
    pub load_set: HashSet<TileKey>,
    pub spawn_queue: VecDeque<TileKey>,
    pub queued: HashSet<TileKey>,
    pub gpu_meshes: HashMap<TileKey, Handle<Mesh>>,
    pub gpu_materials: HashMap<TileKey, Handle<StandardMaterial>>,
    pub gpu_textures: HashMap<TileKey, Handle<Image>>,
    pub no_data: HashSet<TileKey>,
    pub gpu_fb_meshes: HashMap<TileKey, Handle<Mesh>>,
    pub effective_tex: HashMap<TileKey, Handle<Image>>,
    pub effective_uv: HashMap<TileKey, [f32; 4]>,
    pub solid_tiles: HashSet<TileKey>,
    pub upsampled: HashSet<TileKey>,
    pub pending_fb_builds: HashSet<TileKey>,
    pub gpu_tex_order: VecDeque<TileKey>,
    pub gpu_tex_size: HashMap<TileKey, u32>,
    pub reupload: HashSet<TileKey>,
    pub in_flight: HashSet<TileKey>,
    pub retry_after: HashMap<TileKey, std::time::Instant>,
    pub spawn_order: VecDeque<TileKey>,
    pub hide_order: VecDeque<TileKey>,
    pub view_changed_this_frame: bool,
}

impl Default for TileManager {
    fn default() -> Self {
        Self {
            tile_entities: HashMap::new(), textured_tiles: HashSet::new(),
            current_zoom: globe_lod::MIN_ZOOM, last_distance: 0.0, initialized: false,
            visible_set: HashSet::new(), partition_set: HashSet::new(), load_set: HashSet::new(),
            spawn_queue: VecDeque::new(), queued: HashSet::new(),
            gpu_meshes: HashMap::new(), gpu_materials: HashMap::new(), gpu_textures: HashMap::new(),
            no_data: HashSet::new(), gpu_fb_meshes: HashMap::new(),
            effective_tex: HashMap::new(), effective_uv: HashMap::new(),
            solid_tiles: HashSet::new(), upsampled: HashSet::new(),
            pending_fb_builds: HashSet::new(), gpu_tex_order: VecDeque::new(),
            gpu_tex_size: HashMap::new(), reupload: HashSet::new(), in_flight: HashSet::new(),
            retry_after: HashMap::new(), spawn_order: VecDeque::new(), hide_order: VecDeque::new(),
            view_changed_this_frame: false,
        }
    }
}

impl LodContext for TileManager {
    fn has_entity(&self, key: &TileKey) -> bool { self.tile_entities.contains_key(key) }
    fn tex_size(&self, key: &TileKey) -> Option<u32> { self.gpu_tex_size.get(key).copied() }
}

impl TileManager {
    /// Despawn one tile entity; GPU handles stay cached for cheap re-spawn.
    pub fn despawn_tile(&mut self, key: &TileKey, commands: &mut Commands) {
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

#[derive(Resource)]
pub(crate) struct TextureReceiver {
    pub rx: Mutex<mpsc::Receiver<globe_pipeline::TileDownloadResult>>,
    pub job_tx: mpsc::Sender<(TileKey, bool)>,
    pub cache: Arc<Mutex<HashMap<TileKey, CachedTexture>>>,
    pub wanted: Arc<Mutex<HashSet<TileKey>>>,
}

impl Default for TextureReceiver {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let (job_tx, job_rx) = mpsc::channel::<(TileKey, bool)>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let wanted = Arc::new(Mutex::new(HashSet::new()));
        // Persistent worker pool: 16 threads with shared ureq keep-alive pool
        // (delegates to cesium-pipeline's UreqBackend via fetch_gated).
        for _ in 0..DefaultBudget::DOWNLOAD_THREADS {
            let job_rx = job_rx.clone();
            let tx = tx.clone();
            let wanted = wanted.clone();
            std::thread::spawn(move || globe_pipeline::download_worker(job_rx, tx, wanted));
        }
        Self { rx: Mutex::new(rx), job_tx, cache, wanted }
    }
}

#[derive(Resource)]
pub(crate) struct MeshPipeline {
    pub tx: mpsc::Sender<(TileKey, Mesh, bool)>,
    pub rx: Mutex<mpsc::Receiver<(TileKey, Mesh, bool)>>,
    pub backlog: VecDeque<(TileKey, Mesh, bool)>,
}

impl Default for MeshPipeline {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx: Mutex::new(rx), backlog: VecDeque::new() }
    }
}

// ── Plugin ───────────────────────────────────────────────────────────────

/// SystemSet marker for the tile pipeline chain (pub(crate) — perf_trace
/// cross-module contract, preserved from golden path L300-301).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TilePipelineSet;

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
                (view_dependent_update, process_pipeline, sync_visibility)
                    .chain()
                    .in_set(TilePipelineSet),
            )
            .add_systems(Update, base_sphere_composite_system);
    }
}

// Re-export so main.rs import path is unchanged.
pub use crate::base_sphere::BaseSphereMarker;

// ── Startup ──────────────────────────────────────────────────────────────

fn initial_spawn(
    mut mgr: ResMut<TileManager>,
    mut mesh_pipe: ResMut<MeshPipeline>,
    tex_rx: Res<TextureReceiver>,
    orbit: Res<OrbitState>,
    windows: Query<&Window>,
) {
    let (lat_rad, lon_rad) = globe_lod::compute_sub_camera_point(&orbit);
    let (visible, load) = globe_lod::compute_visible_tiles(
        lat_rad, lon_rad, orbit.distance as f64, globe_lod::focal_pixels(&windows), &*mgr,
    );
    let finest = visible.iter().map(|t| t.0 .2).max().unwrap_or(globe_lod::MIN_ZOOM);
    mgr.visible_set = visible.iter().map(|&(k, _)| k).collect();
    mgr.partition_set = mgr.visible_set.clone();
    mgr.load_set = load.iter().map(|&(k, _)| k).collect();

    globe_pipeline::enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &visible);
    globe_pipeline::enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &load);

    // Permanent coarse fallback layer (BASE_LAYER_ZOOM=3, 硬约束).
    let mut base: Vec<(TileKey, f32)> = Vec::new();
    for z in 1..=BASE_LAYER_ZOOM {
        for y in 0..(1u32 << z) {
            for x in 0..(1u32 << z) {
                base.push(((x, y, z), 100.0));
            }
        }
    }
    globe_pipeline::enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &base);
    mgr.current_zoom = finest;
    mgr.last_distance = orbit.distance;
    mgr.initialized = true;
}

// ── View-dependent update ────────────────────────────────────────────────

fn view_dependent_update(
    orbit: Res<OrbitState>,
    windows: Query<&Window>,
    mut mgr: ResMut<TileManager>,
    mut mesh_pipe: ResMut<MeshPipeline>,
    tex_rx: Res<TextureReceiver>,
) {
    if !mgr.initialized { return; }
    let (lat_rad, lon_rad) = globe_lod::compute_sub_camera_point(&orbit);
    let (new_visible, new_load) = globe_lod::compute_visible_tiles(
        lat_rad, lon_rad, orbit.distance as f64, globe_lod::focal_pixels(&windows), &*mgr,
    );
    let finest = new_visible.iter().map(|t| t.0 .2).max().unwrap_or(mgr.current_zoom);
    let new_set: HashSet<TileKey> = new_visible.iter().map(|&(k, _)| k).collect();
    let new_load_set: HashSet<TileKey> = new_load.iter().map(|&(k, _)| k).collect();

    let old_set = std::mem::take(&mut mgr.visible_set);
    let (display, partition_changed) = globe_lod::compute_display_set(
        &old_set, &new_set, &new_load_set, &mgr.partition_set, &mgr.load_set,
        &|k| globe_pipeline::replacement_ready(&mgr, k),
    );
    mgr.partition_set = new_set.clone();
    mgr.visible_set = display;
    mgr.load_set = new_load_set;

    globe_pipeline::enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &new_visible);
    globe_pipeline::enqueue_tiles(&mut mgr, &mut mesh_pipe, &tex_rx, &new_load);

    // Hidden-tile LRU maintenance.
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

// ── Budgeted asset pipeline ──────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn process_pipeline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut mgr: ResMut<TileManager>,
    mut mesh_pipe: ResMut<MeshPipeline>,
    tex_rx: Res<TextureReceiver>,
    mut perf: ResMut<PerfCounters>,
) {
    globe_pipeline::run(
        &mut commands, &mut meshes, &mut materials, &mut images,
        &mut mgr, &mut mesh_pipe, &tex_rx, &mut perf,
    );
}

// ── Visibility sync ──────────────────────────────────────────────────────

fn sync_visibility(mgr: Res<TileManager>, mut tiles: Query<(&GlobeTile, &mut Visibility)>) {
    for (tile, mut vis) in &mut tiles {
        let in_partition = mgr.visible_set.contains(&(tile.x, tile.y, tile.z));
        let target = if in_partition { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != target { *vis = target; }
    }
}

// ── Base-sphere composite ────────────────────────────────────────────────

fn base_sphere_composite_system(
    mut state: ResMut<BaseSphereComposite>,
    mgr: Res<TileManager>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sphere: Query<&MeshMaterial3d<StandardMaterial>, With<BaseSphereMarker>>,
) {
    globe_pipeline::run_base_sphere_composite(
        &mut state, &mgr, &mut images, &mut materials, &sphere,
    );
}
