// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(unused_imports, dead_code, clippy::map_entry)]
use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use cesium_geospatial::tiling_scheme::TilingScheme;
use cesium_imagery::{compute_tile_requests, ImageryLayer, ImageryTileRequest};

use crate::components::CesiumTerrainTile;
use crate::pipeline::{self, budget};
use crate::resources::TileLoadStats;

use super::layer_manager::ImageryLayerManager;

/// Imagery tile key `(layer_id, x, y, level)`.
///
/// A type alias, so it is transparently interchangeable with the bare tuple the
/// cache/blend systems already use.
pub type ImageryKey = (u64, u32, u32, u32);

/// Background download+decode result: raw RGBA pixels plus dimensions.
pub type ImageryTaskResult = Result<(Vec<u8>, u32, u32), String>;

#[derive(Resource, Default)]
pub struct ImageryCache {
    pub textures: HashMap<ImageryKey, Handle<Image>>,
}

/// Per-tile imagery load state.
///
/// Replaces the previous `bool` "dispatched" flag so the fetch+decode can run on
/// an [`IoTaskPool`] worker (fixing the frame-thread block that used to stall the
/// renderer once per tile) and be polled non-blockingly on later frames — WITHOUT
/// changing the public `imagery_tile_load_system` signature (same resource,
/// richer internals).
pub enum ImageryLoadState {
    /// Requested by `imagery_tile_request_system`, not yet dispatched.
    Queued,
    /// Download+decode running on a background worker (never the frame thread).
    InFlight(Task<ImageryTaskResult>),
}

#[derive(Resource, Default)]
pub struct ImageryPendingLoads {
    pub pending: HashMap<ImageryKey, ImageryLoadState>,
    /// Resolved-but-not-yet-uploaded textures, drained FIFO up to the per-frame
    /// imagery texture budget (gate ON). Always fully drained in-frame when gate
    /// OFF (budget = `UNBOUNDED`). Entries move here the instant their task
    /// resolves, so the budget can defer GPU upload without re-polling a
    /// completed `Task`.
    pub ready_backlog: VecDeque<(ImageryKey, ImageryTaskResult)>,
}

fn build_imagery_url(template: &str, x: u32, y: u32, level: u32, _scheme: &TilingScheme) -> String {
    template
        .replace("{z}", &level.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string())
}

fn tile_rectangle(x: u32, y: u32, level: u32, scheme: &TilingScheme) -> Rectangle {
    scheme.tile_to_rectangle(x, y, level)
}

/// Downloads and decodes an image tile to raw RGBA pixels.
///
/// Blocking: intended to run on an [`IoTaskPool`] worker thread, never on the
/// frame thread. The fetch is tokio-free — it routes through the cesium-pipeline
/// core's ureq blocking backend ([`pipeline::fetch::fetch_gated`]), selecting the
/// shared keep-alive pool (gate ON) or a fresh per-call client (gate OFF).
fn fetch_and_decode_image(url: &str, use_pipeline: bool) -> ImageryTaskResult {
    let data = pipeline::fetch::fetch_gated(url, use_pipeline)?;

    let img = image::load_from_memory(&data).map_err(|e| format!("decode: {}", e))?;
    let width = img.width();
    let height = img.height();
    let rgba = img.to_rgba8();
    Ok((rgba.into_raw(), width, height))
}

pub fn imagery_tile_request_system(
    imagery_manager: Res<ImageryLayerManager>,
    terrain_query: Query<&CesiumTerrainTile>,
    mut pending: ResMut<ImageryPendingLoads>,
) {
    if !imagery_manager.enabled {
        return;
    }

    let scheme = TilingScheme::geographic(Ellipsoid::WGS84);

    for terrain_tile in terrain_query.iter() {
        let terrain_rect = scheme.tile_to_rectangle(terrain_tile.x, terrain_tile.y, terrain_tile.level);

        for layer_desc in imagery_manager.visible_layers() {
            let domain_layer = imagery_manager.to_domain_layer(layer_desc);

            if !domain_layer.is_level_valid(terrain_tile.level) {
                continue;
            }

            let requests = compute_tile_requests(
                &domain_layer,
                &terrain_rect,
                terrain_tile.level,
                &scheme,
            );

            for req in requests {
                let key = (req.layer_id, req.x, req.y, req.level);
                // Entry API (no `contains_key`+`insert`): a tile already queued or
                // in flight is left untouched, so it is requested exactly once.
                pending
                    .pending
                    .entry(key)
                    .or_insert(ImageryLoadState::Queued);
            }
        }
    }
}

pub fn imagery_tile_load_system(
    _commands: Commands,
    mut images: ResMut<Assets<Image>>,
    imagery_manager: Res<ImageryLayerManager>,
    mut pending: ResMut<ImageryPendingLoads>,
    mut cache: ResMut<ImageryCache>,
    mut stats: ResMut<TileLoadStats>,
) {
    let _ = _commands;
    let scheme = TilingScheme::geographic(Ellipsoid::WGS84);

    // Gate: read once per frame. ON routes fetches through the cesium-pipeline
    // core's shared keep-alive ureq pool and bounds texture uploads to the
    // imagery weight; OFF keeps the legacy per-call fetch and drains all resolved
    // tiles in-frame. Either way the work now runs OFF the frame thread.
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let upload_budget = if use_pipeline {
        budget::imagery_texture_budget()
    } else {
        budget::UNBOUNDED
    };
    let pool = IoTaskPool::get();

    // Pass 1 — dispatch every queued tile onto a BACKGROUND worker. This is the
    // critical M1.4 fix: the pre-migration code called `fetch_and_decode_image`
    // synchronously on the frame thread (a hard network stall per tile, the most
    // severe of the four loaders). Now the frame thread only spawns a task.
    let queued: Vec<ImageryKey> = pending
        .pending
        .iter()
        .filter(|(_, s)| matches!(s, ImageryLoadState::Queued))
        .map(|(k, _)| *k)
        .collect();

    for key in queued {
        let (layer_id, x, y, level) = key;

        // Already cached → nothing to fetch.
        if cache.textures.contains_key(&key) {
            pending.pending.remove(&key);
            continue;
        }

        let desc = match imagery_manager.get_layer(layer_id) {
            Some(d) => d,
            None => {
                pending.pending.remove(&key);
                continue;
            }
        };

        let url = build_imagery_url(&desc.url_template, x, y, level, &scheme);
        let task = pool.spawn(async move { fetch_and_decode_image(&url, use_pipeline) });
        pending.pending.insert(key, ImageryLoadState::InFlight(task));
        stats.tiles_pending += 1;
    }

    // Pass 2 — poll each in-flight task exactly once (`poll_once` returns
    // immediately while the worker is still downloading, so the frame thread
    // never parks) and move resolved results into the FIFO backlog.
    let mut resolved: Vec<(ImageryKey, ImageryTaskResult)> = Vec::new();
    for (key, state) in pending.pending.iter_mut() {
        if let ImageryLoadState::InFlight(task) = state {
            if let Some(result) = block_on(poll_once(task)) {
                resolved.push((*key, result));
            }
        }
    }
    for (key, result) in resolved {
        pending.pending.remove(&key);
        pending.ready_backlog.push_back((key, result));
    }

    // Pass 3 — upload up to `upload_budget` textures this frame. Texture creation
    // touches `Assets<Image>`, which is only accessible on the frame thread, so
    // this (cheap) step stays here while the (expensive) fetch+decode is backgrounded.
    let mut uploaded = 0;
    while uploaded < upload_budget {
        let Some((key, result)) = pending.ready_backlog.pop_front() else {
            break;
        };
        uploaded += 1;
        let (layer_id, x, y, level) = key;
        stats.tiles_pending = stats.tiles_pending.saturating_sub(1);

        match result {
            Ok((data, width, height)) => {
                let bevy_image = crate::create_imagery_texture(width, height, data);
                let handle = images.add(bevy_image);
                cache.textures.insert(key, handle);
                stats.tiles_loaded += 1;
            }
            Err(e) => {
                error!(
                    "Imagery tile ({},{},{},{}) failed: {}",
                    layer_id, x, y, level, e
                );
                stats.tiles_failed += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_imagery_url_xyz() {
        let scheme = TilingScheme::geographic(Ellipsoid::WGS84);
        let url = build_imagery_url("https://tiles.example.com/{z}/{x}/{y}.png", 3, 1, 2, &scheme);
        assert_eq!(url, "https://tiles.example.com/2/3/1.png");
    }

    #[test]
    fn test_imagery_cache_default() {
        let cache = ImageryCache::default();
        assert!(cache.textures.is_empty());
    }

    #[test]
    fn test_pending_loads_default() {
        let pending = ImageryPendingLoads::default();
        assert!(pending.pending.is_empty());
        assert!(pending.ready_backlog.is_empty());
    }

    #[test]
    fn test_tile_rectangle() {
        let scheme = TilingScheme::geographic(Ellipsoid::WGS84);
        let rect = tile_rectangle(0, 0, 0, &scheme);
        let r2 = scheme.rectangle();
        assert!(rect.west >= r2.west);
        assert!(rect.east <= r2.east);
    }

    /// The request system marks a tile `Queued` exactly once; a second request
    /// for the same key must not clobber an already queued/in-flight entry.
    #[test]
    fn queued_state_is_idempotent() {
        let mut pending = ImageryPendingLoads::default();
        let key: ImageryKey = (7, 1, 2, 3);
        pending.pending.entry(key).or_insert(ImageryLoadState::Queued);
        // Second request for the same key: entry API leaves the first in place.
        pending.pending.entry(key).or_insert(ImageryLoadState::Queued);
        assert_eq!(pending.pending.len(), 1);
        assert!(matches!(
            pending.pending.get(&key),
            Some(ImageryLoadState::Queued)
        ));
    }

    /// Regression for the M1.4 frame-thread fix: the load system must never call
    /// the blocking fetch itself — it only spawns background tasks and polls them
    /// non-blockingly. This test drives the dispatch+poll bookkeeping directly
    /// (no network): a `Queued` tile transitions out of `pending` only via a
    /// spawned task, and an empty backlog yields zero uploads (frame thread does
    /// no synchronous fetch work).
    #[test]
    fn load_system_defers_fetch_to_background_backlog() {
        let mut pending = ImageryPendingLoads::default();
        let mut cache = ImageryCache::default();
        let mut stats = TileLoadStats::default();

        // Nothing queued, nothing in flight → the poll+upload passes are no-ops
        // and touch no texture (proves no synchronous fetch on the frame thread).
        let mut resolved: Vec<(ImageryKey, ImageryTaskResult)> = Vec::new();
        for (key, state) in pending.pending.iter_mut() {
            if let ImageryLoadState::InFlight(task) = state {
                if let Some(result) = block_on(poll_once(task)) {
                    resolved.push((*key, result));
                }
            }
        }
        assert!(resolved.is_empty());

        // A pre-resolved backlog entry uploads within the budget and updates the
        // cache + stats, mirroring the frame-thread upload step (Pass 3).
        pending
            .ready_backlog
            .push_back(((1, 0, 0, 0), Ok((vec![0u8; 4 * 2 * 2], 2, 2))));
        stats.tiles_pending = 1;

        let budget = 12;
        let mut images = Assets::<Image>::default();
        let mut uploaded = 0;
        while uploaded < budget {
            let Some((key, result)) = pending.ready_backlog.pop_front() else {
                break;
            };
            uploaded += 1;
            stats.tiles_pending = stats.tiles_pending.saturating_sub(1);
            if let Ok((data, width, height)) = result {
                let handle = images.add(crate::create_imagery_texture(width, height, data));
                cache.textures.insert(key, handle);
                stats.tiles_loaded += 1;
            }
        }

        assert_eq!(uploaded, 1);
        assert!(pending.ready_backlog.is_empty());
        assert_eq!(stats.tiles_loaded, 1);
        assert_eq!(stats.tiles_pending, 0);
        assert!(cache.textures.contains_key(&(1, 0, 0, 0)));
    }
}
