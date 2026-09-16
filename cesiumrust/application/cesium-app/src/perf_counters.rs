//! PerfCounters — Bevy Resource exposing dynamic_globe TileManager metrics.
//!
//! ## Design
//!
//! `PerfCounters` is a **pure observation surface**: dynamic_globe writes
//! into it every frame, and the M0.5 perf-trace plugin reads from it to
//! produce CSV rows. It holds **no control logic** — no budget, no
//! eviction, no download decision lives here. This separation is what
//! makes the M0.4 injection pixel-neutral: the golden path in
//! `dynamic_globe.rs` keeps its exact semantics, and the only added work
//! is a handful of integer stores per frame.
//!
//! ## Counter taxonomy
//!
//! The 11 categories requested by the plan map onto three shapes:
//!
//! 1. **Gauges** (instantaneous sizes, read straight from TileManager
//!    collections): `in_flight`, `gpu_tex_order`, `tile_entities`,
//!    `spawn_queue`, `backlog`, `retry_after`.
//! 2. **Per-frame deltas** (reset to 0 at the start of each frame, then
//!    incremented by the pipeline as it does work): `frame_mesh`,
//!    `frame_tex`, `frame_spawn`, `frame_despawn`.
//! 3. **Cumulative totals** (monotonic since process start): `stale_skips`
//!    (aborted downloads because the tile left the wanted set),
//!    `evict_total` (GPU cache entries actually removed), `evict_deferred`
//!    (eviction candidates skipped because the tile was still rendered).
//!
//! ## Thread-safety
//!
//! All fields are plain integers behind Bevy's `ResMut` access, so the
//! frame thread is the only writer. The perf-trace plugin reads them on
//! the same thread (in an `Update` system) and hands a snapshot to a
//! dedicated writer thread via a channel — no lock, no atomics, no
//! contention with the render loop.

use bevy::prelude::*;

/// Per-frame + cumulative counters for the dynamic_globe tile pipeline.
///
/// Default is all-zero, which is the correct state for frame 0 (nothing
/// has been uploaded / spawned / evicted yet).
#[derive(Resource, Debug, Clone, Default)]
pub struct PerfCounters {
    // ── Gauges (instantaneous collection sizes) ────────────────────────

    /// Downloads currently in flight (`TileManager::in_flight.len()`).
    pub in_flight: u32,
    /// GPU texture cache entries (`TileManager::gpu_tex_order.len()`).
    /// This is the FIFO eviction order, so it equals the live texture count.
    pub gpu_tex_order: u32,
    /// Spawned tile entities (`TileManager::tile_entities.len()`).
    pub tile_entities: u32,
    /// Tiles waiting for mesh build + spawn (`TileManager::spawn_queue.len()`).
    pub spawn_queue: u32,
    /// Finished meshes waiting for GPU upload (`MeshPipeline::backlog.len()`).
    pub backlog: u32,
    /// Tiles in download cooldown after a transient failure
    /// (`TileManager::retry_after.len()`).
    pub retry_after: u32,
    /// Tiles in the load set — still loading but NOT in the render partition
    /// (KICK-blocked children). `TileManager::load_set.len()`.
    pub load_set: u32,

    // ── Per-frame deltas (reset each frame, then incremented) ──────────

    /// Mesh uploads performed this frame (capped by `MAX_MESH_UPLOADS_PER_FRAME`).
    pub frame_mesh: u32,
    /// Texture uploads applied this frame (capped by `MAX_TEXTURE_UPLOADS_PER_FRAME`).
    pub frame_tex: u32,
    /// Entities spawned this frame (capped by `MAX_SPAWNS_PER_FRAME`).
    pub frame_spawn: u32,
    /// Entities despawned this frame (capped by `MAX_DESPAWNS_PER_FRAME`).
    pub frame_despawn: u32,

    // ── Cumulative totals (monotonic since process start) ──────────────

    /// Downloads aborted because the tile left the wanted set mid-flight
    /// (the "stale skip" path in `download_worker`).
    pub stale_skips: u32,
    /// GPU cache entries actually evicted (removed from `gpu_textures` etc.).
    pub evict_total: u32,
    /// Eviction candidates deferred because the tile was still rendered
    /// (pushed back onto `gpu_tex_order` instead of removed).
    pub evict_deferred: u32,

    // ── Frame bookkeeping (used by the trace writer) ───────────────────

    /// Monotonic frame index, incremented once per frame by the trace
    /// system. Starts at 0; the first CSV row is frame 1.
    pub frame_idx: u32,
    /// Wall-clock delta of the last frame in milliseconds (from Bevy's
    /// `Time::delta_secs_f64`). Written by the trace system, not by
    /// dynamic_globe.
    pub dt_ms: f64,
}

impl PerfCounters {
    /// Reset the four per-frame delta counters to zero. Called at the
    /// **start** of each frame (before the pipeline runs) so the deltas
    /// reflect only this frame's work.
    ///
    /// Gauges and cumulative totals are intentionally NOT reset here:
    /// gauges are re-read from TileManager every frame, and cumulative
    /// totals must stay monotonic.
    #[inline]
    pub fn begin_frame(&mut self) {
        self.frame_mesh = 0;
        self.frame_tex = 0;
        self.frame_spawn = 0;
        self.frame_despawn = 0;
    }

    /// Advance the frame index and record the wall-clock delta. Called
    /// **once per frame** by the trace system after the pipeline has run,
    /// so `frame_idx` in the CSV row matches the frame that produced the
    /// counters.
    #[inline]
    pub fn end_frame(&mut self, dt_ms: f64) {
        self.frame_idx = self.frame_idx.wrapping_add(1);
        self.dt_ms = dt_ms;
    }

    /// One-line human summary, mirroring the legacy `[stats]` printf so a
    /// reviewer can eyeball-consistency between the old log and the new
    /// counters. Used by the M0.4 acceptance check ("PerfCounters 与手工
    /// printf 校对一致").
    pub fn summary_line(&self) -> String {
        format!(
            "frame={} dt={:.2}ms ents={} vis_q={} backlog={} inflight={} retry={} load={} \
             f:mesh={} tex={} spawn={} despawn={} \
             cum:stale={} evict={} defer={} gpu_tex={}",
            self.frame_idx,
            self.dt_ms,
            self.tile_entities,
            self.spawn_queue,
            self.backlog,
            self.in_flight,
            self.retry_after,
            self.load_set,
            self.frame_mesh,
            self.frame_tex,
            self.frame_spawn,
            self.frame_despawn,
            self.stale_skips,
            self.evict_total,
            self.evict_deferred,
            self.gpu_tex_order,
        )
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_zero() {
        let c = PerfCounters::default();
        assert_eq!(c.in_flight, 0);
        assert_eq!(c.frame_idx, 0);
        assert_eq!(c.dt_ms, 0.0);
        assert_eq!(c.stale_skips, 0);
    }

    #[test]
    fn begin_frame_resets_only_deltas() {
        let mut c = PerfCounters {
            frame_mesh: 5,
            frame_tex: 6,
            frame_spawn: 7,
            frame_despawn: 8,
            in_flight: 10,
            tile_entities: 20,
            stale_skips: 30,
            evict_total: 40,
            ..Default::default()
        };
        c.begin_frame();
        assert_eq!(c.frame_mesh, 0);
        assert_eq!(c.frame_tex, 0);
        assert_eq!(c.frame_spawn, 0);
        assert_eq!(c.frame_despawn, 0);
        // Gauges + cumulative untouched.
        assert_eq!(c.in_flight, 10);
        assert_eq!(c.tile_entities, 20);
        assert_eq!(c.stale_skips, 30);
        assert_eq!(c.evict_total, 40);
    }

    #[test]
    fn end_frame_advances_index_and_records_dt() {
        let mut c = PerfCounters::default();
        c.end_frame(16.67);
        assert_eq!(c.frame_idx, 1);
        assert!((c.dt_ms - 16.67).abs() < 1e-9);
        c.end_frame(16.70);
        assert_eq!(c.frame_idx, 2);
        assert!((c.dt_ms - 16.70).abs() < 1e-9);
    }

    #[test]
    fn summary_line_contains_all_fields() {
        let mut c = PerfCounters {
            tile_entities: 123,
            in_flight: 4,
            ..Default::default()
        };
        c.end_frame(16.0);
        let s = c.summary_line();
        assert!(s.contains("frame=1"));
        assert!(s.contains("ents=123"));
        assert!(s.contains("inflight=4"));
    }
}
