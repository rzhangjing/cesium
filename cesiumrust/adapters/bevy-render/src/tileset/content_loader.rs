//! Asynchronous 3D Tiles content loading (content -> mesh).
//!
//! Per-tile pipeline:
//! 1. `tileset_traversal_system` diffs the selection and pushes new paths into
//!    `TileSelection::tiles_to_load`.
//! 2. This system is the *sole* consumer of that queue: it spawns a placeholder
//!    entity in [`TileContentState::Loading`] and dispatches download + decode +
//!    mesh building onto Bevy's [`IoTaskPool`] (never blocking the frame thread).
//! 3. Later frames poll each task exactly once (`poll_once`) and transition the
//!    placeholder to `Ready` (storing the GPU handles in [`TileContent`]),
//!    `Failed`, or despawn it when the format is unsupported.
//!
//! The render system scans component state (`Ready` + `mesh_handle.is_some()`)
//! instead of draining the same queue, which structurally removes the previous
//! load/render drain race.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::GeometryData;
use cesium_tileset::content_decoder::{
    decode_tile_content, detect_content_type, DecodedTile, TileContentType,
};

use crate::components::{CesiumTileNode, TileContent, TileContentState};
use crate::pipeline::{self, budget};
use crate::resources::TileLoadStats;

use super::loader::LoadedTileset;
use super::traversal_system::TileSelection;

/// In-flight asynchronous tile content loads.
///
/// Keyed by tile path; each entry owns the spawned [`IoTaskPool`] task plus the
/// placeholder entity that receives the decoded mesh once the task resolves.
#[derive(Resource, Default)]
pub struct PendingTileLoads {
    pub pending: HashMap<Vec<usize>, TileLoadRequest>,
    /// Content URIs already rejected as unsupported, so a skipped tile is never
    /// re-dispatched on every frame.
    pub skipped_uris: HashSet<String>,
    /// The tileset URL the `skipped_uris` set belongs to. When the active
    /// [`LoadedTileset`] switches to a different URL the set is cleared, so a
    /// skip decision from a previous tileset never leaks into the new one.
    pub tileset_url: String,
    /// Resolved-but-not-yet-uploaded content, drained FIFO up to the per-frame
    /// tileset mesh budget (gate ON). Always fully drained in-frame when gate
    /// OFF (budget = `UNBOUNDED`), so the legacy behaviour is unchanged.
    pub ready_backlog: VecDeque<ResolvedTileLoad>,
}

/// A content load whose worker task has resolved, awaiting its per-frame mesh
/// upload slot. Holds everything the poll step needs to build GPU handles and
/// (re)attach [`CesiumTileNode`] without re-borrowing the domain `Tile`.
pub struct ResolvedTileLoad {
    pub path: Vec<usize>,
    pub url: String,
    pub entity: Entity,
    pub meta: TileNodeMeta,
    pub payload: TileLoadPayload,
}

pub struct TileLoadRequest {
    pub path: Vec<usize>,
    pub url: String,
    /// Placeholder entity spawned in `Loading` state.
    pub entity: Entity,
    /// Tile metadata captured at dispatch time (the domain `Tile` borrow cannot
    /// outlive the frame, and the task resolves on a later one).
    pub meta: TileNodeMeta,
    /// Background download + decode + mesh build.
    pub task: Task<TileLoadPayload>,
}

/// Immutable tile metadata needed to (re)build [`CesiumTileNode`].
#[derive(Debug, Clone, Copy)]
pub struct TileNodeMeta {
    pub geometric_error: f64,
    pub screen_space_error: f64,
    /// Tile bounding-volume center in ECEF metres (f64) — also the RTC center.
    pub bounding_center: glam::DVec3,
    pub bounding_radius: f64,
}

/// Worker result: CPU-side data only. GPU handles are created on the frame
/// thread, because `Assets<T>` is not accessible from a task-pool worker.
pub struct TileLoadPayload {
    pub bytes_downloaded: u64,
    pub outcome: TileLoadOutcome,
}

pub enum TileLoadOutcome {
    /// Decoded successfully; mesh already recentered on the RTC center.
    Ready(PreparedTileContent),
    /// Unsupported format (Draco / pnts / cmpt / subt / external i3dm URI):
    /// warn + count as skipped, never as a failure.
    Skipped(String),
    /// Real failure (network error, malformed payload): count as failed.
    Failed(String),
}

pub struct PreparedTileContent {
    pub mesh: Mesh,
    pub has_batch_table: bool,
}

/// Separates "unsupported, degrade gracefully" from "broken, report".
#[derive(Debug)]
enum ContentError {
    Unsupported(String),
    Invalid(String),
}

impl ContentError {
    fn unsupported(msg: impl Into<String>) -> Self {
        ContentError::Unsupported(msg.into())
    }

    fn invalid(msg: impl Into<String>) -> Self {
        ContentError::Invalid(msg.into())
    }
}

struct DecodedGlb {
    glb: Vec<u8>,
    has_batch_table: bool,
}

/// Downloads raw tile bytes.
///
/// Blocking; intended for an [`IoTaskPool`] worker thread. The fetch is
/// tokio-free — it routes through the cesium-pipeline core's ureq blocking
/// backend ([`pipeline::fetch::fetch_gated`]), selecting the shared keep-alive
/// pool (gate ON) or a fresh per-call client (gate OFF).
fn fetch_tile_bytes(url: &str, use_pipeline: bool) -> Result<Vec<u8>, String> {
    pipeline::fetch::fetch_gated(url, use_pipeline)
}

/// Classifies the payload by magic bytes, then decodes only supported formats.
///
/// Classification happens *before* decoding: `decode_tile_content` reports
/// `InvalidMagic` for e.g. `subt`/`geom`/`vctr`, which would otherwise be
/// mis-counted as a hard failure instead of a graceful skip.
fn extract_glb(raw: &[u8]) -> Result<DecodedGlb, ContentError> {
    let content_type = detect_content_type(raw);
    match content_type {
        // Only b3dm and bare GLB are renderable today. i3dm (instanced) is
        // deliberately excluded here as well as below: rejecting it at the
        // classification gate guarantees *every* i3dm becomes a graceful skip
        // (`tiles_skipped`) rather than risking a malformed-instance decode being
        // mis-counted as a hard failure.
        TileContentType::Batched3DModel | TileContentType::GltfBinary => {}
        other => {
            return Err(ContentError::unsupported(format!(
                "content format {:?} is not supported yet",
                other
            )))
        }
    }

    let decoded =
        decode_tile_content(raw).map_err(|e| ContentError::invalid(format!("decode: {}", e)))?;

    match decoded {
        DecodedTile::B3dm(b3dm) => Ok(DecodedGlb {
            has_batch_table: b3dm.batch_table_json.is_some() || !b3dm.batch_table_binary.is_empty(),
            glb: b3dm.gltf,
        }),
        // i3dm (instanced) is unsupported: neither the embedded-GLB
        // (`gltf_format == 1`) nor the external-URI (`== 0`) variant. Drawing the
        // prototype once would be a wrong half-state (a single copy at the RTC
        // center, every per-instance transform ignored, yet counted as loaded).
        // The classification gate above already rejects i3dm, so this arm is a
        // defensive exhaustive fallback — it must stay a skip, never a render.
        DecodedTile::I3dm(_) => Err(ContentError::unsupported(
            "i3dm instancing is not supported yet",
        )),
        DecodedTile::Glb(glb) => Ok(DecodedGlb {
            glb,
            has_batch_table: false,
        }),
        DecodedTile::Pnts(_) => Err(ContentError::unsupported(
            "pnts point clouds are not supported yet",
        )),
        DecodedTile::Cmpt(_) => Err(ContentError::unsupported(
            "cmpt composite tiles are not supported yet",
        )),
    }
}

/// Full CPU-side worker: fetch -> classify/decode -> parse glTF -> build the
/// Bevy mesh with the RTC center subtracted in f64 before the f32 cast.
///
/// Runs on an [`IoTaskPool`] worker thread; returns only CPU-side data.
fn load_tile_content(url: &str, rtc_center: glam::DVec3, use_pipeline: bool) -> TileLoadPayload {
    let raw = match fetch_tile_bytes(url, use_pipeline) {
        Ok(bytes) => bytes,
        Err(e) => {
            return TileLoadPayload {
                bytes_downloaded: 0,
                outcome: TileLoadOutcome::Failed(e),
            }
        }
    };
    let bytes_downloaded = raw.len() as u64;

    let outcome = match extract_glb(&raw)
        .and_then(|decoded| parse_glb_to_geometry(&decoded.glb).map(|g| (g, decoded.has_batch_table)))
    {
        Ok((geometry, has_batch_table)) => TileLoadOutcome::Ready(PreparedTileContent {
            mesh: crate::geometry_to_mesh(&geometry, Some(rtc_center)),
            has_batch_table,
        }),
        Err(ContentError::Unsupported(reason)) => TileLoadOutcome::Skipped(reason),
        Err(ContentError::Invalid(reason)) => TileLoadOutcome::Failed(reason),
    };

    TileLoadPayload {
        bytes_downloaded,
        outcome,
    }
}

/// Neutral material until per-tile materials/textures are wired up.
fn default_tile_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.8, 0.8, 0.8),
        ..default()
    }
}

fn tile_node(path: Vec<usize>, meta: TileNodeMeta, state: TileContentState) -> CesiumTileNode {
    CesiumTileNode {
        path,
        screen_space_error: meta.screen_space_error,
        geometric_error: meta.geometric_error,
        state,
        bounding_sphere_center: Some(meta.bounding_center),
        bounding_sphere_radius: Some(meta.bounding_radius),
    }
}

/// Polls in-flight tasks (once each, so the frame thread never parks) and
/// transitions resolved tiles `Loading -> Ready/Failed`, or despawns the
/// placeholder of a tile whose content format is unsupported.
///
/// Resolved payloads are moved into a FIFO backlog and uploaded up to `budget`
/// meshes per frame — `UNBOUNDED` (gate OFF) drains everything in-frame exactly
/// like the pre-migration code, while gate ON bounds GPU work to the tileset mesh
/// weight ([`budget::tileset_mesh_budget`]).
fn poll_pending_loads(
    commands: &mut Commands,
    pending_loads: &mut PendingTileLoads,
    stats: &mut TileLoadStats,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    budget: usize,
) {
    // 1. Poll every in-flight task once; move resolved payloads into the backlog.
    //    A finished task is drained here and never re-polled, so the budget can
    //    defer the GPU upload without touching a completed `Task`.
    let mut resolved: Vec<(Vec<usize>, TileLoadPayload)> = Vec::new();
    for (path, request) in pending_loads.pending.iter_mut() {
        if let Some(payload) = block_on(poll_once(&mut request.task)) {
            resolved.push((path.clone(), payload));
        }
    }
    for (path, payload) in resolved {
        // Safe: `path` was just read from `pending` and nothing removed it.
        let request = pending_loads
            .pending
            .remove(&path)
            .expect("pending tile load vanished during poll");
        pending_loads.ready_backlog.push_back(ResolvedTileLoad {
            path: request.path,
            url: request.url,
            entity: request.entity,
            meta: request.meta,
            payload,
        });
    }

    // 2. Upload up to `budget` meshes this frame (FIFO).
    let mut uploaded = 0;
    while uploaded < budget {
        let Some(resolved_load) = pending_loads.ready_backlog.pop_front() else {
            break;
        };
        uploaded += 1;
        let ResolvedTileLoad {
            path,
            url,
            entity,
            meta,
            payload,
        } = resolved_load;

        stats.tiles_pending = stats.tiles_pending.saturating_sub(1);
        stats.bytes_downloaded += payload.bytes_downloaded;

        match payload.outcome {
            TileLoadOutcome::Ready(content) => {
                let mesh_handle = meshes.add(content.mesh);
                let material_handle = materials.add(default_tile_material());

                // `try_insert`: the tile may have been unloaded while in flight.
                commands.entity(entity).try_insert((
                    TileContent {
                        mesh_handle: Some(mesh_handle),
                        material_handle: Some(material_handle),
                        has_batch_table: content.has_batch_table,
                    },
                    tile_node(path, meta, TileContentState::Ready),
                ));
                stats.tiles_loaded += 1;
            }
            TileLoadOutcome::Skipped(reason) => {
                // Graceful degradation: warn + skip, never a failure or a panic.
                warn!("Skipping tile {:?} ({}): {}", path, url, reason);
                pending_loads.skipped_uris.insert(url);
                commands.entity(entity).try_despawn();
                stats.tiles_skipped += 1;
            }
            TileLoadOutcome::Failed(reason) => {
                error!("Failed to load tile {:?} ({}): {}", path, url, reason);
                commands
                    .entity(entity)
                    .try_insert(tile_node(path, meta, TileContentState::Failed));
                stats.tiles_failed += 1;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn tile_content_load_system(
    mut commands: Commands,
    loaded: Option<Res<LoadedTileset>>,
    selection: ResMut<TileSelection>,
    mut pending_loads: ResMut<PendingTileLoads>,
    mut stats: ResMut<TileLoadStats>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tile_query: Query<&CesiumTileNode>,
) {
    // Gate: read once per frame. ON routes fetches through the cesium-pipeline
    // core's shared keep-alive ureq pool and bounds uploads to the tileset mesh
    // weight; OFF keeps the legacy per-call fetch and drains all resolved tiles.
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let upload_budget = if use_pipeline {
        budget::tileset_mesh_budget()
    } else {
        budget::UNBOUNDED
    };

    // Retire tasks that resolved since the previous frame.
    poll_pending_loads(
        &mut commands,
        &mut pending_loads,
        &mut stats,
        &mut meshes,
        &mut materials,
        upload_budget,
    );

    let loaded = match loaded {
        Some(l) => l,
        None => return,
    };

    let tileset_json = match &loaded.tileset_json {
        Some(ts) => ts,
        None => return,
    };

    // Tileset switch: drop skip decisions that belonged to a different tileset so
    // they never suppress content in the newly active one.
    if pending_loads.tileset_url != loaded.url {
        pending_loads.skipped_uris.clear();
        pending_loads.tileset_url = loaded.url.clone();
    }

    // Disjoint field borrows of one resource: draining `tiles_to_load` while
    // reading `selected_tiles` for the per-tile screen space error. `into_inner`
    // consumes the `ResMut` and flags the change-detection tick.
    let sel = selection.into_inner();

    let ellipsoid = Ellipsoid::WGS84;
    let pool = IoTaskPool::get();

    // Path index of the tile nodes that already exist. Scanning the query per
    // requested tile was O(n*m); this also guarantees one entity per path.
    let mut known_paths: HashSet<Vec<usize>> = tile_query
        .iter()
        .map(|node| node.path.clone())
        .collect();

    for path in sel.tiles_to_load.drain(..) {
        // Already loaded/loading/failed in an earlier frame, or in flight now.
        // Counted as a skip to match the terrain loader's accounting for a
        // duplicate key (the request is intentionally not re-dispatched).
        if known_paths.contains(&path) || pending_loads.pending.contains_key(&path) {
            stats.tiles_skipped += 1;
            continue;
        }

        let tile = match cesium_tileset::lod_selection::get_tile_by_path(&tileset_json.root, &path) {
            Some(t) => t,
            // Path not in this tileset (stale selection) — nothing to load.
            None => continue,
        };

        // Pure group tiles carry no renderable content of their own.
        let uri = match tile.content_uris().first() {
            Some(u) => (*u).to_string(),
            None => continue,
        };

        let url = loaded.state.resolve_uri(&uri);

        // Known-unsupported content: do not re-dispatch it every frame. Counted
        // as a skip for the same accounting reason as the duplicate-path guard.
        if pending_loads.skipped_uris.contains(&url) {
            stats.tiles_skipped += 1;
            continue;
        }

        let bounding = tile.bounding_volume.to_bounding_sphere(&ellipsoid);

        let screen_space_error = sel
            .selected_tiles
            .iter()
            .find(|t| t.path == path)
            .map(|t| t.screen_space_error)
            .unwrap_or(0.0);

        let meta = TileNodeMeta {
            geometric_error: tile.geometric_error,
            screen_space_error,
            bounding_center: bounding.center,
            bounding_radius: bounding.radius,
        };

        // Placeholder in `Loading` state; the render system only spawns meshes
        // for `Ready` nodes that actually carry a mesh handle.
        let entity = commands
            .spawn((
                tile_node(path.clone(), meta, TileContentState::Loading),
                TileContent {
                    mesh_handle: None,
                    material_handle: None,
                    has_batch_table: false,
                },
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        known_paths.insert(path.clone());

        // RTC center = tile bounding-volume center (ECEF metres, f64). Vertices
        // are recentered on it in f64 before the f32 cast, and the render entity
        // is placed back at `center / METERS_PER_RENDER_UNIT`.
        let rtc_center = bounding.center;
        let task_url = url.clone();
        let task = pool.spawn(async move { load_tile_content(&task_url, rtc_center, use_pipeline) });

        pending_loads.pending.insert(
            path.clone(),
            TileLoadRequest {
                path,
                url,
                entity,
                meta,
                task,
            },
        );
        stats.tiles_pending += 1;
    }
}

/// `CESIUM_ENABLE_GLTF_UPGRADE` — glTF 1.0 → 2.0 upgrade gate. The name matches
/// the cesium-app feature-flag registry (`feature_flags::ENV_ENABLE_GLTF_UPGRADE`)
/// so both read paths observe the identical env var. bevy-render does not depend
/// on the application layer, so the gate is evaluated here through the M1.4
/// [`pipeline::fetch::gate_from_env_value`] precedent (defaults OFF).
const ENV_ENABLE_GLTF_UPGRADE: &str = "CESIUM_ENABLE_GLTF_UPGRADE";

/// Reads the glTF-upgrade gate without depending on the application layer.
/// Defaults OFF, so the existing glTF 2.0 render golden path stays byte-for-byte
/// unchanged unless `CESIUM_ENABLE_GLTF_UPGRADE` is explicitly truthy.
fn gltf_upgrade_gate_enabled() -> bool {
    pipeline::fetch::gate_from_env_value(std::env::var(ENV_ENABLE_GLTF_UPGRADE).ok())
}

/// Decodes an embedded GLB into the typed [`cesium_gltf::gltf_model::GltfModel`]
/// plus its per-buffer byte sources (`buffers[i]` is the decoded source of
/// `gltf.buffers[i]`; for a GLB `buffers[0]` is the embedded binary chunk).
///
/// Gate OFF (`upgrade_enabled == false`): delegates straight to
/// [`GlbData::from_bytes`] — **byte-for-byte identical** to the pre-M9.2 path,
/// protecting the existing glTF 2.0 render golden path.
///
/// Gate ON: parses the container *untyped* ([`parse_glb_container`]); a payload
/// that is not already 2.0 is run through [`detect_version`] +
/// [`update_version_with_buffers`] (threading the embedded binary chunk as
/// `buffers[0]` to drive the M9.2 binary stage) before the typed
/// [`GltfModel::from_value`] deserialization. An already-2.0 payload is a pure
/// passthrough (zero mutation), so even gate ON never disturbs a valid 2.0 asset.
fn decode_gltf_model(
    glb: &[u8],
    upgrade_enabled: bool,
) -> Result<(cesium_gltf::gltf_model::GltfModel, Vec<Vec<u8>>), ContentError> {
    if !upgrade_enabled {
        let glb_data = cesium_gltf::binary_format::GlbData::from_bytes(glb)
            .map_err(|e| ContentError::invalid(format!("GLB parse: {}", e)))?;
        // GLB buffer 0 is the embedded binary chunk.
        let buffers = vec![glb_data.binary_chunk.unwrap_or_default()];
        return Ok((glb_data.model, buffers));
    }

    let (mut value, binary_chunk) = cesium_gltf::binary_format::parse_glb_container(glb)
        .map_err(|e| ContentError::invalid(format!("GLB parse: {}", e)))?;
    let mut buffers: Vec<Vec<u8>> = vec![binary_chunk.unwrap_or_default()];

    if cesium_gltf::gltf_upgrade::detect_version(&value)
        != cesium_gltf::gltf_upgrade::GltfVersion::V20
    {
        cesium_gltf::gltf_upgrade::update_version_with_buffers(
            &mut value,
            &cesium_gltf::gltf_upgrade::UpgradeOptions::default(),
            &mut buffers,
        )
        .map_err(|e| ContentError::invalid(format!("glTF upgrade: {}", e)))?;
    }

    let model = cesium_gltf::gltf_model::GltfModel::from_value(value)
        .map_err(|e| ContentError::invalid(format!("glTF parse: {}", e)))?;
    Ok((model, buffers))
}

/// Parses an embedded GLB into CPU-side geometry.
///
/// Chunk walking is delegated to the domain parser (12-byte header, then
/// `length|type|data` chunks). The previous hand-rolled slicing read the JSON
/// chunk from byte 16 instead of 20, i.e. it tried to parse the 4-byte chunk
/// type as JSON and therefore failed on every real payload — no tile could ever
/// produce a mesh.
///
/// The glTF 1.0 → 2.0 upgrade route is selected by the
/// `CESIUM_ENABLE_GLTF_UPGRADE` gate (see [`decode_gltf_model`]) and defaults
/// OFF, leaving the existing 2.0 path byte-for-byte unchanged.
///
/// Draco-compressed payloads are rejected as `Unsupported` (the decoder backend
/// is still a stub) so the caller degrades gracefully instead of emitting an
/// empty/garbage mesh.
fn parse_glb_to_geometry(glb: &[u8]) -> Result<GeometryData, ContentError> {
    parse_glb_to_geometry_gated(glb, gltf_upgrade_gate_enabled())
}

/// Testable seam for [`parse_glb_to_geometry`]: `upgrade_enabled` selects the
/// gate ON/OFF decode route directly, without mutating process-global env
/// (which would race parallel tests).
fn parse_glb_to_geometry_gated(
    glb: &[u8],
    upgrade_enabled: bool,
) -> Result<GeometryData, ContentError> {
    let (gltf, buffers) = decode_gltf_model(glb, upgrade_enabled)?;

    if gltf
        .extensions_required
        .iter()
        .any(|ext| ext == "KHR_draco_mesh_compression")
    {
        return Err(ContentError::unsupported(
            "KHR_draco_mesh_compression (Draco) is not supported yet",
        ));
    }

    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut normals: Vec<[f64; 3]> = Vec::new();
    let mut tex_coords: Vec<[f64; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut index_offset: u32 = 0;

    for mesh in &gltf.meshes {
        for prim in &mesh.primitives {
            let pos_count = prim
                .attributes
                .get("POSITION")
                .and_then(|&idx| gltf.accessors.get(idx))
                .map(|a| a.count)
                .unwrap_or(0);

            if let Some(&pos_idx) = prim.attributes.get("POSITION") {
                if let Some(acc) = gltf.accessors.get(pos_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(3) {
                        positions.push([chunk[0] as f64, chunk[1] as f64, chunk[2] as f64]);
                    }
                }
            }

            if let Some(&nrm_idx) = prim.attributes.get("NORMAL") {
                if let Some(acc) = gltf.accessors.get(nrm_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(3) {
                        normals.push([chunk[0] as f64, chunk[1] as f64, chunk[2] as f64]);
                    }
                }
            }

            if let Some(&uv_idx) = prim.attributes.get("TEXCOORD_0") {
                if let Some(acc) = gltf.accessors.get(uv_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(2) {
                        tex_coords.push([chunk[0] as f64, chunk[1] as f64]);
                    }
                }
            }

            if let Some(idx_idx) = prim.indices {
                if let Some(acc) = gltf.accessors.get(idx_idx) {
                    let use_u16 = matches!(
                        acc.component_type,
                        cesium_gltf::gltf_model::ComponentType::U16
                    );
                    if use_u16 {
                        let idx_data = acc.read_u16_data(&buffers, &gltf.buffer_views);
                        for idx in &idx_data {
                            indices.push(*idx as u32 + index_offset);
                        }
                    } else {
                        let idx_data = acc.read_u32_data(&buffers, &gltf.buffer_views);
                        for idx in &idx_data {
                            indices.push(*idx + index_offset);
                        }
                    }
                }
            }

            index_offset += pos_count as u32;
        }
    }

    if positions.is_empty() {
        return Err(ContentError::invalid("No vertex positions found"));
    }

    let bounding_sphere = cesium_geospatial::bounding::BoundingSphere::from_points(
        &positions
            .iter()
            .map(|p| glam::DVec3::new(p[0], p[1], p[2]))
            .collect::<Vec<_>>(),
    );

    Ok(GeometryData {
        positions,
        normals: if normals.is_empty() { None } else { Some(normals) },
        tex_coords: if tex_coords.is_empty() {
            None
        } else {
            Some(tex_coords)
        },
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: cesium_geospatial::geometry::PrimitiveType::Triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal GLB container wrapping `json` (4-byte aligned, no BIN
    /// chunk) so the header/JSON parsing path can be exercised without a file.
    fn build_glb(json: &str) -> Vec<u8> {
        build_glb_with_bin(json, &[])
    }

    /// Same as [`build_glb`], plus a trailing BIN chunk when `bin` is non-empty.
    fn build_glb_with_bin(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut json_chunk = json.as_bytes().to_vec();
        // GLB chunks are 4-byte aligned; JSON is padded with spaces (0x20).
        let json_pad = (4 - json_chunk.len() % 4) % 4;
        json_chunk.resize(json_chunk.len() + json_pad, b' ');

        let mut bin_chunk = bin.to_vec();
        // BIN chunks are 4-byte aligned too, padded with zeroes.
        let bin_pad = (4 - bin_chunk.len() % 4) % 4;
        bin_chunk.resize(bin_chunk.len() + bin_pad, 0);

        let bin_total = if bin_chunk.is_empty() {
            0
        } else {
            8 + bin_chunk.len()
        };
        let total = 12 + 8 + json_chunk.len() + bin_total;

        let mut glb = Vec::with_capacity(total);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
        glb.extend_from_slice(&json_chunk);

        if !bin_chunk.is_empty() {
            glb.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
            glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
            glb.extend_from_slice(&bin_chunk);
        }

        glb
    }

    #[test]
    fn test_detect_content_types() {
        assert_eq!(
            detect_content_type(b"b3dm...."),
            TileContentType::Batched3DModel
        );
        assert_eq!(
            detect_content_type(b"i3dm...."),
            TileContentType::Instanced3DModel
        );
        assert_eq!(
            detect_content_type(b"glTF...."),
            TileContentType::GltfBinary
        );
        assert_eq!(detect_content_type(b"xxxx...."), TileContentType::Unknown);
    }

    #[test]
    fn test_decode_b3dm_to_gltf() {
        let gltf_content = b"glTF test glb data here".to_vec();
        let b3dm = cesium_tileset::content_decoder::B3dmContent {
            batch_length: 1,
            feature_table_json: None,
            feature_table_binary: vec![],
            batch_table_json: None,
            batch_table_binary: vec![],
            gltf: gltf_content.clone(),
        };
        let decoded = DecodedTile::B3dm(b3dm);
        match decoded {
            DecodedTile::B3dm(c) => assert_eq!(c.gltf, gltf_content),
            _ => panic!("Expected B3dm"),
        }
    }

    #[test]
    fn test_pending_loads_default() {
        let pending = PendingTileLoads::default();
        assert!(pending.pending.is_empty());
        assert!(pending.skipped_uris.is_empty());
    }

    #[test]
    fn test_parse_glb_rejects_draco() {
        // A required Draco extension must degrade to `Unsupported` (warn + skip),
        // never to a hard failure or an empty mesh.
        let glb = build_glb(
            r#"{"asset":{"version":"2.0"},"extensionsRequired":["KHR_draco_mesh_compression"]}"#,
        );

        match parse_glb_to_geometry(&glb) {
            Err(ContentError::Unsupported(reason)) => {
                assert!(reason.contains("Draco"), "unexpected reason: {}", reason);
            }
            other => panic!("expected Unsupported(Draco), got {:?}", other.err()),
        }
    }

    #[test]
    fn test_parse_glb_accepts_plain_gltf() {
        // Same container without the Draco requirement passes the extension gate
        // (it then fails later only because this fixture has no vertex data).
        let glb = build_glb(r#"{"asset":{"version":"2.0"}}"#);

        match parse_glb_to_geometry(&glb) {
            Err(ContentError::Invalid(_)) => {}
            Err(ContentError::Unsupported(reason)) => {
                panic!("plain glTF must not be treated as unsupported: {}", reason)
            }
            Ok(_) => panic!("fixture has no positions"),
        }
    }

    #[test]
    fn test_parse_glb_reads_positions_from_binary_chunk() {
        // Guards the GLB chunk layout fix: with a spec-correct container the
        // JSON chunk must be found at byte 20 and the BIN chunk right after it,
        // so the single triangle is actually decoded.
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }

        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":36}],
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],
            "accessors":[{"bufferView":0,"byteOffset":0,"componentType":5126,
                "count":3,"type":"VEC3"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}]}"#;

        let geometry = parse_glb_to_geometry(&build_glb_with_bin(json, &bin))
            .expect("a plain GLB triangle must decode");
        assert_eq!(geometry.positions.len(), 3);
        assert_eq!(geometry.positions[1], [1.0, 0.0, 0.0]);
    }

    /// M9.2-b: a glTF **1.0** payload (object-keyed collections) embedded in a
    /// GLB v2 container. Gate OFF must fail — the array-based typed model cannot
    /// deserialize object-keyed dictionaries, proving the legacy path is
    /// unchanged and 1.0 was never silently supported. Gate ON must run
    /// `detect_version` + `update_version_with_buffers` (objects→arrays, string
    /// ids→indices, min/max from the binary chunk) and decode the triangle.
    #[test]
    fn test_parse_glb_upgrades_gltf_10_only_when_gate_on() {
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }

        // glTF 1.0: every top-level collection is an object keyed by string id,
        // and references (bufferView / buffer / POSITION) are string ids too.
        let json = r#"{"asset":{"version":"1.0"},
            "buffers":{"buf":{"byteLength":36}},
            "bufferViews":{"bv":{"buffer":"buf","byteOffset":0,"byteLength":36}},
            "accessors":{"acc":{"bufferView":"bv","byteOffset":0,
                "componentType":5126,"count":3,"type":"VEC3"}},
            "meshes":{"mesh":{"primitives":[{"attributes":{"POSITION":"acc"}}]}}}"#;
        let glb = build_glb_with_bin(json, &bin);

        // Gate OFF: byte-for-byte the pre-M9.2 path → the typed deserialization
        // of the object-keyed 1.0 collections fails.
        assert!(
            parse_glb_to_geometry_gated(&glb, false).is_err(),
            "gate OFF must not decode a glTF 1.0 payload"
        );

        // Gate ON: 1.0 → 2.0 upgrade then typed decode.
        let geometry = parse_glb_to_geometry_gated(&glb, true)
            .expect("gate ON must upgrade a glTF 1.0 payload to 2.0");
        assert_eq!(geometry.positions.len(), 3);
        assert_eq!(geometry.positions[1], [1.0, 0.0, 0.0]);
        assert_eq!(geometry.positions[2], [0.0, 1.0, 0.0]);
    }

    /// M9.2-b gate neutrality: for an already-2.0 GLB the gate ON route is a
    /// pure passthrough (`detect_version == V20` ⇒ `update_version` is skipped),
    /// so it must yield geometry byte-identical to the gate OFF (legacy) route.
    /// Geometry → mesh → pixels is deterministic, hence identical geometry is the
    /// CPU-side proof of pixel zero-diff on the existing glTF 2.0 render path
    /// (the full GPU screenshot diff runs in the xvfb headless e2e CI).
    #[test]
    fn test_parse_glb_gate_on_is_passthrough_for_gltf_20() {
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":36}],
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],
            "accessors":[{"bufferView":0,"byteOffset":0,"componentType":5126,
                "count":3,"type":"VEC3"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}]}"#;
        let glb = build_glb_with_bin(json, &bin);

        let off = parse_glb_to_geometry_gated(&glb, false).expect("gate OFF decodes 2.0");
        let on = parse_glb_to_geometry_gated(&glb, true).expect("gate ON passthrough decodes 2.0");

        assert_eq!(
            off.positions, on.positions,
            "gate ON must not perturb 2.0 positions"
        );
        assert_eq!(
            off.indices, on.indices,
            "gate ON must not perturb 2.0 indices"
        );
        assert_eq!(off.normals.is_none(), on.normals.is_none());
        assert_eq!(off.tex_coords.is_none(), on.tex_coords.is_none());
    }

    #[test]
    fn test_extract_glb_skips_unsupported_formats() {
        // pnts / cmpt / subt / unknown magics are skipped, not failed.
        for magic in [b"pnts....", b"cmpt....", b"subt....", b"geom....", b"xxxx...."] {
            match extract_glb(magic) {
                Err(ContentError::Unsupported(_)) => {}
                other => panic!(
                    "expected Unsupported for {:?}, got {:?}",
                    &magic[0..4],
                    other.err()
                ),
            }
        }
    }

    #[test]
    fn test_extract_glb_accepts_glb_magic() {
        let glb = build_glb(r#"{"asset":{"version":"2.0"}}"#);
        let decoded = extract_glb(&glb).expect("glTF magic must be accepted");
        assert!(!decoded.has_batch_table);
        assert_eq!(decoded.glb, glb);
    }

    /// Wraps a GLB body in a minimal b3dm container: 28-byte header (magic,
    /// version 1, byteLength, then four zero table lengths) followed by the GLB.
    fn build_b3dm(glb: &[u8]) -> Vec<u8> {
        let total = 28 + glb.len();
        let mut b3dm = Vec::with_capacity(total);
        b3dm.extend_from_slice(b"b3dm");
        b3dm.extend_from_slice(&1u32.to_le_bytes()); // version
        b3dm.extend_from_slice(&(total as u32).to_le_bytes()); // byteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // featureTableJSONByteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // featureTableBinaryByteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // batchTableJSONByteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // batchTableBinaryByteLength
        b3dm.extend_from_slice(glb);
        b3dm
    }

    /// Always-on end-to-end guard for the numeric `BufferTarget` deserialization.
    ///
    /// A synthetic b3dm whose embedded GLB declares `bufferView.target` as the
    /// OpenGL integers 34962/34963. Before the fix the derived string-variant
    /// serde impl rejected these, and after the strictness change an *unknown*
    /// integer must error rather than silently degrade. Neither behaviour depends
    /// on an external sample file, so this runs on every CI machine.
    #[test]
    fn test_parse_synthetic_b3dm_numeric_buffer_target() {
        // 3 positions (36 bytes) then 3 u16 indices (6 bytes), padded to 44.
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let indices: [u16; 3] = [0, 1, 2];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }
        for i in indices {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        bin.resize(44, 0);

        // Both buffer views carry a numeric `target` (34962 / 34963).
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":44}],
            "bufferViews":[
                {"buffer":0,"byteOffset":0,"byteLength":36,"target":34962},
                {"buffer":0,"byteOffset":36,"byteLength":6,"target":34963}],
            "accessors":[
                {"bufferView":0,"byteOffset":0,"componentType":5126,"count":3,"type":"VEC3"},
                {"bufferView":1,"byteOffset":0,"componentType":5123,"count":3,"type":"SCALAR"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}]}"#;

        let b3dm = build_b3dm(&build_glb_with_bin(json, &bin));
        let decoded = extract_glb(&b3dm).expect("synthetic b3dm must extract a GLB");
        let geometry = parse_glb_to_geometry(&decoded.glb)
            .expect("numeric bufferView.target (34962/34963) must deserialize");
        assert_eq!(geometry.positions.len(), 3);
        assert_eq!(geometry.indices, vec![0, 1, 2]);
    }

    /// Regression: a real b3dm from the Cesium sample tileset must decode all the
    /// way to geometry. Guards the `BufferTarget` numeric-deserialization fix —
    /// `bufferView.target` is 34962/34963 (integers), which the previously
    /// derived string-variant serde impl rejected with a JSON parse error.
    ///
    /// The path is resolved relative to `CARGO_MANIFEST_DIR` (three levels up to
    /// the repo root, which also holds the CesiumJS `Apps/SampleData` tree) so it
    /// works on any checkout, not just the original author's machine. When the
    /// sample data is absent the test skips; the synthetic test above is the
    /// always-on guard.
    #[test]
    fn test_parse_real_parent_b3dm_to_geometry() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../Apps/SampleData/Cesium3DTiles/Tilesets/Tileset/parent.b3dm"
        ));
        if !path.exists() {
            eprintln!("skipping: sample b3dm not found at {}", path.display());
            return;
        }
        let raw = std::fs::read(path).expect("read parent.b3dm");
        let decoded = extract_glb(&raw).expect("b3dm must extract a GLB");
        let geometry = parse_glb_to_geometry(&decoded.glb)
            .expect("embedded GLB must parse to geometry (BufferTarget fix)");
        assert_eq!(geometry.positions.len(), 240, "parent.b3dm has 240 vertices");
        assert!(!geometry.indices.is_empty(), "must have triangle indices");
    }
}
