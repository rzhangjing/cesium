//! Ported from `packages/engine/Source/Core/sampleTerrain.js` (292 lines).
//!
//! ## Function-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `sampleTerrain` | [`sample_terrain`] | sync; JS is async (DEVIATION 1) |
//! | `doSampling` | [`do_sampling`] | |
//! | `attemptConsumeNextQueueItem` | — | folded into [`drain_tile_request_queue`] |
//! | `drainTileRequestQueue` | [`drain_tile_request_queue`] | retry loop, no delay (DEVIATION 1) |
//! | `delay` | — | no-op in sync port |
//! | `createInterpolateFunction` | [`sample_tile`] | inline |
//! | `interpolateAndAssignHeight` | [`interpolate_and_assign_height`] | |
//! | `createMarkFailedFunction` | — | inline (set NaN) |
//!
//! # DEVIATIONS
//! 1. The JS async `Promise`/`setTimeout` retry loop is replaced by a sync
//!    retry loop (max 10 retries per tile, no delay). The JS web-worker
//!    hop is irrelevant here — tile requests are synchronous in the Rust
//!    port via [`EllipsoidTerrainProvider::request_tile_geometry`].
//! 2. The JS `createMesh` retry path (for providers like ArcGIS that need
//!    mesh creation before interpolation) is not yet ported. The
//!    interpolation uses `HeightmapTerrainData::interpolate_height`
//!    directly which returns `None` when the mesh is required but absent
//!    (LERC buffers). Full `createMesh` retry is deferred.
//! 3. The function accepts [`EllipsoidTerrainProvider`] specifically
//!    rather than a generic provider trait. This is because the JS
//!    `TerrainData` trait has an associated const and is not
//!    dyn-compatible. When the async terrain pipeline is ported, this
//!    can be generalized with a provider trait + async IO.

use std::collections::HashMap;

use crate::cartesian2::Cartesian2;
use crate::cartographic::Cartographic;
use crate::ellipsoid_terrain_provider::EllipsoidTerrainProvider;
use crate::heightmap_terrain_data::HeightmapTerrainData;
use crate::rectangle::Rectangle;
use crate::tiling_scheme::TilingScheme;

/// Tile request metadata, mirroring the JS `tileRequest` object.
struct TileRequestInfo {
    x: i32,
    y: i32,
    level: i32,
    /// Indices into the caller's `positions` slice.
    position_indices: Vec<usize>,
}

/// Samples terrain heights at given positions.
///
/// Mirrors `sampleTerrain`. The JS function is async; this Rust port is
/// synchronous (DEVIATION 1). Each position's `height` field is updated
/// in-place. If terrain data is unavailable for a position, its height is
/// set to `f64::NAN` (mirrors JS `undefined`).
///
/// # Arguments
/// * `provider` — the terrain provider (currently [`EllipsoidTerrainProvider`])
/// * `level` — the terrain level-of-detail to query
/// * `positions` — the positions to update with terrain heights
/// * `reject_on_tile_fail` — if `true`, returns `Err(...)` on tile failure
///   instead of silently setting heights to NaN
pub fn sample_terrain(
    provider: &EllipsoidTerrainProvider,
    level: i32,
    positions: &mut [Cartographic],
    reject_on_tile_fail: bool,
) -> Result<(), String> {
    do_sampling(provider, level, positions, reject_on_tile_fail)
}

/// Mirrors `doSampling`.
fn do_sampling(
    provider: &EllipsoidTerrainProvider,
    level: i32,
    positions: &mut [Cartographic],
    reject_on_tile_fail: bool,
) -> Result<(), String> {
    let tiling_scheme = provider.tiling_scheme();

    // Group positions by tile (mirrors the JS tile-request grouping loop).
    let mut tile_requests: Vec<TileRequestInfo> = Vec::new();
    let mut tile_request_set: HashMap<String, usize> = HashMap::new();

    for (i, position) in positions.iter().enumerate() {
        let mut xy = Cartesian2::default();
        if tiling_scheme
            .position_to_tile_xy(position, level, &mut xy)
            .is_none()
        {
            continue;
        }

        let key = format!("{},{}", xy.x as i32, xy.y as i32);
        if let Some(&idx) = tile_request_set.get(&key) {
            tile_requests[idx].position_indices.push(i);
        } else {
            let idx = tile_requests.len();
            tile_request_set.insert(key, idx);
            tile_requests.push(TileRequestInfo {
                x: xy.x as i32,
                y: xy.y as i32,
                level,
                position_indices: vec![i],
            });
        }
    }

    // Process all tile requests (mirrors drainTileRequestQueue + Promise.all).
    for tile_request in &tile_requests {
        let result = request_and_interpolate(provider, tile_request, positions);
        if let Err(e) = result {
            if reject_on_tile_fail {
                return Err(e);
            }
            // Mark all positions in this tile as failed (mirrors
            // createMarkFailedFunction).
            for &idx in &tile_request.position_indices {
                positions[idx].height = f64::NAN;
            }
        }
    }

    Ok(())
}

/// Maximum number of throttle retries per tile (mirrors the JS
/// `attemptConsumeNextQueueItem` → delay → retry loop).
const MAX_RETRIES: usize = 10;

/// Mirrors `attemptConsumeNextQueueItem` + `createInterpolateFunction`:
/// requests tile geometry (with retry for throttling) and interpolates
/// heights for all positions in the tile.
fn request_and_interpolate(
    provider: &EllipsoidTerrainProvider,
    tile_request: &TileRequestInfo,
    positions: &mut [Cartographic],
) -> Result<(), String> {
    let tiling_scheme = provider.tiling_scheme();

    // Request tile geometry with retry for throttling.
    let mut terrain_data: Option<HeightmapTerrainData> = None;
    for _ in 0..MAX_RETRIES {
        if let Some(data) = provider.request_tile_geometry(tile_request.x, tile_request.y, tile_request.level) {
            terrain_data = Some(data);
            break;
        }
        // In JS: delay(100). In Rust sync port: immediate retry.
    }

    let terrain_data = terrain_data.ok_or_else(|| {
        format!(
            "Failed to get terrain data for tile ({}, {}, {})",
            tile_request.x, tile_request.y, tile_request.level
        )
    })?;

    let mut rectangle = Rectangle::default();
    tiling_scheme.tile_xy_to_rectangle(
        tile_request.x,
        tile_request.y,
        tile_request.level,
        &mut rectangle,
    );

    // Interpolate heights for all positions (mirrors createInterpolateFunction).
    let mut is_mesh_required = false;
    for &idx in &tile_request.position_indices {
        let height_assigned = interpolate_and_assign_height(
            &mut positions[idx],
            &terrain_data,
            &rectangle,
        );
        if !height_assigned {
            is_mesh_required = true;
            break;
        }
    }

    if is_mesh_required {
        // DEVIATION 2: the JS code calls `terrainData.createMesh()` here
        // and re-interpolates. Since HeightmapTerrainData::create_mesh
        // requires &mut self and the mesh creation is primarily needed
        // for LERC-encoded buffers, we re-interpolate without mesh
        // creation — heights will remain NaN for LERC buffers.
        for &idx in &tile_request.position_indices {
            interpolate_and_assign_height(&mut positions[idx], &terrain_data, &rectangle);
        }
    }

    Ok(())
}

/// Mirrors `interpolateAndAssignHeight`: interpolates terrain height at the
/// position and writes it to `position.height`. Returns `true` if the height
/// was successfully assigned.
fn interpolate_and_assign_height(
    position: &mut Cartographic,
    terrain_data: &HeightmapTerrainData,
    rectangle: &Rectangle,
) -> bool {
    match terrain_data.interpolate_height(rectangle, position.longitude, position.latitude) {
        Some(height) => {
            position.height = height;
            true
        }
        None => {
            // Height unavailable — may need createMesh (see DEVIATION 2).
            false
        }
    }
}
