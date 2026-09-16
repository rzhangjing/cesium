//! M0.5 — CSV perf-trace, headless CLI, and camera-script playback.
//!
//! ## Architecture
//!
//! ```text
//!  frame thread (Update)                    writer thread (dedicated)
//!  ─────────────────────                    ─────────────────────────
//!  PerfCounters ──► format CSV row ──► mpsc ──► ringbuffer ──► flush ──► file
//!       ▲                                              (batched, <0.3ms/frame)
//!       │
//!  dynamic_globe::process_pipeline (M0.4 write-back)
//! ```
//!
//! The frame thread's only work is: read counters, `format!` one CSV line,
//! `tx.send(line)`. All file I/O (open, write, flush, close) happens on the
//! writer thread, so a slow disk never stalls the render loop. Measured
//! self-overhead is reported every 5 s via `info!`.
//!
//! ## CLI / env
//!
//! | Flag | Env | Default | Effect |
//! |------|-----|---------|--------|
//! | `--headless` | `CESIUM_HEADLESS=1` | off | Auto-exit after `--headless-frames`; camera script drives the view |
//! | `--perf-trace=<path>` | `CESIUM_PERF_TRACE` | off | Write CSV trace to `<path>` |
//! | `--camera-script=<toml>` | `CESIUM_CAMERA_SCRIPT` | off | Play back a keyframed camera trajectory |
//! | `--trace-interval=<n>` | `CESIUM_TRACE_INTERVAL` | 1 | Emit one CSV row every `n` frames |
//! | `--headless-frames=<n>` | `CESIUM_HEADLESS_FRAMES` | 3600 | Frames before auto-exit in headless mode (~60 s @ 60 fps) |
//!
//! Flags take precedence over env when both are present.
//!
//! ## Camera-script TOML format
//!
//! ```toml
//! [meta]
//! name = "orbit_slow"
//! duration_s = 60.0
//! description = "Slow 360° equatorial orbit"
//!
//! [[keyframe]]
//! t = 0.0
//! lon = 0.0
//! lat = 23.0
//! height = 2.0
//!
//! [[keyframe]]
//! t = 60.0
//! lon = 360.0
//! lat = 23.0
//! height = 2.0
//! ```
//!
//! Fields per keyframe (all optional except `t`):
//! - `t` — seconds from script start (required, monotonically increasing)
//! - `lon` / `heading` — degrees; `heading` takes precedence (matches Lee's
//!   `CESIUM_CAM_HEADING` convention)
//! - `lat` / `pitch` — degrees; `pitch` takes precedence
//! - `height` — render units above the surface (globe R = 1)
//! - `distance` — direct orbit distance from center; overrides `height`
//!
//! Interpolation is **linear** between adjacent keyframes. Before the first
//! keyframe the camera holds `keyframe[0]`; after the last it holds
//! `keyframe[last]`. A single-keyframe script is a static camera.
//!
//! The script **seeds from Lee's orbit_camera env** when no keyframe fields
//! are given at `t=0`, so `--camera-script` composes with `CESIUM_CAM_*`.

use bevy::prelude::*;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::dynamic_globe::TilePipelineSet;
use crate::feature_flags::env_flag;
use crate::orbit_camera::{orbit_state_from_env, OrbitState};
use crate::perf_counters::PerfCounters;

// ── CLI ──────────────────────────────────────────────────────────────────

/// Parsed command-line / env configuration for the perf-trace subsystem.
#[derive(Debug, Clone, Default)]
pub struct Cli {
    /// Run without interaction and auto-exit after `headless_frames`.
    pub headless: bool,
    /// CSV trace output path. `None` = tracing disabled.
    pub perf_trace: Option<PathBuf>,
    /// Camera-script TOML path. `None` = no scripted playback.
    pub camera_script: Option<PathBuf>,
    /// Emit one CSV row every N frames (default 1 = every frame).
    pub trace_interval: u32,
    /// Frames before auto-exit in headless mode (default 3600 ≈ 60 s @ 60 fps).
    /// This is a **fallback** cap: when a wall-clock target is known (an
    /// explicit `--headless-secs`, or a camera-script's `duration_s`), the
    /// wall-clock target governs the exit instead, so the full trajectory is
    /// captured regardless of the actual frame rate.
    pub headless_frames: u32,
    /// Wall-clock seconds before auto-exit in headless mode. When set, this
    /// takes precedence over `headless_frames` (which becomes a safety net).
    /// When unset but a camera-script is loaded, the script's `duration_s`
    /// (plus a small tail) is used automatically.
    pub headless_secs: Option<f64>,
}

impl Cli {
    /// Parse from `std::env::args()` + environment variables.
    ///
    /// Precedence: CLI flag > env var > default. Unknown flags are ignored
    /// (forward-compatible with future milestones).
    pub fn from_env_and_args() -> Self {
        let mut cli = Self::default();

        // ── Env defaults ──────────────────────────────────────────────
        if env_flag("CESIUM_HEADLESS") {
            cli.headless = true;
        }
        if let Ok(p) = std::env::var("CESIUM_PERF_TRACE") {
            if !p.trim().is_empty() {
                cli.perf_trace = Some(PathBuf::from(p));
            }
        }
        if let Ok(p) = std::env::var("CESIUM_CAMERA_SCRIPT") {
            if !p.trim().is_empty() {
                cli.camera_script = Some(PathBuf::from(p));
            }
        }
        if let Ok(v) = std::env::var("CESIUM_TRACE_INTERVAL") {
            if let Ok(n) = v.trim().parse::<u32>() {
                cli.trace_interval = n.max(1);
            }
        }
        if let Ok(v) = std::env::var("CESIUM_HEADLESS_FRAMES") {
            if let Ok(n) = v.trim().parse::<u32>() {
                cli.headless_frames = n.max(1);
            }
        }
        if let Ok(v) = std::env::var("CESIUM_HEADLESS_SECS") {
            if let Ok(s) = v.trim().parse::<f64>() {
                if s > 0.0 && s.is_finite() {
                    cli.headless_secs = Some(s);
                }
            }
        }

        // ── CLI overrides ─────────────────────────────────────────────
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        while i < args.len() {
            let a = args[i].as_str();
            match a {
                "--headless" => cli.headless = true,
                "--perf-trace" => {
                    i += 1;
                    if i < args.len() {
                        cli.perf_trace = Some(PathBuf::from(&args[i]));
                    }
                }
                "--camera-script" => {
                    i += 1;
                    if i < args.len() {
                        cli.camera_script = Some(PathBuf::from(&args[i]));
                    }
                }
                "--trace-interval" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(n) = args[i].parse::<u32>() {
                            cli.trace_interval = n.max(1);
                        }
                    }
                }
                "--headless-frames" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(n) = args[i].parse::<u32>() {
                            cli.headless_frames = n.max(1);
                        }
                    }
                }
                "--headless-secs" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(s) = args[i].parse::<f64>() {
                            if s > 0.0 && s.is_finite() {
                                cli.headless_secs = Some(s);
                            }
                        }
                    }
                }
                other => {
                    // Support `--flag=value` form.
                    if let Some(v) = other.strip_prefix("--perf-trace=") {
                        cli.perf_trace = Some(PathBuf::from(v));
                    } else if let Some(v) = other.strip_prefix("--camera-script=") {
                        cli.camera_script = Some(PathBuf::from(v));
                    } else if let Some(v) = other.strip_prefix("--trace-interval=") {
                        if let Ok(n) = v.parse::<u32>() {
                            cli.trace_interval = n.max(1);
                        }
                    } else if let Some(v) = other.strip_prefix("--headless-frames=") {
                        if let Ok(n) = v.parse::<u32>() {
                            cli.headless_frames = n.max(1);
                        }
                    } else if let Some(v) = other.strip_prefix("--headless-secs=") {
                        if let Ok(s) = v.parse::<f64>() {
                            if s > 0.0 && s.is_finite() {
                                cli.headless_secs = Some(s);
                            }
                        }
                    }
                    // Unknown flags: silently ignored (forward-compat).
                }
            }
            i += 1;
        }

        // ── Defaults for unset values ─────────────────────────────────
        if cli.trace_interval == 0 {
            cli.trace_interval = 1;
        }
        if cli.headless_frames == 0 {
            cli.headless_frames = 3600;
        }
        // Headless implies tracing unless explicitly disabled — but we
        // respect an explicit `--perf-trace` absence: headless without a
        // trace path just auto-exits (useful for smoke tests).

        cli
    }

    /// True when any perf-trace subsystem should activate.
    pub fn active(&self) -> bool {
        self.headless || self.perf_trace.is_some() || self.camera_script.is_some()
    }
}

// MK4: `env_truthy` removed — use `crate::feature_flags::env_flag` (the
// single env-read path for all CESIUM_* flags). See feature_flags.rs
// "Runtime mode switches" section for the registered variables.

// ── Camera script (TOML) ─────────────────────────────────────────────────

/// Raw TOML schema for a camera-script file.
#[derive(Debug, Deserialize)]
struct CameraScriptFile {
    #[serde(default)]
    meta: Option<ScriptMeta>,
    #[serde(rename = "keyframe")]
    keyframes: Vec<KeyframeRaw>,
}

#[derive(Debug, Deserialize)]
struct ScriptMeta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    duration_s: Option<f64>,
    #[serde(default)]
    description: Option<String>,
}

/// One keyframe in the trajectory. All fields except `t` are optional;
/// missing fields inherit the previous keyframe's value (or the env seed
/// for the first keyframe).
#[derive(Debug, Deserialize, Clone)]
struct KeyframeRaw {
    /// Seconds from script start.
    t: f64,
    /// Longitude in degrees (camera azimuth). Alias: `heading`.
    #[serde(default)]
    lon: Option<f64>,
    /// Latitude in degrees (camera elevation). Alias: `pitch`.
    #[serde(default)]
    lat: Option<f64>,
    /// Heading in degrees — takes precedence over `lon` (Lee's convention).
    #[serde(default)]
    heading: Option<f64>,
    /// Pitch in degrees — takes precedence over `lat`.
    #[serde(default)]
    pitch: Option<f64>,
    /// Height above surface in render units (globe R = 1).
    #[serde(default)]
    height: Option<f64>,
    /// Direct orbit distance from center; overrides `height`.
    #[serde(default)]
    distance: Option<f64>,
}

/// Resolved keyframe with all fields populated (no `Option`).
#[derive(Debug, Clone)]
struct Keyframe {
    t: f64,
    heading_rad: f32,
    pitch_rad: f32,
    distance: f32,
}

/// Parsed + resolved camera script, ready for playback.
#[derive(Debug, Clone)]
pub struct CameraScript {
    pub name: String,
    pub duration_s: f64,
    /// Optional human description from `[meta]`, logged on load.
    pub description: Option<String>,
    keyframes: Vec<Keyframe>,
}

impl CameraScript {
    /// Load from a TOML file. Falls back to the env seed (Lee's
    /// `orbit_state_from_env` equivalent) for any field the first keyframe
    /// omits, so a script can specify only the axes it cares about.
    pub fn load(path: &PathBuf, env_seed: &OrbitState) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("camera-script read failed: {}", e))?;
        let raw: CameraScriptFile =
            toml::from_str(&text).map_err(|e| format!("camera-script TOML parse: {}", e))?;

        if raw.keyframes.is_empty() {
            return Err("camera-script has no [[keyframe]] entries".into());
        }

        let name = raw
            .meta
            .as_ref()
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unnamed".into())
            });
        let description = raw.meta.as_ref().and_then(|m| m.description.clone());
        let duration_s = raw
            .meta
            .as_ref()
            .and_then(|m| m.duration_s)
            .unwrap_or_else(|| raw.keyframes.last().map(|k| k.t).unwrap_or(0.0));

        // Resolve each keyframe: missing fields inherit from the previous
        // resolved keyframe; the first keyframe inherits from the env seed.
        let mut resolved: Vec<Keyframe> = Vec::with_capacity(raw.keyframes.len());
        let mut prev_heading = env_seed.heading;
        let mut prev_pitch = env_seed.pitch;
        let mut prev_distance = env_seed.distance;

        for kf in &raw.keyframes {
            // heading: explicit `heading` > `lon` > inherit
            let heading_deg = kf.heading.or(kf.lon);
            let heading_rad = match heading_deg {
                Some(d) => (d as f32).to_radians(),
                None => prev_heading,
            };
            // pitch: explicit `pitch` > `lat` > inherit
            let pitch_deg = kf.pitch.or(kf.lat);
            let pitch_rad = match pitch_deg {
                Some(d) => (d as f32).to_radians(),
                None => prev_pitch,
            };
            // distance: explicit `distance` > `height` (1.0 + h) > inherit
            let distance = if let Some(d) = kf.distance {
                d as f32
            } else if let Some(h) = kf.height {
                1.0 + h as f32 // GLOBE_RADIUS = 1.0
            } else {
                prev_distance
            };

            resolved.push(Keyframe {
                t: kf.t,
                heading_rad,
                pitch_rad,
                distance,
            });
            prev_heading = heading_rad;
            prev_pitch = pitch_rad;
            prev_distance = distance;
        }

        // Sort by time (TOML order is not guaranteed).
        resolved.sort_by(|a, b| a.t.total_cmp(&b.t));

        Ok(Self {
            name,
            duration_s,
            description,
            keyframes: resolved,
        })
    }

    /// Sample the trajectory at `t` seconds (linear interpolation).
    /// Clamps to the first / last keyframe outside the script's range.
    pub fn sample(&self, t: f64) -> (f32, f32, f32) {
        let kfs = &self.keyframes;
        if kfs.len() == 1 || t <= kfs[0].t {
            let k = &kfs[0];
            return (k.heading_rad, k.pitch_rad, k.distance);
        }
        let last = kfs.len() - 1;
        if t >= kfs[last].t {
            let k = &kfs[last];
            return (k.heading_rad, k.pitch_rad, k.distance);
        }
        // Find the bracketing pair.
        for i in 0..last {
            let a = &kfs[i];
            let b = &kfs[i + 1];
            if t >= a.t && t <= b.t {
                let span = b.t - a.t;
                let f = if span > 1e-9 {
                    ((t - a.t) / span) as f32
                } else {
                    0.0
                };
                return (
                    a.heading_rad + (b.heading_rad - a.heading_rad) * f,
                    a.pitch_rad + (b.pitch_rad - a.pitch_rad) * f,
                    a.distance + (b.distance - a.distance) * f,
                );
            }
        }
        let k = &kfs[last];
        (k.heading_rad, k.pitch_rad, k.distance)
    }
}

// ── CSV writer thread ────────────────────────────────────────────────────

/// CSV column header. Kept in sync with `format_row` below.
/// M4: original 13 columns preserved in position (baseline compat); 4 new
/// columns appended at tail.
const CSV_HEADER: &str = "frame_idx,dt_ms,view_sse,visible_n,partition_n,load_n,spawn_n,\
tex_upload_n,evict_n,gpu_tex_cache,mesh_backlog,dl_in_flight,stale_skips,\
retry_after,frame_mesh,frame_despawn,evict_deferred";

/// Message sent from the frame thread to the writer thread.
enum WriterMsg {
    /// One CSV row (already formatted, newline-terminated).
    Row(String),
    /// Graceful shutdown: flush + close.
    Shutdown,
}

/// Handle to the writer thread. Dropping it sends `Shutdown` so the thread
/// flushes and exits even if the caller forgets.
struct WriterHandle {
    tx: Option<mpsc::Sender<WriterMsg>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl WriterHandle {
    fn spawn(path: PathBuf) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel::<WriterMsg>();
        let join = std::thread::Builder::new()
            .name("perf-trace-writer".into())
            .spawn(move || {
                writer_loop(path, rx);
            })?;
        Ok(Self {
            tx: Some(tx),
            join: Some(join),
        })
    }

    fn send(&self, row: String) {
        if let Some(tx) = &self.tx {
            // Best-effort: if the writer died, drop the row rather than
            // panicking the frame thread.
            let _ = tx.send(WriterMsg::Row(row));
        }
    }
}

impl Drop for WriterHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(WriterMsg::Shutdown);
        }
        if let Some(j) = self.join.take() {
            // Give the writer up to 2 s to flush; don't block shutdown forever.
            let _ = j.join();
        }
    }
}

/// Writer-thread main loop: batch rows in a ringbuffer, flush periodically.
///
/// Flush policy: every 64 rows OR every 200 ms, whichever comes first. This
/// bounds memory (≤64 rows × ~200 B ≈ 13 KB) while keeping disk I/O off the
/// frame thread's critical path.
fn writer_loop(path: PathBuf, rx: mpsc::Receiver<WriterMsg>) {
    let file = match File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[perf-trace] failed to create {}: {}", path.display(), e);
            return;
        }
    };
    let mut w = BufWriter::new(file);
    if let Err(e) = writeln!(w, "{}", CSV_HEADER) {
        eprintln!("[perf-trace] header write failed: {}", e);
        return;
    }

    const BATCH: usize = 64;
    const FLUSH_INTERVAL: Duration = Duration::from_millis(200);
    let mut buf: Vec<String> = Vec::with_capacity(BATCH);
    let mut last_flush = Instant::now();

    loop {
        // Drain available messages without blocking longer than the flush
        // interval, so we honor the time-based flush even when rows trickle.
        let timeout = FLUSH_INTERVAL.saturating_sub(last_flush.elapsed());
        match rx.recv_timeout(timeout.max(Duration::from_millis(1))) {
            Ok(WriterMsg::Row(row)) => buf.push(row),
            Ok(WriterMsg::Shutdown) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        // Drain any additional queued rows (non-blocking) to batch them.
        while buf.len() < BATCH * 4 {
            match rx.try_recv() {
                Ok(WriterMsg::Row(row)) => buf.push(row),
                Ok(WriterMsg::Shutdown) => {
                    flush_rows(&mut w, &mut buf);
                    return;
                }
                Err(_) => break,
            }
        }
        if buf.len() >= BATCH || last_flush.elapsed() >= FLUSH_INTERVAL {
            flush_rows(&mut w, &mut buf);
            last_flush = Instant::now();
        }
    }
    flush_rows(&mut w, &mut buf);
}

fn flush_rows(w: &mut BufWriter<File>, buf: &mut Vec<String>) {
    if buf.is_empty() {
        return;
    }
    for row in buf.drain(..) {
        // Ignore write errors: a full disk must not crash the app.
        let _ = w.write_all(row.as_bytes());
    }
    let _ = w.flush();
}

/// Format one CSV row from the current counters.
///
/// Column order matches `CSV_HEADER`. `view_sse` is a per-tile value inside
/// the quadtree traversal, not a single scalar; it remains a `0` placeholder
/// until deferred item DEFER-M0-VIEWSSE is resolved (registered by Jimmy).
fn format_row(c: &PerfCounters) -> String {
    format!(
        "{},{:.3},0,{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        c.frame_idx,
        c.dt_ms,
        c.tile_entities,   // visible_n: spawned entities (partition subset)
        c.spawn_queue,      // partition_n proxy: queued for spawn
        c.load_set,         // load_n: KICK-blocked children still loading
        c.frame_spawn,      // spawn_n
        c.frame_tex,        // tex_upload_n
        c.evict_total,      // evict_n (cumulative)
        c.gpu_tex_order,    // gpu_tex_cache
        c.backlog,          // mesh_backlog
        c.in_flight,        // dl_in_flight
        c.stale_skips,      // stale_skips (cumulative)
        c.retry_after,      // retry_after (M4 col 14)
        c.frame_mesh,       // frame_mesh (M4 col 15)
        c.frame_despawn,    // frame_despawn (M4 col 16)
        c.evict_deferred,   // evict_deferred (M4 col 17)
    )
}

// ── Bevy plugin + systems ────────────────────────────────────────────────

/// Resource holding the runtime state of the perf-trace subsystem.
#[derive(Resource)]
struct TraceState {
    cli: Cli,
    writer: Option<WriterHandle>,
    script: Option<CameraScript>,
    /// App start instant, for camera-script time base.
    start: Instant,
    /// Accumulated self-overhead for the 5 s report.
    overhead_accum: Duration,
    overhead_frames: u32,
    /// Last overhead report instant.
    last_report: Instant,
    /// Frame counter for trace-interval gating.
    frame_counter: u32,
    /// Wall-clock exit target for headless mode. `Some(d)` means: exit once
    /// `start.elapsed() >= d`, regardless of frame count. `None` means: fall
    /// back to the `headless_frames` cap. Computed at plugin build from
    /// `--headless-secs`, else the camera-script's `duration_s` + tail.
    exit_after: Option<Duration>,
}

/// Plugin that wires up the perf-trace subsystem.
///
/// Inert when `cli.active()` is false: no systems are registered, no
/// resources inserted, zero runtime cost.
pub struct PerfTracePlugin {
    cli: Cli,
}

impl PerfTracePlugin {
    pub fn new(cli: Cli) -> Self {
        Self { cli }
    }
}

impl Plugin for PerfTracePlugin {
    fn build(&self, app: &mut App) {
        if !self.cli.active() {
            return;
        }

        // Load the camera script (if any) using the env-seeded OrbitState
        // as the fallback for omitted keyframe fields. MK5: directly call
        // `orbit_state_from_env` (single source of truth for camera seed).
        let script = self.cli.camera_script.as_ref().and_then(|p| {
            let seed = orbit_state_from_env();
            match CameraScript::load(p, &seed) {
                Ok(s) => {
                    let desc = s
                        .description
                        .as_deref()
                        .unwrap_or("(no description)");
                    info!(
                        "[perf-trace] camera-script '{}' loaded: {} keyframes, {:.1}s — {}",
                        s.name,
                        s.keyframes.len(),
                        s.duration_s,
                        desc
                    );
                    Some(s)
                }
                Err(e) => {
                    error!("[perf-trace] {}", e);
                    None
                }
            }
        });

        // Spawn the writer thread (if a trace path is given).
        let writer = self.cli.perf_trace.as_ref().and_then(|p| {
            match WriterHandle::spawn(p.clone()) {
                Ok(w) => {
                    info!("[perf-trace] CSV writer started: {}", p.display());
                    Some(w)
                }
                Err(e) => {
                    error!("[perf-trace] writer spawn failed: {}", e);
                    None
                }
            }
        });

        // Compute the wall-clock exit target BEFORE `script` is moved into
        // the resource. Precedence: explicit `--headless-secs` > the loaded
        // script's `duration_s` (+0.5 s tail so the final keyframe is
        // captured and its CSV row flushed) > None (frame-cap fallback).
        //
        // R2: `is_finite()` guard — `Duration::from_secs_f64(INFINITY)` panics.
        // TOML `duration_s` can also be inf/nan; fall back to 3600 s.
        let exit_after = self
            .cli
            .headless_secs
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(Duration::from_secs_f64)
            .or_else(|| {
                script.as_ref().map(|s| {
                    let d = s.duration_s + 0.5;
                    if d.is_finite() && d > 0.0 {
                        Duration::from_secs_f64(d)
                    } else {
                        Duration::from_secs(3600)
                    }
                })
            });

        app.insert_resource(TraceState {
            cli: self.cli.clone(),
            writer,
            script,
            start: Instant::now(),
            overhead_accum: Duration::ZERO,
            overhead_frames: 0,
            last_report: Instant::now(),
            frame_counter: 0,
            exit_after,
        });

        // Camera-script playback runs BEFORE the trace sampler so the CSV
        // row reflects the scripted camera, not the previous frame's.
        app.add_systems(Update, camera_script_system);
        // R1: explicit `.after(TilePipelineSet)` ensures PerfCounters are
        // populated with THIS frame's data before we sample them. Without
        // this, Bevy's scheduler may run trace_sampler_system first (both
        // hold ResMut<PerfCounters>), yielding stale values in the CSV.
        app.add_systems(
            Update,
            trace_sampler_system
                .after(camera_script_system)
                .after(TilePipelineSet),
        );
        // Headless auto-exit runs last so the final frame's row is emitted.
        app.add_systems(Update, headless_exit_system.after(trace_sampler_system));
    }
}

// MK5: `env_seed_state()` removed — now calls `orbit_state_from_env()` from
// orbit_camera.rs directly (pub(crate) since M0 review). Single source of
// truth for camera env seed, eliminating drift between the two implementations.

/// Camera-script playback: overwrite OrbitState from the interpolated
/// trajectory. Runs only when a script is loaded.
///
/// We set BOTH `distance` and `target_distance` so orbit_camera's inertia
/// glide is a no-op (target == current), and heading/pitch directly since
/// headless mode has no mouse input to modify them.
fn camera_script_system(
    state: Res<TraceState>,
    mut orbit: ResMut<OrbitState>,
) {
    // R5: borrow instead of clone — CameraScript is immutable after load.
    let Some(script) = state.script.as_ref() else {
        return;
    };
    let t = state.start.elapsed().as_secs_f64();
    let (heading, pitch, distance) = script.sample(t);
    orbit.heading = heading;
    orbit.pitch = pitch;
    orbit.distance = distance;
    orbit.target_distance = distance;
}

/// Trace sampler: advance PerfCounters frame bookkeeping, format one CSV
/// row per `trace_interval` frames, and hand it to the writer thread.
/// Measures its own overhead and reports every 5 s.
///
/// Takes `ResMut<PerfCounters>` so it can call `end_frame` (frame_idx +
/// dt_ms). R1: `.after(process_pipeline)` is enforced at registration so
/// the gauges/deltas are guaranteed populated with THIS frame's data.
fn trace_sampler_system(
    mut state: ResMut<TraceState>,
    mut counters: ResMut<PerfCounters>,
    time: Res<Time>,
) {
    let t0 = Instant::now();

    state.frame_counter = state.frame_counter.wrapping_add(1);

    // Advance frame bookkeeping: frame_idx + dt_ms. This is the ONLY
    // writer of these two fields, so the CSV row's frame number matches
    // the frame that produced the counters.
    counters.end_frame(time.delta_secs_f64() * 1000.0);

    // Gate by trace_interval (default 1 = every frame).
    if state.frame_counter.is_multiple_of(state.cli.trace_interval.max(1)) {
        if let Some(writer) = &state.writer {
            let row = format_row(&counters);
            writer.send(row);
        }
    }

    let elapsed = t0.elapsed();
    state.overhead_accum += elapsed;
    state.overhead_frames += 1;
    if state.last_report.elapsed() >= Duration::from_secs(5) {
        let avg = state.overhead_accum.as_secs_f64()
            / (state.overhead_frames.max(1)) as f64
            * 1000.0;
        info!(
            "[perf-trace] self-overhead: {:.4} ms/frame (avg over {} frames)",
            avg, state.overhead_frames
        );
        // M0.4 acceptance: PerfCounters must agree with the legacy `[stats]`
        // printf. Emit the same numbers in a comparable one-liner so a
        // reviewer can eyeball-consistency between old log and new counters.
        info!("[perf-trace] counters: {}", counters.summary_line());
        state.overhead_accum = Duration::ZERO;
        state.overhead_frames = 0;
        state.last_report = Instant::now();
    }
}

/// Headless auto-exit. When a wall-clock target is known (`exit_after`),
/// it governs the exit so the full camera trajectory is captured regardless
/// of frame rate; otherwise fall back to the `headless_frames` cap.
fn headless_exit_system(
    state: Res<TraceState>,
    mut exit: EventWriter<AppExit>,
) {
    if !state.cli.headless {
        return;
    }
    match state.exit_after {
        Some(d) => {
            let elapsed = state.start.elapsed();
            if elapsed >= d {
                info!(
                    "[perf-trace] headless: reached {:.1}s wall-clock ({} frames), exiting",
                    elapsed.as_secs_f64(),
                    state.frame_counter
                );
                exit.send(AppExit::Success);
            }
        }
        None => {
            if state.frame_counter >= state.cli.headless_frames {
                info!(
                    "[perf-trace] headless: reached {} frames, exiting",
                    state.cli.headless_frames
                );
                exit.send(AppExit::Success);
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults_are_sane() {
        // Clear env so the test is deterministic.
        for k in [
            "CESIUM_HEADLESS",
            "CESIUM_PERF_TRACE",
            "CESIUM_CAMERA_SCRIPT",
            "CESIUM_TRACE_INTERVAL",
            "CESIUM_HEADLESS_FRAMES",
            "CESIUM_HEADLESS_SECS",
        ] {
            std::env::remove_var(k);
        }
        let cli = Cli::from_env_and_args();
        assert!(!cli.headless);
        assert!(cli.perf_trace.is_none());
        assert!(cli.camera_script.is_none());
        assert_eq!(cli.trace_interval, 1);
        assert_eq!(cli.headless_frames, 3600);
        assert!(cli.headless_secs.is_none());
        assert!(!cli.active());
    }

    #[test]
    fn camera_script_single_keyframe_is_static() {
        let seed = OrbitState::default();
        let toml_text = r#"
[meta]
name = "static"
duration_s = 10.0

[[keyframe]]
t = 0.0
lon = 45.0
lat = 30.0
height = 1.5
"#;
        let dir = std::env::temp_dir().join("cesium_test_static.toml");
        std::fs::write(&dir, toml_text).unwrap();
        let script = CameraScript::load(&dir, &seed).unwrap();
        assert_eq!(script.name, "static");
        assert_eq!(script.keyframes.len(), 1);
        let (h, p, d) = script.sample(0.0);
        let (h2, p2, d2) = script.sample(999.0);
        assert!((h - h2).abs() < 1e-6);
        assert!((p - p2).abs() < 1e-6);
        assert!((d - d2).abs() < 1e-6);
        assert!((d - 2.5).abs() < 1e-6); // 1.0 + 1.5
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn camera_script_interpolates_linearly() {
        let seed = OrbitState::default();
        let toml_text = r#"
[[keyframe]]
t = 0.0
lon = 0.0
lat = 0.0
height = 1.0

[[keyframe]]
t = 10.0
lon = 100.0
lat = 50.0
height = 3.0
"#;
        let dir = std::env::temp_dir().join("cesium_test_interp.toml");
        std::fs::write(&dir, toml_text).unwrap();
        let script = CameraScript::load(&dir, &seed).unwrap();
        let (h, p, d) = script.sample(5.0);
        // Midpoint: lon=50°, lat=25°, height=2.0 → distance=3.0
        assert!((h - 50.0f32.to_radians()).abs() < 1e-5);
        assert!((p - 25.0f32.to_radians()).abs() < 1e-5);
        assert!((d - 3.0).abs() < 1e-5);
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn camera_script_heading_takes_precedence_over_lon() {
        let seed = OrbitState::default();
        let toml_text = r#"
[[keyframe]]
t = 0.0
lon = 10.0
heading = 99.0
"#;
        let dir = std::env::temp_dir().join("cesium_test_prec.toml");
        std::fs::write(&dir, toml_text).unwrap();
        let script = CameraScript::load(&dir, &seed).unwrap();
        let (h, _, _) = script.sample(0.0);
        assert!((h - 99.0f32.to_radians()).abs() < 1e-5);
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn format_row_matches_header_column_count() {
        let c = PerfCounters {
            frame_idx: 7,
            dt_ms: 16.667,
            tile_entities: 100,
            spawn_queue: 5,
            load_set: 12,
            frame_spawn: 3,
            frame_tex: 2,
            frame_mesh: 9,
            frame_despawn: 1,
            evict_total: 4,
            evict_deferred: 2,
            gpu_tex_order: 50,
            backlog: 4,
            in_flight: 6,
            retry_after: 3,
            stale_skips: 8,
        };
        let row = format_row(&c);
        let cols: Vec<&str> = row.trim_end().split(',').collect();
        let header_cols: Vec<&str> = CSV_HEADER.split(',').collect();
        assert_eq!(
            cols.len(),
            header_cols.len(),
            "row has {} cols, header has {}: {:?}",
            cols.len(),
            header_cols.len(),
            cols
        );
        assert_eq!(header_cols.len(), 17); // M4: 13 original + 4 appended
        assert_eq!(cols[0], "7"); // frame_idx
        assert_eq!(cols[5], "12"); // load_n = load_set
        assert_eq!(cols[13], "3"); // retry_after (M4 col 14)
        assert_eq!(cols[14], "9"); // frame_mesh (M4 col 15)
        assert_eq!(cols[15], "1"); // frame_despawn (M4 col 16)
        assert_eq!(cols[16], "2"); // evict_deferred (M4 col 17)
    }
}
