use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_decoders::decode_quantized_mesh;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use cesium_terrain::{QuantizedMeshTerrainData, TerrainMesh};

use crate::components::{CesiumTerrainTile, TileContentState};
use crate::pipeline::{self, budget};
use crate::resources::{GlobeConfig, TileLoadStats};

use super::lod_system::TerrainSelection;

/// Tile coordinate key `(x, y, level)`.
pub type TileKey = (u32, u32, u32);

#[derive(Resource, Default)]
pub struct TerrainLoadState {
    pub loaded_count: u32,
    pub failed_count: u32,
}

/// In-flight asynchronous terrain tile loads.
///
/// Keyed by `TileKey`; each entry owns the spawned [`IoTaskPool`] task plus the
/// placeholder entity that receives the decoded mesh once the task resolves.
/// Storing the task here (rather than draining a shared queue) is what
/// structurally removes the load/render drain race: the loader is the *only*
/// consumer of `TerrainSelection::tiles_to_load`.
#[derive(Resource, Default)]
pub struct TerrainPendingLoads {
    pub pending: HashMap<TileKey, PendingLoad>,
    /// Resolved-but-not-yet-uploaded meshes, drained FIFO up to the per-frame
    /// terrain mesh budget (gate ON). Always fully drained in-frame when gate
    /// OFF (budget = `UNBOUNDED`), so the legacy behaviour is unchanged. Entries
    /// are moved here the moment their task resolves (never re-polled), which is
    /// what lets the budget defer GPU uploads without touching a finished task.
    pub ready_backlog: VecDeque<(TileKey, Entity, Result<TerrainMesh, String>)>,
}

pub struct PendingLoad {
    /// Entity spawned in `Loading` state, awaiting the decoded mesh.
    pub entity: Entity,
    /// Background download+decode task; yields the CPU-side `TerrainMesh`.
    pub task: Task<Result<TerrainMesh, String>>,
}

fn build_terrain_url(config: &GlobeConfig, x: u32, y: u32, level: u32) -> Option<String> {
    let base = config.terrain_provider_url.as_ref()?;
    Some(
        base.replace("{z}", &level.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string()),
    )
}

fn tile_rectangle(x: u32, y: u32, level: u32) -> Rectangle {
    let n = 2u32.pow(level.max(1)) as f64;
    let west = (x as f64 / n) * std::f64::consts::TAU - std::f64::consts::PI;
    let east = ((x as f64 + 1.0) / n) * std::f64::consts::TAU - std::f64::consts::PI;
    let south = (y as f64 / n) * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;
    let north = ((y as f64 + 1.0) / n) * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;
    Rectangle::from_radians(west, south, east, north)
}

/// Level-zero maximum *geometric* error for terrain, in metres.
///
/// This is the coarsest terrain height-error scale (metres) — NOT the tiling
/// scheme's horizontal half-circumference (`semiMajorAxis * PI / 2 ≈ 1e7 m`).
/// Deriving the skirt from that horizontal figure made the level-0 skirt ~5e7 m
/// (larger than Earth's radius ~6.4e6 m), so `add_skirts`' `carto.height -
/// skirt` went hugely negative and mirrored skirt vertices through the geocenter
/// into degenerate spikes that pierce the globe.
const LEVEL_ZERO_MAXIMUM_GEOMETRIC_ERROR: f64 = 100.0;

/// Hard upper bound (metres) on any tile's skirt height. A skirt only needs to
/// cover the LOD crack between neighbouring tiles; this clamp guarantees it can
/// never grow large enough to fold geometry through the geocenter, even if the
/// level-0 estimate above is later revised upward.
const MAX_SKIRT_HEIGHT: f64 = 1000.0;

/// Vertical skirt height (metres) for a tile at `level`.
///
/// Mirrors CesiumJS `getLevelMaximumGeometricError(level) * 5.0`
/// (`= levelZeroMaximumGeometricError / 2^level * 5.0`), but with a metre-scale
/// level-0 error and a sane upper clamp so skirts stay physically plausible.
/// TODO 对齐 CesiumJS 垂裙公式：接入真实 tiling scheme 的 level-zero error 后精确对齐。
fn skirt_height_for_level(level: u32) -> f64 {
    let denom = (1u64 << level.min(32)) as f64;
    (LEVEL_ZERO_MAXIMUM_GEOMETRIC_ERROR / denom * 5.0).min(MAX_SKIRT_HEIGHT)
}

/// Downloads and binary-decodes a quantized-mesh terrain tile.
///
/// Blocking: intended to run on an [`IoTaskPool`] worker thread, never on the
/// frame thread. The fetch is tokio-free — it routes through the cesium-pipeline
/// core's ureq blocking backend ([`pipeline::fetch::fetch_gated`]), selecting the
/// shared keep-alive pool (gate ON) or a fresh per-call client (gate OFF).
fn fetch_and_decode_terrain(
    url: &str,
    skirt_height: f64,
    use_pipeline: bool,
) -> Result<QuantizedMeshTerrainData, String> {
    let data = pipeline::fetch::fetch_gated(url, use_pipeline)?;

    // Binary quantized-mesh decode (was incorrectly `serde_json::from_slice`).
    decode_quantized_mesh(&data, skirt_height).map_err(|e| format!("decode: {}", e))
}

/// Full CPU-side worker: fetch + decode + build the render mesh with skirts.
///
/// Produces only CPU-side data (`TerrainMesh`); GPU upload happens later in
/// `terrain_render_system`. Runs on an [`IoTaskPool`] worker thread.
fn load_and_decode_terrain(
    url: &str,
    rect: &Rectangle,
    ellipsoid: &Ellipsoid,
    skirt_height: f64,
    use_pipeline: bool,
) -> Result<TerrainMesh, String> {
    let qm = fetch_and_decode_terrain(url, skirt_height, use_pipeline)?;
    Ok(qm.create_mesh_with_skirts(rect, ellipsoid, 1.0))
}

/// Polls in-flight tasks and transitions resolved tiles `Loading → Ready/Failed`.
///
/// Each task is polled exactly once (`poll_once`) so this never blocks the frame
/// thread; unresolved tasks stay in `pending` and are polled again next frame.
/// Resolved results are moved into a FIFO backlog and uploaded up to `budget`
/// meshes per frame — `UNBOUNDED` (gate OFF) drains everything in-frame exactly
/// like the pre-migration code, while gate ON bounds GPU work to the terrain
/// mesh weight ([`budget::terrain_mesh_budget`]).
fn poll_pending_loads(
    commands: &mut Commands,
    pending: &mut TerrainPendingLoads,
    load_state: &mut TerrainLoadState,
    stats: &mut TileLoadStats,
    budget: usize,
) {
    // 1. Poll every in-flight task once; move resolved results into the backlog.
    //    A finished task is drained here and never re-polled, so the budget can
    //    defer the GPU upload without touching a completed `Task`.
    let mut resolved: Vec<(TileKey, Entity, Result<TerrainMesh, String>)> = Vec::new();
    for (key, load) in pending.pending.iter_mut() {
        if let Some(result) = block_on(poll_once(&mut load.task)) {
            resolved.push((*key, load.entity, result));
        }
    }
    for (key, entity, result) in resolved {
        // Safe: `key` was just read from `pending.pending` and nothing removed it.
        pending.pending.remove(&key);
        pending.ready_backlog.push_back((key, entity, result));
    }

    // 2. Upload up to `budget` meshes this frame (FIFO).
    let mut uploaded = 0;
    while uploaded < budget {
        let Some((key, entity, result)) = pending.ready_backlog.pop_front() else {
            break;
        };
        uploaded += 1;
        stats.tiles_pending = stats.tiles_pending.saturating_sub(1);

        match result {
            Ok(mesh) => {
                // `try_insert`: the placeholder entity may have been unloaded
                // (despawned) while the task was in flight — a plain `insert`
                // would panic with Bevy B0003. Mirrors tileset content_loader.
                commands.entity(entity).try_insert(TerrainTileReady {
                    terrain_mesh: Some(mesh),
                    state: TileContentState::Ready,
                });
                load_state.loaded_count += 1;
                stats.tiles_loaded += 1;
            }
            Err(e) => {
                // Graceful degradation: warn + skip, never panic.
                warn!(
                    "Terrain tile ({},{},{}) failed to load/decode, skipping: {}",
                    key.0, key.1, key.2, e
                );
                commands.entity(entity).try_insert(TerrainTileReady {
                    terrain_mesh: None,
                    state: TileContentState::Failed,
                });
                load_state.failed_count += 1;
                stats.tiles_failed += 1;
            }
        }
    }
}

pub fn terrain_tile_load_system(
    mut commands: Commands,
    config: Option<Res<GlobeConfig>>,
    mut selection: ResMut<TerrainSelection>,
    mut pending: ResMut<TerrainPendingLoads>,
    mut load_state: ResMut<TerrainLoadState>,
    mut stats: ResMut<TileLoadStats>,
    terrain_query: Query<(Entity, &CesiumTerrainTile)>,
) {
    // Gate: read once per frame. ON routes fetches through the cesium-pipeline
    // core's shared keep-alive ureq pool and bounds uploads to the terrain mesh
    // weight; OFF keeps the legacy per-call fetch and drains all resolved tiles.
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let upload_budget = if use_pipeline {
        budget::terrain_mesh_budget()
    } else {
        budget::UNBOUNDED
    };

    // Retire tasks that resolved since the previous frame (Loading → Ready/Failed).
    poll_pending_loads(
        &mut commands,
        &mut pending,
        &mut load_state,
        &mut stats,
        upload_budget,
    );

    let config = match config {
        Some(c) => c,
        None => return,
    };

    // HashMap<TileKey, Entity> index replaces the previous per-tile O(n) linear
    // `find` over the query, which made the whole system O(n²).
    let mut index: HashMap<TileKey, Entity> = HashMap::new();
    for (entity, tile) in terrain_query.iter() {
        index.insert((tile.x, tile.y, tile.level), entity);
    }

    let ellipsoid = config.ellipsoid;
    let pool = IoTaskPool::get();

    // The loader is the sole consumer of `tiles_to_load`; the render system now
    // scans `state == Ready` instead of draining the same queue (race removed).
    for key in selection.tiles_to_load.drain(..) {
        let (x, y, level) = key;

        // Already loaded/loading (entity exists) or a task is already in flight.
        if index.contains_key(&key) || pending.pending.contains_key(&key) {
            stats.tiles_skipped += 1;
            continue;
        }

        let url = match build_terrain_url(&config, x, y, level) {
            Some(u) => u,
            None => {
                stats.tiles_skipped += 1;
                continue;
            }
        };

        // Spawn a placeholder entity in `Loading` state so the LOD system treats
        // the tile as existing (no duplicate requests) and the loader owns it.
        let entity = commands
            .spawn((
                CesiumTerrainTile { x, y, level },
                TerrainTileReady {
                    terrain_mesh: None,
                    state: TileContentState::Loading,
                },
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        index.insert(key, entity);

        // Dispatch async download + decode onto the IO task pool (never blocks
        // the frame thread). The worker yields only the CPU-side `TerrainMesh`.
        let rect = tile_rectangle(x, y, level);
        let skirt_height = skirt_height_for_level(level);
        let task = pool.spawn(async move {
            load_and_decode_terrain(&url, &rect, &ellipsoid, skirt_height, use_pipeline)
        });

        pending.pending.insert(key, PendingLoad { entity, task });
        stats.tiles_pending += 1;
    }
}

#[derive(Component)]
pub struct TerrainTileReady {
    pub terrain_mesh: Option<cesium_terrain::TerrainMesh>,
    pub state: TileContentState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_terrain_url_xyz() {
        let config = GlobeConfig {
            terrain_provider_url: Some("https://tiles.example.com/{z}/{x}/{y}.terrain".into()),
            ..Default::default()
        };
        let url = build_terrain_url(&config, 3, 1, 2);
        assert_eq!(url.unwrap(), "https://tiles.example.com/2/3/1.terrain");
    }

    #[test]
    fn test_build_terrain_url_none() {
        let config = GlobeConfig::default();
        let url = build_terrain_url(&config, 0, 0, 0);
        assert!(url.is_none());
    }

    #[test]
    fn test_tile_rectangle_level_0() {
        let rect = tile_rectangle(0, 0, 0);
        assert!(rect.west <= rect.east);
        assert!(rect.south <= rect.north);
    }

    #[test]
    fn test_tile_rectangle_hemisphere() {
        let rect0 = tile_rectangle(0, 0, 1);
        let rect1 = tile_rectangle(1, 0, 1);
        assert!(rect0.east <= rect1.west + 1e-10);
    }

    #[test]
    fn test_terrain_load_state_default() {
        let state = TerrainLoadState::default();
        assert_eq!(state.loaded_count, 0);
        assert_eq!(state.failed_count, 0);
    }

    #[test]
    fn test_pending_loads_default() {
        let pending = TerrainPendingLoads::default();
        assert!(pending.pending.is_empty());
    }

    #[test]
    fn test_skirt_height_halves_per_level() {
        // Skirt height must shrink geometrically with level (÷2 per level).
        let l0 = skirt_height_for_level(0);
        let l1 = skirt_height_for_level(1);
        let l2 = skirt_height_for_level(2);
        assert!(l0 > 0.0);
        assert!((l1 - l0 / 2.0).abs() < 1e-6);
        assert!((l2 - l0 / 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_skirt_height_high_level_no_overflow() {
        // Levels beyond 32 are clamped; the helper must not overflow or panic.
        let h = skirt_height_for_level(64);
        assert!(h >= 0.0);
    }

    #[test]
    fn test_skirt_height_level0_is_physically_bounded() {
        // BLOCKER regression: the level-0 skirt must stay in a physically sane
        // range (metres / hundreds of metres), far below Earth's radius
        // (~6.4e6 m), so `carto.height - skirt` in add_skirts never folds
        // vertices through the geocenter. It was previously ~5e7 m.
        let l0 = skirt_height_for_level(0);
        assert!(l0 > 0.0);
        assert!(l0 < 5_000.0, "level-0 skirt {} m is unphysically large", l0);
        assert!(l0 <= MAX_SKIRT_HEIGHT);
    }
}
