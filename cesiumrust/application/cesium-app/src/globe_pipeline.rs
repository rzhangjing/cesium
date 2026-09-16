//! Globe tile pipeline internals — extracted from the dynamic_globe golden
//! path (M1.5). Contains the budgeted process_pipeline body, download worker
//! (with offline wiring), eviction, mesh builds, and coverage repair.
//!
//! All logic is **byte-identical** to the original monolith; only the module
//! boundary changed. The three `evict_gpu_cache` invariants are preserved:
//! - BASE_LAYER (z ≤ 3) permanent exemption (`DefaultBudget::BASE_LAYER_ZOOM`)
//! - Live-entity push_back deferral (花屏防護核心)
//! - Termination: `MAX_TILE_ENTITIES(1800) << MAX_GPU_CACHE_ENTRIES(3000)`
//!
//! ## Offline wiring (M3 gate L127)
//! `download_worker` checks `feature_flags::offline_imagery_root()`:
//! - Set → reads `{root}/{z}/{x}/{y}.png` from disk (no network)
//! - `STRICT_OFFLINE=1` + https URL → synchronous panic (no fallback)

// frozen legacy golden-path style debt; local allow to satisfy strict CI clippy gate
#![allow(clippy::type_complexity, clippy::unnecessary_map_or, clippy::too_many_arguments)]

use bevy::prelude::*;
use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::base_sphere::{
    self, BaseSphereComposite, BaseSphereMarker, COMPOSITE_SIZE, COMPOSITE_TILE,
};
use crate::dynamic_globe::{MeshPipeline, TextureReceiver, TileManager};
use crate::feature_flags;
use crate::globe_lod::{self, TileKey, BASE_LAYER_ZOOM};
use crate::globe_textures::{build_mip_chain, is_placeholder_tile, make_image};
use crate::perf_counters::PerfCounters;
use crate::tile_mesh::{create_tile_mesh, create_tile_mesh_uv, render_scale, GlobeTile};
use cesium_bevy_render::pipeline::{fetch_gated, pipeline_gate_enabled};
use cesium_bevy_render::CesiumGlobe;
use cesium_pipeline::DefaultBudget;
use cesium_ports_driven::BudgetPolicy;

// ── Payload type ─────────────────────────────────────────────────────────

/// Download result from the worker pool (replaces the legacy TileDownloadResult).
/// Specialized ImageryPayload: RGBA + mip chain + staleness flags.
pub struct TileDownloadResult {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub rgba_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub placeholder: bool,
    pub aborted: bool,
    pub failed: bool,
}

// ── Enqueue tiles ────────────────────────────────────────────────────────

/// Queue mesh builds + downloads for tiles that need them, highest screen
/// footprint first. Original: `dynamic_globe.rs:371-459`.
pub fn enqueue_tiles(
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
            // Resolution upgrade: own_full_res contract (原 L837-841).
            if !mgr.no_data.contains(&key)
                && !mgr.reupload.contains(&key)
                && (prio > 192.0 || mgr.upsampled.contains(&key))
                && mgr.gpu_tex_size.get(&key).map_or(true, |&sz| sz < 256)
                && mgr.retry_after.get(&key).map_or(true, |t| std::time::Instant::now() >= *t)
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
                mesh_jobs.push((key, globe_lod::compute_segments(key.2), None));
            }
            if !cache.contains_key(&key)
                && !mgr.gpu_textures.contains_key(&key)
                && !mgr.no_data.contains(&key)
                && !mgr.in_flight.contains(&key)
                && mgr.retry_after.get(&key).map_or(true, |t| std::time::Instant::now() >= *t)
            {
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
    let downloads: Vec<(TileKey, bool)> = downloads.into_iter().map(|(k, d, _)| (k, d)).collect();

    { let mut w = tex_rx.wanted.lock().unwrap(); w.extend(mgr.queued.iter().copied()); }
    if !mesh_jobs.is_empty() { start_mesh_builds(mesh_pipe, mesh_jobs); }
    if !downloads.is_empty() { start_downloads(tex_rx, &downloads); }
}

// ── Main pipeline run (process_pipeline body) ────────────────────────────

/// The budgeted asset pipeline. Original: `dynamic_globe.rs:662-1429`.
#[allow(clippy::too_many_arguments)]
pub fn run(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    mgr: &mut TileManager,
    mesh_pipe: &mut MeshPipeline,
    tex_rx: &TextureReceiver,
    perf: &mut PerfCounters,
) {
    if !mgr.initialized { return; }
    perf.begin_frame();
    let mut evict_total_delta: u32 = 0;
    let mut evict_deferred_delta: u32 = 0;
    let mut stale_skips_delta: u32 = 0;
    let view_changed = mgr.view_changed_this_frame;
    mgr.view_changed_this_frame = false;

    // 1) Collect finished background meshes into the backlog.
    let drained: Vec<(TileKey, Mesh, bool)> = {
        let rx = mesh_pipe.rx.lock().unwrap();
        let mut v = Vec::new();
        while let Ok(item) = rx.try_recv() { v.push(item); }
        v
    };
    for item in drained { mesh_pipe.backlog.push_back(item); }

    // 2) Upload backlog meshes to the GPU within budget.
    let mut mesh_uploads = 0;
    while mesh_uploads < DefaultBudget.max_mesh_uploads_per_frame() {
        let Some((key, mesh, is_fb)) = mesh_pipe.backlog.pop_front() else { break };
        if (is_fb && mgr.gpu_fb_meshes.contains_key(&key))
            || (!is_fb && mgr.gpu_meshes.contains_key(&key)) { continue; }
        if !mgr.visible_set.contains(&key) && !mgr.load_set.contains(&key)
            && !mgr.queued.contains(&key) { continue; }
        if is_fb {
            mgr.pending_fb_builds.remove(&key);
            mgr.gpu_fb_meshes.insert(key, meshes.add(mesh));
        } else {
            mgr.gpu_meshes.insert(key, meshes.add(mesh));
        }
        let (e, d) = evict_gpu_cache(mgr);
        evict_total_delta += e; evict_deferred_delta += d;
        mesh_uploads += 1;
    }

    // 3) Spawn entities whose mesh handle is ready, within budget.
    let scale = render_scale();
    let pending: Vec<TileKey> = mgr.spawn_queue.drain(..).collect();
    let mut still_queued: Vec<TileKey> = Vec::new();
    let mut spawns = 0;
    let max_spawns = DefaultBudget::MAX_SPAWNS_PER_FRAME;

    let cache = tex_rx.cache.lock().unwrap();
    for key in pending {
        let already = mgr.tile_entities.contains_key(&key);
        if already || !mgr.queued.remove(&key) { continue; }

        let nodata = mgr.no_data.contains(&key);
        let mesh_ready = if nodata {
            if mgr.solid_tiles.contains(&key) { mgr.gpu_meshes.contains_key(&key) }
            else { mgr.gpu_fb_meshes.contains_key(&key) }
        } else {
            mgr.gpu_meshes.contains_key(&key) || mgr.gpu_fb_meshes.contains_key(&key)
        };
        if !mesh_ready || spawns >= max_spawns {
            still_queued.push(key); mgr.queued.insert(key); continue;
        }
        if !mgr.visible_set.contains(&key) && !mgr.load_set.contains(&key)
            && !mgr.partition_set.contains(&key) { continue; }

        if nodata {
            if mgr.solid_tiles.contains(&key) && has_live_ancestor(mgr, &key) { continue; }
            let mesh_handle = if mgr.solid_tiles.contains(&key) {
                mgr.gpu_meshes[&key].clone()
            } else { mgr.gpu_fb_meshes[&key].clone() };
            let material = if let Some(mat) = mgr.gpu_materials.get(&key) { mat.clone() }
            else if let Some(tex) = mgr.effective_tex.get(&key).cloned() {
                let mat = materials.add(StandardMaterial {
                    base_color: Color::WHITE, base_color_texture: Some(tex),
                    perceptual_roughness: 0.9, cull_mode: None, ..default()
                });
                mgr.gpu_materials.insert(key, mat.clone()); mat
            } else {
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.01, 0.05, 0.10),
                    perceptual_roughness: 1.0, cull_mode: None, ..default()
                });
                mgr.gpu_materials.insert(key, mat.clone()); mat
            };
            let entity = commands.spawn((CesiumGlobe, GlobeTile { x: key.0, y: key.1, z: key.2 },
                Mesh3d(mesh_handle), MeshMaterial3d(material),
                Transform::from_scale(Vec3::splat(scale)))).id();
            mgr.tile_entities.insert(key, entity);
            mgr.spawn_order.push_back(key);
            mgr.textured_tiles.insert(key);
            spawns += 1; continue;
        }

        // Real imagery path — own_full_res contract (原 L837-841, L1163):
        // A tile only displays its OWN texture once full resolution (≥256 px).
        let cached_tex = mgr.gpu_textures.get(&key).cloned().or_else(|| {
            cache.get(&key).map(|c| make_image(images, c.rgba_data.clone(), c.width, c.height, c.mip_levels))
        });
        let own_width = mgr.gpu_tex_size.get(&key).copied()
            .or_else(|| cache.get(&key).map(|c| c.width));
        let own_full_res = cached_tex.is_some()
            && (key.2 <= BASE_LAYER_ZOOM || own_width.map_or(false, |w| w >= 256));

        if !own_full_res {
            if key.2 <= BASE_LAYER_ZOOM {
                // Base root without imagery: fall through to solid spawn.
            } else if mgr.upsampled.contains(&key) {
                still_queued.push(key); mgr.queued.insert(key); continue;
            } else {
                // Inherit nearest textured ancestor (UV-remap).
                let (mut ax, mut ay, mut az) = key;
                let mut ancestor: Option<(TileKey, Handle<Image>)> = None;
                while az > 0 {
                    ax >>= 1; ay >>= 1; az -= 1;
                    if let Some(t) = mgr.effective_tex.get(&(ax, ay, az)) {
                        ancestor = Some(((ax, ay, az), t.clone())); break;
                    }
                }
                let Some((anc, tex)) = ancestor else {
                    still_queued.push(key); mgr.queued.insert(key); continue;
                };
                let dz = key.2 - anc.2;
                let side = (1u32 << dz) as f32;
                let rx = (key.0 % (1u32 << dz)) as f32;
                let ry = (key.1 % (1u32 << dz)) as f32;
                let [au0, av0, au1, av1] = mgr.effective_uv.get(&anc).copied().unwrap_or([0.,0.,1.,1.]);
                let fu = (au1 - au0) / side; let fv = (av1 - av0) / side;
                let uv = [au0 + rx*fu, av0 + ry*fv, au0 + (rx+1.)*fu, av0 + (ry+1.)*fv];
                let mesh_handle = if let Some(h) = mgr.gpu_fb_meshes.get(&key) { h.clone() }
                else {
                    if !mgr.pending_fb_builds.contains(&key) {
                        mgr.pending_fb_builds.insert(key);
                        start_mesh_builds(mesh_pipe, vec![(key, globe_lod::compute_segments(key.2), Some(uv))]);
                    }
                    still_queued.push(key); mgr.queued.insert(key); continue;
                };
                let material = if let Some(mat) = mgr.gpu_materials.get(&key) { mat.clone() }
                else {
                    let mat = materials.add(StandardMaterial {
                        base_color: Color::WHITE, base_color_texture: Some(tex.clone()),
                        perceptual_roughness: 0.9, cull_mode: None, ..default()
                    });
                    mgr.gpu_materials.insert(key, mat.clone()); mat
                };
                mgr.effective_tex.insert(key, tex);
                mgr.effective_uv.insert(key, uv);
                let entity = commands.spawn((CesiumGlobe, GlobeTile { x: key.0, y: key.1, z: key.2 },
                    Mesh3d(mesh_handle), MeshMaterial3d(material),
                    Transform::from_scale(Vec3::splat(scale)))).id();
                mgr.tile_entities.insert(key, entity);
                mgr.spawn_order.push_back(key);
                mgr.textured_tiles.insert(key);
                mgr.upsampled.insert(key);
                spawns += 1; continue;
            }
        }

        // Own full-res path (or base root).
        let Some(mesh_handle) = mgr.gpu_meshes.get(&key).cloned() else {
            still_queued.push(key); mgr.queued.insert(key); continue;
        };
        if let Some(tex) = &cached_tex {
            mgr.effective_tex.insert(key, tex.clone());
            mgr.effective_uv.insert(key, [0.0, 0.0, 1.0, 1.0]);
        }
        let material = if let Some(mat) = mgr.gpu_materials.get(&key) { mat.clone() }
        else if let Some(tex) = cached_tex.clone() {
            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE, base_color_texture: Some(tex),
                perceptual_roughness: 0.9, cull_mode: None, ..default()
            });
            mgr.gpu_materials.insert(key, mat.clone()); mat
        } else {
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.01, 0.05, 0.10),
                perceptual_roughness: 1.0, cull_mode: None, ..default()
            });
            mgr.gpu_materials.insert(key, mat.clone()); mat
        };
        if let Some(tex) = cached_tex {
            if !mgr.gpu_textures.contains_key(&key) { mgr.gpu_tex_order.push_back(key); }
            mgr.gpu_textures.insert(key, tex);
            let (e, d) = evict_gpu_cache(mgr);
            evict_total_delta += e; evict_deferred_delta += d;
        }
        let entity = commands.spawn((CesiumGlobe, GlobeTile { x: key.0, y: key.1, z: key.2 },
            Mesh3d(mesh_handle), MeshMaterial3d(material),
            Transform::from_scale(Vec3::splat(scale)))).id();
        mgr.tile_entities.insert(key, entity);
        mgr.spawn_order.push_back(key);
        mgr.textured_tiles.insert(key);
        spawns += 1;
    }
    drop(cache);
    mgr.spawn_queue = still_queued.into();

    // 4) Apply downloaded textures within budget.
    let mut tex_uploads = 0;
    while tex_uploads < DefaultBudget::MAX_TEXTURE_UPLOADS_PER_FRAME {
        let Ok(result) = tex_rx.rx.lock().unwrap().try_recv() else { break };
        let key = (result.x, result.y, result.z);
        mgr.in_flight.remove(&key);
        if result.aborted {
            mgr.reupload.remove(&key); stale_skips_delta += 1; continue;
        }
        if result.failed {
            mgr.reupload.remove(&key);
            mgr.retry_after.insert(key, std::time::Instant::now() + std::time::Duration::from_secs(10));
            continue;
        }
        if result.placeholder {
            mgr.reupload.remove(&key);
            if mgr.gpu_textures.contains_key(&key) { tex_uploads += 1; continue; }
            if mgr.no_data.insert(key) {
                let (mut ax, mut ay, mut az) = key;
                let mut ancestor: Option<(TileKey, Handle<Image>)> = None;
                while az > 0 {
                    ax >>= 1; ay >>= 1; az -= 1;
                    if let Some(t) = mgr.effective_tex.get(&(ax, ay, az)) {
                        ancestor = Some(((ax, ay, az), t.clone())); break;
                    }
                }
                if let Some((anc, tex)) = ancestor {
                    let dz = key.2 - anc.2; let side = (1u32 << dz) as f32;
                    let rx = (key.0 % (1u32 << dz)) as f32; let ry = (key.1 % (1u32 << dz)) as f32;
                    let [au0, av0, au1, av1] = mgr.effective_uv.get(&anc).copied().unwrap_or([0.,0.,1.,1.]);
                    let fu = (au1-au0)/side; let fv = (av1-av0)/side;
                    let uv = [au0+rx*fu, av0+ry*fv, au0+(rx+1.)*fu, av0+(ry+1.)*fv];
                    mgr.effective_tex.insert(key, tex);
                    mgr.effective_uv.insert(key, uv);
                    if !mgr.gpu_fb_meshes.contains_key(&key) {
                        start_mesh_builds(mesh_pipe, vec![(key, globe_lod::compute_segments(key.2), Some(uv))]);
                    }
                } else {
                    let mut ix = key.0; let mut iy = key.1; let mut iz = key.2;
                    let mut inherited = false;
                    while iz > 0 {
                        ix >>= 1; iy >>= 1; iz -= 1;
                        if mgr.tile_entities.contains_key(&(ix, iy, iz)) {
                            if let Some(mat) = mgr.gpu_materials.get(&(ix,iy,iz)).cloned() {
                                mgr.gpu_materials.insert(key, mat); inherited = true;
                            }
                            break;
                        }
                    }
                    if !inherited { mgr.solid_tiles.insert(key); }
                }
            }
            if !mgr.tile_entities.contains_key(&key) && !mgr.queued.contains(&key) {
                mgr.spawn_queue.push_back(key); mgr.queued.insert(key);
            }
            tex_uploads += 1; continue;
        }

        // Successful texture download.
        let tex_handle = if matches!(
            (mgr.gpu_textures.get(&key), mgr.gpu_tex_size.get(&key)),
            (Some(_), Some(&sz)) if sz >= result.width
        ) { mgr.gpu_textures[&key].clone() }
        else {
            let h = make_image(images, result.rgba_data, result.width, result.height, result.mip_levels);
            if !mgr.gpu_textures.contains_key(&key) { mgr.gpu_tex_order.push_back(key); }
            mgr.gpu_textures.insert(key, h.clone());
            mgr.gpu_tex_size.insert(key, result.width);
            let (e, d) = evict_gpu_cache(mgr);
            evict_total_delta += e; evict_deferred_delta += d;
            h
        };
        mgr.reupload.remove(&key);
        // own_full_res contract (原 L1163): sub-256 filler must not repaint
        // a tile currently upsampling an ancestor.
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
        if mgr.tile_entities.contains_key(&key) { mgr.textured_tiles.insert(key); }
        tex_uploads += 1;
    }

    // Sharpen pass: upsampled tile with own full-res + real mesh swaps atomically.
    let sharpen: Vec<TileKey> = mgr.upsampled.iter().filter(|k| {
        mgr.tile_entities.contains_key(*k) && mgr.gpu_meshes.contains_key(*k)
            && matches!(mgr.gpu_tex_size.get(*k), Some(&s) if s >= 256)
    }).copied().collect();
    for key in sharpen {
        let ent = mgr.tile_entities[&key];
        if let (Some(mesh), Some(mat)) = (mgr.gpu_meshes.get(&key).cloned(), mgr.gpu_materials.get(&key).cloned()) {
            commands.entity(ent).insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            mgr.upsampled.remove(&key);
        }
    }

    // 5) Budgeted cleanup (MAX_TILE_ENTITIES cap, fine-level-first LRU).
    let mut removed = 0;
    let max_entities = DefaultBudget::MAX_TILE_ENTITIES;
    let max_despawns = DefaultBudget::MAX_DESPAWNS_PER_FRAME;
    if mgr.tile_entities.len() > max_entities {
        let prot = protected_ancestors(mgr);
        while mgr.tile_entities.len() > max_entities && removed < max_despawns {
            let len = mgr.hide_order.len();
            let Some(key) = mgr.hide_order.iter().enumerate()
                .filter(|(_, k)| mgr.tile_entities.contains_key(*k) && !prot.contains(*k)
                    && !mgr.visible_set.contains(*k) && !mgr.load_set.contains(*k)
                    && k.2 > BASE_LAYER_ZOOM)
                .max_by_key(|(idx, k)| (k.2, len - idx))
                .map(|(_, k)| *k)
            else { break };
            mgr.despawn_tile(&key, commands); removed += 1;
        }
        if mgr.tile_entities.len() > max_entities {
            let mut extras: Vec<(usize, TileKey)> = {
                let pos = |k: &TileKey| mgr.spawn_order.iter().position(|o| o == k).unwrap_or(usize::MAX);
                let mut v: Vec<(usize, TileKey)> = mgr.tile_entities.keys().copied()
                    .filter(|k| k.2 > BASE_LAYER_ZOOM && !mgr.visible_set.contains(k)
                        && !mgr.load_set.contains(k) && has_live_ancestor(mgr, k))
                    .map(|k| (pos(&k), k)).collect();
                v.sort(); v
            };
            for (_, key) in extras.drain(..) {
                if mgr.tile_entities.len() <= max_entities || removed >= max_despawns * 2 { break; }
                mgr.despawn_tile(&key, commands); removed += 1;
            }
        }
    }

    // Coverage repair (stable frames only).
    if !view_changed {
        let missing: Vec<TileKey> = mgr.visible_set.iter().chain(mgr.load_set.iter())
            .filter(|k| !mgr.tile_entities.contains_key(k) && !mgr.queued.contains(k))
            .copied().take(32).collect();
        let starved: Vec<TileKey> = {
            let cache = tex_rx.cache.lock().unwrap();
            mgr.visible_set.iter().filter(|k| {
                mgr.queued.contains(k) && !mgr.tile_entities.contains_key(k)
                    && !mgr.in_flight.contains(k) && !mgr.no_data.contains(k)
                    && !mgr.gpu_textures.contains_key(k) && !cache.contains_key(k)
            }).copied().collect()
        };
        if !missing.is_empty() || !starved.is_empty() {
            let mut repair_mesh: Vec<(TileKey, u32, Option<[f32; 4]>)> = Vec::new();
            let mut repair_dl: Vec<(TileKey, bool)> = Vec::new();
            { let cache = tex_rx.cache.lock().unwrap();
              for k in missing.iter().chain(starved.iter()) {
                if !mgr.gpu_meshes.contains_key(k) && !mgr.gpu_fb_meshes.contains_key(k) {
                    repair_mesh.push((*k, globe_lod::compute_segments(k.2), None));
                }
                if !cache.contains_key(k) && !mgr.gpu_textures.contains_key(k)
                    && !mgr.no_data.contains(k) && !mgr.in_flight.contains(k)
                    && mgr.retry_after.get(k).map_or(true, |t| std::time::Instant::now() >= *t) {
                    repair_dl.push((*k, false)); mgr.in_flight.insert(*k);
                }
              }
            }
            if !repair_mesh.is_empty() { start_mesh_builds(mesh_pipe, repair_mesh); }
            if !repair_dl.is_empty() { start_downloads(tex_rx, &repair_dl); }
            for k in missing { mgr.spawn_queue.push_back(k); mgr.queued.insert(k); }
        }
    }

    // Wanted refresh.
    { let mut w = tex_rx.wanted.lock().unwrap(); w.clear();
      w.extend(mgr.queued.iter().copied()); w.extend(mgr.visible_set.iter().copied());
      w.extend(mgr.load_set.iter().copied()); w.extend(mgr.tile_entities.keys().copied());
    }

    // PerfCounters write-back (bidirectional with legacy).
    perf.in_flight = mgr.in_flight.len() as u32;
    perf.gpu_tex_order = mgr.gpu_tex_order.len() as u32;
    perf.tile_entities = mgr.tile_entities.len() as u32;
    perf.spawn_queue = mgr.spawn_queue.len() as u32;
    perf.backlog = mesh_pipe.backlog.len() as u32;
    perf.retry_after = mgr.retry_after.len() as u32;
    perf.load_set = mgr.load_set.len() as u32;
    perf.frame_mesh = mesh_uploads as u32;
    perf.frame_tex = tex_uploads as u32;
    perf.frame_spawn = spawns as u32;
    perf.frame_despawn = removed as u32;
    perf.stale_skips = perf.stale_skips.wrapping_add(stale_skips_delta);
    perf.evict_total = perf.evict_total.wrapping_add(evict_total_delta);
    perf.evict_deferred = perf.evict_deferred.wrapping_add(evict_deferred_delta);
}

// ── GPU cache eviction (three invariants) ────────────────────────────────

/// FIFO-evict the oldest cached GPU handles once over the cap.
///
/// Three invariants (delegated from `cesium-pipeline` GpuCache design, #25):
/// 1. **BASE_LAYER exempt** — z ≤ BASE_LAYER_ZOOM never evicted (permanent fallback)
/// 2. **Live-entity push_back deferral** — 花屏防護: freeing Assets data while a
///    spawned entity references it corrupts the mesh allocator (horizontal stripes)
/// 3. **Termination** — MAX_TILE_ENTITIES(1800) << MAX_GPU_CACHE_ENTRIES(3000)
///    guarantees evictable (dead) entries always exist
///
/// Original: `dynamic_globe.rs:1476-1502` (逐字節保留).
pub fn evict_gpu_cache(mgr: &mut TileManager) -> (u32, u32) {
    let mut evicted: u32 = 0;
    let mut deferred: u32 = 0;
    while mgr.gpu_tex_order.len() > DefaultBudget::MAX_GPU_CACHE_ENTRIES {
        let Some(old) = mgr.gpu_tex_order.pop_front() else {
            break;
        };
        if old.2 <= BASE_LAYER_ZOOM {
            // Invariant 1: Permanent fallback layer — never evict.
            continue;
        }
        if mgr.tile_entities.contains_key(&old) {
            // Invariant 2: Still rendered — defer eviction (花屏防護).
            mgr.gpu_tex_order.push_back(old);
            deferred += 1;
            continue;
        }
        mgr.gpu_textures.remove(&old);
        mgr.gpu_materials.remove(&old);
        mgr.gpu_meshes.remove(&old);
        mgr.gpu_fb_meshes.remove(&old);
        mgr.effective_tex.remove(&old);
        mgr.gpu_tex_size.remove(&old);
        evicted += 1;
    }
    (evicted, deferred)
}

// ── Display-set helpers ──────────────────────────────────────────────────

/// A partition leaf the display can safely hand over to: a live entity
/// showing real pixels (own full-res texture, or an intentional solid /
/// no-data inheritance) — never a stretched-ancestor upsample.
///
/// Original: `dynamic_globe.rs:1435-1446` (逐字節保留).
pub fn replacement_ready(mgr: &TileManager, key: &TileKey) -> bool {
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
/// region, so `key` can be safely skipped/evicted without opening a hole.
///
/// Original: `dynamic_globe.rs:1451-1462` (逐字節保留).
pub fn has_live_ancestor(mgr: &TileManager, key: &TileKey) -> bool {
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

/// Ancestor keys of every render/load-set tile that has no spawned entity
/// yet. Protection covers the ENTIRE ancestor chain of each pending leaf.
///
/// Original: `dynamic_globe.rs:631-650` (逐字節保留).
pub fn protected_ancestors(mgr: &TileManager) -> HashSet<TileKey> {
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

// ── Background mesh builds ───────────────────────────────────────────────

/// Build tile meshes on worker threads; results flow back through the
/// pipeline channel and are uploaded to the GPU within the frame budget.
///
/// Original: `dynamic_globe.rs:2076-2108` (逐字節保留).
pub fn start_mesh_builds(pipe: &mut MeshPipeline, jobs: Vec<(TileKey, u32, Option<[f32; 4]>)>) {
    let tx = pipe.tx.clone();

    std::thread::spawn(move || {
        let chunks: Vec<Vec<(TileKey, u32, Option<[f32; 4]>)>> = {
            let mut c: Vec<Vec<(TileKey, u32, Option<[f32; 4]>)>> =
                (0..DefaultBudget::DOWNLOAD_THREADS).map(|_| Vec::new()).collect();
            for (i, job) in jobs.into_iter().enumerate() {
                c[i % DefaultBudget::DOWNLOAD_THREADS].push(job);
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

// ── Bing Maps downloads ──────────────────────────────────────────────────

/// Bing Maps quadkey encoding from tile coordinates.
/// Original: `dynamic_globe.rs:2112-2126` (逐字節保留).
pub fn tile_to_quadkey(x: u32, y: u32, level: u32) -> String {
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

/// Feed download jobs to the persistent worker pool. Non-blocking: jobs queue
/// up and workers skip stale ones (not in `wanted`) at dequeue time.
///
/// Original: `dynamic_globe.rs:2133-2137` (逐字節保留).
pub fn start_downloads(tex_rx: &TextureReceiver, tiles: &[(TileKey, bool)]) {
    for &(key, downscale) in tiles {
        let _ = tex_rx.job_tx.send((key, downscale));
    }
}

// ── Download worker (offline wiring: M3 gate L127) ───────────────────────

/// Persistent download-pool worker: owns one ureq agent for its whole life
/// (connection pool keeps tile-server connections warm across fetches) and
/// pulls jobs until the feed closes.
///
/// ## Offline wiring (M3 gate L127)
/// - `OFFLINE_IMAGERY_ROOT` set → reads `{root}/{z}/{x}/{y}.png` from disk
/// - `STRICT_OFFLINE=1` + any https URL → synchronous panic (no fallback)
/// - Neither set → online Bing via `fetch_gated` (shared ureq keep-alive pool)
///
/// Original: `dynamic_globe.rs:2143-2285` (逐字節保留, plus offline wiring).
pub fn download_worker(
    job_rx: Arc<Mutex<mpsc::Receiver<(TileKey, bool)>>>,
    tx: mpsc::Sender<TileDownloadResult>,
    wanted: Arc<Mutex<HashSet<TileKey>>>,
) {
    // Resolve offline config once at worker startup (env is process-global,
    // immutable after main() entry — no TOCTOU risk).
    let offline_root = feature_flags::offline_imagery_root();
    let strict = feature_flags::strict_offline();
    let use_pipeline = pipeline_gate_enabled();

    loop {
        let job = job_rx.lock().unwrap().recv();
        let Ok(((px, py, pz), downscale)) = job else {
            return;
        };
        // Wanted-staleness skip: fast pans make whole batches stale within a
        // few frames; skip fetches nobody will look at.
        if !wanted.lock().unwrap().contains(&(px, py, pz)) {
            let _ = tx.send(TileDownloadResult {
                x: px, y: py, z: pz,
                rgba_data: Vec::new(), width: 0, height: 0, mip_levels: 0,
                placeholder: false, aborted: true, failed: false,
            });
            continue;
        }

        // ── Offline path: read from disk ─────────────────────────────────
        if let Some(root) = &offline_root {
            let path = root.join(pz.to_string()).join(px.to_string())
                .join(format!("{}.png", py));
            match std::fs::read(&path) {
                Ok(data) => {
                    if let Ok(img) = image::load_from_memory(&data) {
                        let rgba = img.to_rgba8();
                        let (is_ph, ph_avg, ph_maxd) = is_placeholder_tile(&rgba);
                        if is_ph {
                            eprintln!(
                                "[nodata] z{pz} x{px} y{py} verdict=placeholder avg={ph_avg:.2} maxd={ph_maxd}"
                            );
                            let _ = tx.send(TileDownloadResult {
                                x: px, y: py, z: pz,
                                rgba_data: Vec::new(), width: 0, height: 0, mip_levels: 0,
                                placeholder: true, aborted: false, failed: false,
                            });
                            continue;
                        }
                        let (rgba, w, h) = if downscale && rgba.width() > 128 {
                            let small = image::DynamicImage::ImageRgba8(rgba).resize(
                                128, 128, image::imageops::FilterType::Triangle,
                            );
                            let r = small.to_rgba8();
                            let (w, h) = r.dimensions();
                            (r, w, h)
                        } else {
                            let (w, h) = rgba.dimensions();
                            (rgba, w, h)
                        };
                        let (chain, levels) = build_mip_chain(rgba.into_raw(), w, h);
                        let _ = tx.send(TileDownloadResult {
                            x: px, y: py, z: pz,
                            rgba_data: chain, width: w, height: h, mip_levels: levels,
                            placeholder: false, aborted: false, failed: false,
                        });
                        continue;
                    }
                    // Decode failed — fall through to failed result.
                }
                Err(_) => {
                    // File not found — treat as no-data (placeholder).
                    let _ = tx.send(TileDownloadResult {
                        x: px, y: py, z: pz,
                        rgba_data: Vec::new(), width: 0, height: 0, mip_levels: 0,
                        placeholder: true, aborted: false, failed: false,
                    });
                    continue;
                }
            }
            // If we get here, decode failed.
            let _ = tx.send(TileDownloadResult {
                x: px, y: py, z: pz,
                rgba_data: Vec::new(), width: 0, height: 0, mip_levels: 0,
                placeholder: false, aborted: false, failed: true,
            });
            continue;
        }

        // ── Online path: Bing Maps via fetch_gated ───────────────────────
        let qk = tile_to_quadkey(px, py, pz);
        let sub = (px + py) % 8;
        let url = format!(
            "https://ecn.t{}.tiles.virtualearth.net/tiles/a{}.jpeg?g=14393",
            sub, qk
        );

        // STRICT_OFFLINE: any https URL is a hard error (M3 gate L127).
        if strict && url.starts_with("https") {
            panic!(
                "STRICT_OFFLINE=1: network fetch attempted for {} — \
                 set OFFLINE_IMAGERY_ROOT to provide tiles from disk",
                url
            );
        }

        // Retry with backoff: tile servers throttle bursty clients.
        let mut delivered = false;
        for attempt in 0..3u32 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(250u64 << attempt));
            }
            let fetched = match fetch_gated(&url, use_pipeline) {
                Ok(data) => image::load_from_memory(&data).ok(),
                Err(_) => None,
            };
            if let Some(img) = fetched {
                let rgba = img.to_rgba8();
                let (is_ph, ph_avg, ph_maxd) = is_placeholder_tile(&rgba);
                if is_ph {
                    eprintln!(
                        "[nodata] z{pz} x{px} y{py} verdict=placeholder avg={ph_avg:.2} maxd={ph_maxd}"
                    );
                    let _ = tx.send(TileDownloadResult {
                        x: px, y: py, z: pz,
                        rgba_data: Vec::new(), width: 0, height: 0, mip_levels: 0,
                        placeholder: true, aborted: false, failed: false,
                    });
                    delivered = true;
                    break;
                }
                let (rgba, w, h) = if downscale && rgba.width() > 128 {
                    let small = image::DynamicImage::ImageRgba8(rgba).resize(
                        128, 128, image::imageops::FilterType::Triangle,
                    );
                    let r = small.to_rgba8();
                    let (w, h) = r.dimensions();
                    (r, w, h)
                } else {
                    let (w, h) = rgba.dimensions();
                    (rgba, w, h)
                };
                let (chain, levels) = build_mip_chain(rgba.into_raw(), w, h);
                let _ = tx.send(TileDownloadResult {
                    x: px, y: py, z: pz,
                    rgba_data: chain, width: w, height: h, mip_levels: levels,
                    placeholder: false, aborted: false, failed: false,
                });
                delivered = true;
                break;
            }
        }
        if !delivered {
            eprintln!("[nodata] z{pz} x{px} y{py} verdict=retries-exhausted");
            let _ = tx.send(TileDownloadResult {
                x: px, y: py, z: pz,
                rgba_data: Vec::new(), width: 0, height: 0, mip_levels: 0,
                placeholder: false, aborted: false, failed: true,
            });
        }
    }
}

// ── Base-sphere composite ────────────────────────────────────────────────

/// Once every base-layer (z=3) tile has either a texture or a no-data
/// verdict, bake them into one 1024×1024 Mercator composite and drape it
/// over the base sphere.
///
/// Original: `dynamic_globe.rs:1920-1991` (逐字節保留).
pub fn run_base_sphere_composite(
    state: &mut BaseSphereComposite,
    mgr: &TileManager,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    sphere: &Query<&MeshMaterial3d<StandardMaterial>, With<BaseSphereMarker>>,
) {
    if state.done {
        return;
    }
    if let Some(rx) = &state.rx {
        let Ok((chain, levels)) = rx.lock().unwrap().try_recv() else {
            return;
        };
        let handle =
            base_sphere::make_clamped_image(images, chain, COMPOSITE_SIZE, COMPOSITE_SIZE, levels);
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

    // Collect 128-px blocks (box-downsampled full-res tiles).
    let mut blocks: Vec<(u32, u32, Vec<u8>)> = Vec::with_capacity(keys.len());
    for k in keys {
        let block = mgr
            .gpu_textures
            .get(&k)
            .and_then(|h| images.get(h))
            .map(|img| {
                let w = img.texture_descriptor.size.width;
                let h = img.texture_descriptor.size.height;
                base_sphere::box_downsample(&img.data[..(w * h * 4) as usize], w, COMPOSITE_TILE)
            })
            .unwrap_or_else(base_sphere::ocean_block);
        blocks.push((k.0, k.1, block));
    }

    state.rx = Some(base_sphere::spawn_composite_bake(blocks));
}
