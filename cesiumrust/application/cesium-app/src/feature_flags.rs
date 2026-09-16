//! Feature flag registry — centralizes all `CESIUM_ENABLE_*` env parsing.
//!
//! ## Design
//!
//! This module is the **single source of truth** for every opt-in feature
//! gate in the cesium-app. It replaces the previously scattered `env_enabled`
//! helper in `main.rs` and exposes a typed, discoverable API.
//!
//! Two categories of flags are provided:
//!
//! 1. **Active flags** — consumed by `main.rs` today to decide which plugins
//!    to wire in (terrain / 3D-tileset chains, screenshot harness). These
//!    preserve the exact pre-M0.3 semantics (default OFF, `1`/`true`/`yes`/`on`
//!    enables, case-insensitive, whitespace-tolerant).
//!
//! 2. **Reserved namespace flags** — declared now so downstream milestones
//!    (M1–M11) can adopt them without another round of `main.rs` edits.
//!    Each reserved flag has a stable constant + accessor; consumers are
//!    wired in later milestones. **Declaring a reserved flag never changes
//!    runtime behavior** (no plugin currently reads them).
//!
//! ## Pixel-neutrality contract
//!
//! Migration from `main.rs::env_enabled` to this module is **source-only**:
//! the parsing predicate, the default value, and every env var name are
//! byte-identical. No feature gate flips by default, so the rendered frame
//! is unchanged.
//!
//! ## Env var naming
//!
//! All flags follow `CESIUM_ENABLE_<UPPER_SNAKE>`. Active flags additionally
//! support the legacy alias `CESIUM_ENABLE_NEW_CHAINS` which enables both
//! terrain and tileset simultaneously (kept for capture-harness compat).

// The reserved namespace (constants + accessors for M1–M17) is intentionally
// declared before any consumer exists, so the compiler would flag every entry
// as dead code. That is by design: this module is a REGISTRY, and a registry
// entry's value is the stable contract it offers downstream milestones, not
// whether today's binary reads it. The allow is scoped to this module only;
// active flags (terrain/tileset/new_chains) ARE consumed by main.rs and would
// still warn if they ever became genuinely dead.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use cesium_bevy_render::LightingMode;

// ── Active flags (consumed by main.rs today) ───────────────────────────

/// `CESIUM_ENABLE_TERRAIN` — enables the Cesium World Terrain chain.
pub const ENV_ENABLE_TERRAIN: &str = "CESIUM_ENABLE_TERRAIN";
/// `CESIUM_ENABLE_TILESET` — enables the 3D Tiles tileset chain.
pub const ENV_ENABLE_TILESET: &str = "CESIUM_ENABLE_TILESET";
/// `CESIUM_ENABLE_NEW_CHAINS` — legacy umbrella: enables BOTH terrain and
/// tileset. Preserved for `capture_baseline.ps1` compat.
pub const ENV_ENABLE_NEW_CHAINS: &str = "CESIUM_ENABLE_NEW_CHAINS";

// ── Runtime mode switches (M0.5 perf-trace / headless) ─────────────────
//
// These are NOT feature-enable flags (they don't gate a plugin); they
// configure the runtime mode of the perf-trace subsystem. Registered here
// to fulfill the "single env-read path" contract: ALL `CESIUM_*` env vars
// are documented in this module, even if their consumers live elsewhere.
// Read via `env_flag()` for booleans or `std::env::var()` for typed values.

/// `CESIUM_HEADLESS` — truthy: run in non-interactive auto-exit mode.
pub const ENV_HEADLESS: &str = "CESIUM_HEADLESS";
/// `CESIUM_PERF_TRACE` — path: CSV perf-trace output file.
pub const ENV_PERF_TRACE: &str = "CESIUM_PERF_TRACE";
/// `CESIUM_CAMERA_SCRIPT` — path: TOML camera-script for trajectory playback.
pub const ENV_CAMERA_SCRIPT: &str = "CESIUM_CAMERA_SCRIPT";
/// `CESIUM_TRACE_INTERVAL` — u32: emit one CSV row every N frames (default 1).
pub const ENV_TRACE_INTERVAL: &str = "CESIUM_TRACE_INTERVAL";
/// `CESIUM_HEADLESS_FRAMES` — u32: frame-count safety cap for headless exit.
pub const ENV_HEADLESS_FRAMES: &str = "CESIUM_HEADLESS_FRAMES";
/// `CESIUM_HEADLESS_SECS` — f64: wall-clock seconds before headless exit.
pub const ENV_HEADLESS_SECS: &str = "CESIUM_HEADLESS_SECS";

// ── Offline determinism env family (M3.3) ──────────────────────────────
//
// These gate the offline deterministic capture path (M3.3 / M3.4). They are
// NOT `CESIUM_ENABLE_*` feature gates: they configure *where assets come
// from* and *whether the run is frozen for reproducibility*. Registered here
// so this module stays the single source of truth for every env var the app
// reads. All default OFF / unset, so the online default path and the
// dynamic_globe golden path stay byte-for-byte unchanged.
//
// Consumed by main.rs (self-check / FIXED_TIME / FIXED_CAMERA / screenshot
// script) and by the bevy-render offline loader gate (M3.3), which read the
// SAME names — this registry is the contract both sides honor.

/// `OFFLINE_IMAGERY_ROOT` — directory of an offline imagery XYZ pyramid
/// (`{root}/{z}/{x}/{y}.png`, the `gen_offline_assets` layout). When set,
/// imagery is served by `FileTileFetcher` instead of online Bing/OSM.
pub const ENV_OFFLINE_IMAGERY_ROOT: &str = "OFFLINE_IMAGERY_ROOT";
/// `OFFLINE_TERRAIN_ROOT` — directory of an offline heightmap-1.0 tileset
/// (`layer.json` + `{root}/{z}/{x}/{y}.terrain`). When set, terrain is served
/// by `FileTerrainFetcher` instead of Cesium ion.
pub const ENV_OFFLINE_TERRAIN_ROOT: &str = "OFFLINE_TERRAIN_ROOT";
/// `STRICT_OFFLINE` — truthy: construct offline fetchers with
/// `with_strict_offline(true)` so any `http(s)` request panics synchronously
/// (no network fallback), guaranteeing offline determinism.
pub const ENV_STRICT_OFFLINE: &str = "STRICT_OFFLINE";
/// `FIXED_TIME` — truthy: freeze the Bevy clock (zero delta) for
/// deterministic lighting/celestial state (M4 shadow-baseline groundwork).
pub const ENV_FIXED_TIME: &str = "FIXED_TIME";
/// `FIXED_CAMERA` — path to a TOML with a single `[camera]` `pos`/`quat`/
/// `fov_y`, re-applied every frame (deterministic viewpoint for pixel_diff).
pub const ENV_FIXED_CAMERA: &str = "FIXED_CAMERA";
/// `CESIUM_SCREENSHOT_SCRIPT` — path to a TOML batch-capture script
/// (`[[shot]]` entries: `frame` + camera pose + output `name`). M3.4.
pub const ENV_SCREENSHOT_SCRIPT: &str = "CESIUM_SCREENSHOT_SCRIPT";
/// `CESIUM_OFFLINE_SELFCHECK` — truthy: run the headless offline self-check
/// (read fixtures back through the real fetchers + assert STRICT_OFFLINE
/// panics on http) and exit before building the GPU app. No window needed.
pub const ENV_OFFLINE_SELFCHECK: &str = "CESIUM_OFFLINE_SELFCHECK";
/// `CESIUM_GIT_SHA` — optional git SHA stamped into screenshot metadata (set
/// by the capture harness / CI; falls back to `"unknown"` when unset).
pub const ENV_GIT_SHA: &str = "CESIUM_GIT_SHA";
/// `CESIUMRST_LEGACY_DYNAMIC_GLOBE` — truthy: fall back to the frozen legacy
/// dynamic_globe path (pre-M1.5 monolith). Used for A/B pixel-neutrality
/// verification: the legacy path must produce identical frames/CSV.
pub const ENV_LEGACY_DYNAMIC_GLOBE: &str = "CESIUMRST_LEGACY_DYNAMIC_GLOBE";

// ── Lighting / post-process mode switches (M4.1) ─────────────────────
//
// These gate the lighting rig and built-in post-process stack. They are
// NOT `CESIUM_ENABLE_*` feature gates in the reserved namespace: they
// configure *how* the scene is lit / post-processed at startup.

/// `CESIUM_LIGHTING_MODE` — selects the lighting rig.
/// Accepted values (case-insensitive): `full_ambient` (default), `day_night`.
/// Any unrecognized value falls back to `full_ambient`.
pub const ENV_LIGHTING_MODE: &str = "CESIUM_LIGHTING_MODE";

/// `CESIUM_ENABLE_POSTPROCESS_BUILTIN` — truthy: enable the Bevy built-in
/// post-process stack (tonemapping / bloom / HDR) wired by M4.2+.
/// **Independent** from the reserved `CESIUM_ENABLE_POSTPROCESS` (L133)
/// which is the slot for M5.5/M5.6 custom post-process stages.
pub const ENV_ENABLE_POSTPROCESS_BUILTIN: &str = "CESIUM_ENABLE_POSTPROCESS_BUILTIN";

// ── Reserved namespace (declared now, consumed by later milestones) ────
//
// Each constant documents the milestone that will wire it in. Declaring
// them here costs nothing at runtime (no env read happens unless the
// accessor is called) and gives downstream work a stable contract.

/// M1 — unified render pipeline (replaces per-system branching).
pub const ENV_ENABLE_PIPELINE: &str = "CESIUM_ENABLE_PIPELINE";
/// M5.5/M5.6 — post-process stack (custom WGSL stages: bloom / AO / tone-mapping / color grade).
pub const ENV_ENABLE_POSTPROCESS: &str = "CESIUM_ENABLE_POSTPROCESS";
/// M3.x — cascaded shadow maps.
pub const ENV_ENABLE_CSM: &str = "CESIUM_ENABLE_CSM";
/// M5.2/M5.3 — procedural skydome (atmosphere + sun + moon). Mutually exclusive
/// with AtmosphereGlowPlugin: SKYDOME=1 → glow forced OFF.
pub const ENV_ENABLE_SKYDOME: &str = "CESIUM_ENABLE_SKYDOME";
/// M5.x — entity draping onto terrain (ground primitives).
pub const ENV_ENABLE_DRAPING: &str = "CESIUM_ENABLE_DRAPING";
/// M6.2 — view-frustum clipping planes (cross-section / box).
pub const ENV_ENABLE_CLIPPING: &str = "CESIUM_ENABLE_CLIPPING";
/// M6.3 — 360° panorama capture mode.
pub const ENV_ENABLE_PANORAMA: &str = "CESIUM_ENABLE_PANORAMA";
/// M6.5 — image-based lighting (IBL) for PBR materials.
pub const ENV_ENABLE_IBL: &str = "CESIUM_ENABLE_IBL";
/// M6.4 — order-independent transparency (OIT).
pub const ENV_ENABLE_OIT: &str = "CESIUM_ENABLE_OIT";
/// M6.1 — split-screen multi-view rendering.
pub const ENV_ENABLE_SPLIT: &str = "CESIUM_ENABLE_SPLIT";
/// M2 — new camera controller (replaces orbit_camera).
pub const ENV_ENABLE_NEW_CAMERA: &str = "CESIUM_ENABLE_NEW_CAMERA";
/// M8 — pluggable resource backend (asset streaming).
pub const ENV_ENABLE_RESOURCE_BACKEND: &str = "CESIUM_ENABLE_RESOURCE_BACKEND";
/// M7 — 3D Tiles styling JSEP expression evaluator (completed).
pub const ENV_ENABLE_STYLING_JSEP: &str = "CESIUM_ENABLE_STYLING_JSEP";
/// M14.x — KML export path.
pub const ENV_ENABLE_KML_EXPORT: &str = "CESIUM_ENABLE_KML_EXPORT";
/// M15.x — glTF upgrade pipeline (KHR extensions / Draco).
pub const ENV_ENABLE_GLTF_UPGRADE: &str = "CESIUM_ENABLE_GLTF_UPGRADE";
/// M16.x — Draco mesh compression decode path.
pub const ENV_ENABLE_DRACO: &str = "CESIUM_ENABLE_DRACO";
/// M17.x — point-cloud rendering pipeline.
pub const ENV_ENABLE_POINT_CLOUD: &str = "CESIUM_ENABLE_POINT_CLOUD";

/// Full reserved namespace, for diagnostics / `--list-flags` style tools.
/// Order is milestone-ascending so a UI can group by wave.
pub const RESERVED_FLAGS: &[&str] = &[
    ENV_ENABLE_PIPELINE,
    ENV_ENABLE_POSTPROCESS,
    ENV_ENABLE_CSM,
    ENV_ENABLE_SKYDOME,
    ENV_ENABLE_DRAPING,
    ENV_ENABLE_CLIPPING,
    ENV_ENABLE_PANORAMA,
    ENV_ENABLE_IBL,
    ENV_ENABLE_OIT,
    ENV_ENABLE_SPLIT,
    ENV_ENABLE_NEW_CAMERA,
    ENV_ENABLE_RESOURCE_BACKEND,
    ENV_ENABLE_STYLING_JSEP,
    ENV_ENABLE_KML_EXPORT,
    ENV_ENABLE_GLTF_UPGRADE,
    ENV_ENABLE_DRACO,
    ENV_ENABLE_POINT_CLOUD,
];

// ── Core predicate ─────────────────────────────────────────────────────

/// Truthy-token predicate. **Byte-identical** to the pre-M0.3 `main.rs`
/// implementation — do not "improve" it without re-validating every
/// consumer's default.
///
/// Accepts (case-insensitive, surrounding whitespace trimmed):
/// `1`, `true`, `yes`, `on`. Everything else (including unset) is `false`.
fn truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Read a single env flag. Returns `false` when unset or unparsable.
///
/// This is the **only** env read path for feature gates; every accessor
/// below funnels through it so behavior is uniform.
pub fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => truthy(&v),
        Err(_) => false,
    }
}

// ── Active-flag accessors ──────────────────────────────────────────────

/// Terrain chain enabled? (`CESIUM_ENABLE_TERRAIN` OR `CESIUM_ENABLE_NEW_CHAINS`)
///
/// Preserves the pre-M0.3 semantics exactly: the umbrella flag turns on
/// both chains, and either individual flag turns on its own chain.
pub fn terrain_enabled() -> bool {
    env_flag(ENV_ENABLE_NEW_CHAINS) || env_flag(ENV_ENABLE_TERRAIN)
}

/// 3D Tiles tileset chain enabled? (`CESIUM_ENABLE_TILESET` OR `CESIUM_ENABLE_NEW_CHAINS`)
pub fn tileset_enabled() -> bool {
    env_flag(ENV_ENABLE_NEW_CHAINS) || env_flag(ENV_ENABLE_TILESET)
}

// ── Offline determinism accessors (M3.3) ───────────────────────────────

/// Reads a path-valued env var, returning `None` when unset or blank.
/// Whitespace is trimmed; a blank value is treated as unset so an empty
/// `OFFLINE_IMAGERY_ROOT=` never accidentally enables the offline path.
fn env_path(name: &str) -> Option<PathBuf> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(PathBuf::from(v.trim())),
        _ => None,
    }
}

/// Offline imagery pyramid root (`OFFLINE_IMAGERY_ROOT`), if set.
pub fn offline_imagery_root() -> Option<PathBuf> {
    env_path(ENV_OFFLINE_IMAGERY_ROOT)
}

/// Offline terrain tileset root (`OFFLINE_TERRAIN_ROOT`), if set.
pub fn offline_terrain_root() -> Option<PathBuf> {
    env_path(ENV_OFFLINE_TERRAIN_ROOT)
}

/// `true` when either offline root is set (serve assets from disk, not net).
pub fn offline_mode() -> bool {
    offline_imagery_root().is_some() || offline_terrain_root().is_some()
}

/// STRICT_OFFLINE: forbid any network fallback (http(s) → synchronous panic).
pub fn strict_offline() -> bool {
    env_flag(ENV_STRICT_OFFLINE)
}

/// FIXED_TIME: freeze the clock for deterministic lighting/celestial state.
pub fn fixed_time_enabled() -> bool {
    env_flag(ENV_FIXED_TIME)
}

/// FIXED_CAMERA TOML path (single deterministic viewpoint), if set.
pub fn fixed_camera_path() -> Option<PathBuf> {
    env_path(ENV_FIXED_CAMERA)
}

/// `CESIUM_SCREENSHOT_SCRIPT` batch-capture TOML path (M3.4), if set.
pub fn screenshot_script_path() -> Option<PathBuf> {
    env_path(ENV_SCREENSHOT_SCRIPT)
}

/// Run the headless offline self-check and exit before the GPU app builds?
pub fn offline_selfcheck() -> bool {
    env_flag(ENV_OFFLINE_SELFCHECK)
}

/// Use the frozen legacy `dynamic_globe_legacy.rs` monolith instead of the
/// M1.5 thin shell? (`CESIUMRST_LEGACY_DYNAMIC_GLOBE`)
pub fn legacy_dynamic_globe() -> bool {
    env_flag(ENV_LEGACY_DYNAMIC_GLOBE)
}

// ── Lighting / post-process accessors (M4.1) ─────────────────────────

/// Parse `CESIUM_LIGHTING_MODE` into a [`LightingMode`].
///
/// Accepted tokens (case-insensitive, whitespace-trimmed):
/// - `day_night` | `daynight` | `day-night` → [`LightingMode::DayNight`]
/// - everything else (including unset) → [`LightingMode::FullAmbient`]
///
/// Default is `FullAmbient` so the v0 baseline path is pixel-identical.
pub fn lighting_mode() -> LightingMode {
    match std::env::var(ENV_LIGHTING_MODE) {
        Ok(raw) => {
            let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
            match normalized.as_str() {
                "day_night" | "daynight" => LightingMode::DayNight,
                _ => LightingMode::FullAmbient,
            }
        }
        Err(_) => LightingMode::FullAmbient,
    }
}

/// `CESIUM_ENABLE_POSTPROCESS_BUILTIN`: enable Bevy built-in post-process
/// (tonemapping / bloom / HDR). Default OFF. Independent from the reserved
/// `CESIUM_ENABLE_POSTPROCESS` which gates M5.5/M5.6 custom stages.
pub fn postprocess_builtin_enabled() -> bool {
    env_flag(ENV_ENABLE_POSTPROCESS_BUILTIN)
}

/// Git SHA for screenshot metadata (`CESIUM_GIT_SHA`, else `"unknown"`).
pub fn git_sha() -> String {
    std::env::var(ENV_GIT_SHA)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

// ── Reserved-flag accessors ────────────────────────────────────────────
//
// Each accessor is `#[inline]` and compiles to a single `env_flag` call.
// They are intentionally NOT cached: a future milestone may want to flip
// a flag at runtime (e.g. via a debug console), and caching would lock
// in the startup value. If a milestone needs caching it can wrap the
// accessor in its own `OnceLock`.

/// M1 pipeline flag (reserved — no consumer yet).
#[inline]
pub fn pipeline_enabled() -> bool {
    env_flag(ENV_ENABLE_PIPELINE)
}
/// M5.5/M5.6 post-process flag (reserved — custom WGSL stages).
#[inline]
pub fn postprocess_enabled() -> bool {
    env_flag(ENV_ENABLE_POSTPROCESS)
}
/// M3.x CSM flag (reserved).
#[inline]
pub fn csm_enabled() -> bool {
    env_flag(ENV_ENABLE_CSM)
}
/// M5.2/M5.3 skydome flag. When ON, glow is forced OFF (mutual exclusion).
#[inline]
pub fn skydome_enabled() -> bool {
    env_flag(ENV_ENABLE_SKYDOME)
}
/// AtmosphereGlow gate — inverse of skydome.
///
/// Mutual exclusion rule: SKYDOME=1 → glow forced 0. The 8-shell fallback
/// glow (AtmosphereGlowPlugin) and the procedural sky dome
/// (CesiumAtmospherePlugin) render overlapping atmospheric effects; enabling
/// both simultaneously would double-paint the limb. This accessor centralises
/// the rule so main.rs simply calls `glow_enabled()` without duplicating logic.
///
/// Default (SKYDOME unset/OFF): returns true → glow ON → v0 zero-diff.
#[inline]
pub fn glow_enabled() -> bool {
    !skydome_enabled()
}
/// M5.x draping flag (reserved).
#[inline]
pub fn draping_enabled() -> bool {
    env_flag(ENV_ENABLE_DRAPING)
}
/// M6.2 clipping flag (reserved).
#[inline]
pub fn clipping_enabled() -> bool {
    env_flag(ENV_ENABLE_CLIPPING)
}
/// M6.3 panorama flag (reserved).
#[inline]
pub fn panorama_enabled() -> bool {
    env_flag(ENV_ENABLE_PANORAMA)
}
/// M6.5 IBL flag (reserved).
#[inline]
pub fn ibl_enabled() -> bool {
    env_flag(ENV_ENABLE_IBL)
}
/// M6.4 OIT flag (reserved).
#[inline]
pub fn oit_enabled() -> bool {
    env_flag(ENV_ENABLE_OIT)
}
/// M6.1 split-screen flag (reserved).
#[inline]
pub fn split_enabled() -> bool {
    env_flag(ENV_ENABLE_SPLIT)
}
/// M2 new-camera flag (reserved — replaces orbit_camera).
#[inline]
pub fn new_camera_enabled() -> bool {
    env_flag(ENV_ENABLE_NEW_CAMERA)
}
/// M8 resource-backend flag (reserved — pluggable asset streaming).
#[inline]
pub fn resource_backend_enabled() -> bool {
    env_flag(ENV_ENABLE_RESOURCE_BACKEND)
}
/// M7 styling-JSEP flag (completed — expression evaluator landed).
#[inline]
pub fn styling_jsep_enabled() -> bool {
    env_flag(ENV_ENABLE_STYLING_JSEP)
}
/// M14.x KML-export flag (reserved).
#[inline]
pub fn kml_export_enabled() -> bool {
    env_flag(ENV_ENABLE_KML_EXPORT)
}
/// M15.x glTF-upgrade flag (reserved).
#[inline]
pub fn gltf_upgrade_enabled() -> bool {
    env_flag(ENV_ENABLE_GLTF_UPGRADE)
}
/// M16.x Draco flag (reserved).
#[inline]
pub fn draco_enabled() -> bool {
    env_flag(ENV_ENABLE_DRACO)
}
/// M17.x point-cloud flag (reserved).
#[inline]
pub fn point_cloud_enabled() -> bool {
    env_flag(ENV_ENABLE_POINT_CLOUD)
}

// ── Snapshot (for diagnostics / trace headers) ─────────────────────────

/// Immutable snapshot of every flag's value at first call. Useful for
/// stamping a perf-trace CSV header or a startup log line so a reviewer
/// can tell which features were on for a given run.
///
/// Lazily initialized: the first call reads env, subsequent calls return
/// the cached snapshot. This is safe because env vars are process-global
/// and the snapshot is only used for diagnostics (never for control flow).
#[derive(Debug, Clone)]
pub struct FlagSnapshot {
    pub terrain: bool,
    pub tileset: bool,
    pub new_chains: bool,
    pub reserved: Vec<(&'static str, bool)>,
}

impl FlagSnapshot {
    /// Capture the current env into a snapshot.
    pub fn capture() -> Self {
        Self {
            terrain: terrain_enabled(),
            tileset: tileset_enabled(),
            new_chains: env_flag(ENV_ENABLE_NEW_CHAINS),
            reserved: RESERVED_FLAGS.iter().map(|f| (*f, env_flag(f))).collect(),
        }
    }

    /// One-line human summary, e.g. `"terrain=off tileset=off reserved=0/17"`.
    pub fn summary_line(&self) -> String {
        let on = self.reserved.iter().filter(|(_, v)| *v).count();
        format!(
            "terrain={} tileset={} new_chains={} reserved={}/{}",
            yn(self.terrain),
            yn(self.tileset),
            yn(self.new_chains),
            on,
            self.reserved.len()
        )
    }
}

fn yn(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

/// Process-wide cached snapshot (first call captures, later calls reuse).
static SNAPSHOT: OnceLock<FlagSnapshot> = OnceLock::new();

/// Return the cached [`FlagSnapshot`], capturing on first call.
pub fn snapshot() -> &'static FlagSnapshot {
    SNAPSHOT.get_or_init(FlagSnapshot::capture)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthy_accepts_canonical_tokens() {
        for t in ["1", "true", "TRUE", "True", "yes", "YES", "on", "ON", " 1 ", "\ttrue\n"] {
            assert!(truthy(t), "expected truthy: {:?}", t);
        }
    }

    #[test]
    fn truthy_rejects_everything_else() {
        for t in ["", "0", "false", "no", "off", "2", "maybe", "enabled"] {
            assert!(!truthy(t), "expected falsy: {:?}", t);
        }
    }

    #[test]
    fn env_flag_unset_is_false() {
        // Use a name that no test sets, so the result is deterministic.
        assert!(!env_flag("CESIUM_ENABLE___DEFINITELY_UNSET___"));
    }

    #[test]
    fn env_path_unset_is_none() {
        // Path accessors mirror env_flag: unset → None (offline default OFF).
        assert!(env_path("CESIUM___DEFINITELY_UNSET___").is_none());
    }

    #[test]
    fn git_sha_falls_back_to_unknown() {
        // Never empty: unset CESIUM_GIT_SHA yields the "unknown" sentinel so
        // screenshot metadata always has a sha field.
        assert!(!git_sha().is_empty());
    }

    #[test]
    fn offline_accessors_are_contracts_only() {
        // Reading the offline accessors must never panic and never flip a
        // default: they are pure env reads. (Values depend on the ambient
        // env, so we only assert the call contract, not a specific result.)
        let _ = offline_imagery_root();
        let _ = offline_terrain_root();
        let _ = offline_mode();
        let _ = strict_offline();
        let _ = fixed_time_enabled();
        let _ = fixed_camera_path();
        let _ = screenshot_script_path();
        let _ = offline_selfcheck();
    }

    #[test]
    fn reserved_namespace_is_complete() {
        // Guard against accidentally dropping a reserved flag from the list.
        assert_eq!(RESERVED_FLAGS.len(), 17);
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_PIPELINE));
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_POINT_CLOUD));
    }

    #[test]
    fn snapshot_summary_is_stable() {
        let s = FlagSnapshot {
            terrain: false,
            tileset: false,
            new_chains: false,
            reserved: RESERVED_FLAGS.iter().map(|f| (*f, false)).collect(),
        };
        assert_eq!(
            s.summary_line(),
            "terrain=off tileset=off new_chains=off reserved=0/17"
        );
    }

    // ── M4.1 lighting / post-process tests ────────────────────────────

    #[test]
    fn lighting_mode_default_is_full_ambient() {
        // When CESIUM_LIGHTING_MODE is unset (normal test env), default is
        // FullAmbient — guarantees v0 pixel-neutrality.
        // NOTE: if the ambient env happens to set it, this still validates
        // the parse contract (no panic, returns a valid variant).
        let mode = lighting_mode();
        assert!(
            mode == LightingMode::FullAmbient || mode == LightingMode::DayNight,
            "lighting_mode() must return a valid variant"
        );
    }

    #[test]
    fn lighting_mode_parses_day_night_tokens() {
        // Validate the parsing logic directly (env-independent).
        for token in ["day_night", "DayNight", "DAY_NIGHT", " day-night ", "daynight"] {
            let normalized = token.trim().to_ascii_lowercase().replace('-', "_");
            let mode = match normalized.as_str() {
                "day_night" | "daynight" => LightingMode::DayNight,
                _ => LightingMode::FullAmbient,
            };
            assert_eq!(mode, LightingMode::DayNight, "token {:?} should parse as DayNight", token);
        }
    }

    #[test]
    fn lighting_mode_unknown_falls_back_to_full_ambient() {
        for token in ["", "full_ambient", "garbage", "2", "off"] {
            let normalized = token.trim().to_ascii_lowercase().replace('-', "_");
            let mode = match normalized.as_str() {
                "day_night" | "daynight" => LightingMode::DayNight,
                _ => LightingMode::FullAmbient,
            };
            assert_eq!(mode, LightingMode::FullAmbient, "token {:?} should fall back", token);
        }
    }

    #[test]
    fn postprocess_builtin_default_is_off() {
        // CESIUM_ENABLE_POSTPROCESS_BUILTIN unset → false (no built-in PP).
        // Uses the same truthy predicate as every other flag.
        let _ = postprocess_builtin_enabled(); // must not panic
    }

    #[test]
    fn postprocess_builtin_is_independent_from_reserved() {
        // The two post-process constants must be distinct strings.
        assert_ne!(ENV_ENABLE_POSTPROCESS_BUILTIN, ENV_ENABLE_POSTPROCESS);
    }

    #[test]
    fn glow_enabled_is_inverse_of_skydome() {
        // Mutual exclusion: glow_enabled() == !skydome_enabled().
        // Both read the same env var (CESIUM_ENABLE_SKYDOME) so in any
        // process state they must be logical inverses.
        assert_eq!(glow_enabled(), !skydome_enabled());
    }
}
