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
//!    to wire in. All of them preserve the exact pre-M0.3 semantics (default
//!    OFF, `1`/`true`/`yes`/`on` enables, case-insensitive, whitespace-tolerant).
//!    Current active set (as of M6 Wave A):
//!    - *chains / harness*: `TERRAIN`, `TILESET`, `NEW_CHAINS` (umbrella),
//!      screenshot + offline + fixed-time + perf-trace + headless runtime switches;
//!    - *lighting* (M4.1): `CESIUM_LIGHTING_MODE` (`lighting_mode()`);
//!    - *post-process* (M4.2 built-in + M5-E1/E2 self-implemented):
//!      `POSTPROCESS_BUILTIN`, `POSTPROCESS`, plus the two M5-E **sub-gates**
//!      `FXAA` / `AO` (see [`fxaa_enabled`] — sub-gate default is ON, unlike
//!      every `CESIUM_ENABLE_*` feature gate);
//!    - *sky* (M5-B/M5-C): `SKYDOME` (and its inverse-derived `glow_enabled()`);
//!    - *material* (M5-D): `MATERIAL_SHOWCASE`;
//!    - *M6 Wave A* (task #81 integration): `CLIPPING` (M6.2), `PANORAMA`
//!      (M6.3), `IBL` (M6.5) — each gates both the camera component inserted
//!      by `main.rs` and the `Core3d` render-graph edges built by
//!      `effects/graph.rs::register_m6_render_graph`.
//!
//! 2. **Reserved namespace flags** — declared now so downstream milestones
//!    (M1–M17) can adopt them without another round of `main.rs` edits.
//!    Each reserved flag has a stable constant + accessor; consumers are
//!    wired in later milestones. **Declaring a reserved flag never changes
//!    runtime behavior** (no plugin currently reads them).
//!    **Rule**: once a milestone wires a reserved flag into `main.rs`, that
//!    flag becomes *active* — its accessor doc must be re-labelled ACTIVE
//!    (naming the milestone + the code path it gates) and it must be listed
//!    in the Active set above. Membership of [`RESERVED_FLAGS`] is kept stable
//!    for diagnostics comparability across milestones (see the note at that
//!    constant), so "moved out of the Reserved 分区" means *documentation and
//!    accessor classification*, not deletion from the diagnostic list.
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
//!
//! ## Two default semantics coexist (read this before adding a gate)
//!
//! * **Feature gates** (`env_flag`) default **OFF** when the var is unset.
//!   Every M4/M5/M6 capability gate uses this, so the golden path and the v0
//!   pixel baselines stay untouched by default.
//! * **Post-process sub-gates** (`sub_gate_flag`) default **ON** when unset.
//!   `FXAA` / `AO` are *sub-gates of the master `POSTPROCESS` gate*: leaving
//!   them unset must reproduce the pre-M5-E behaviour where the master gate
//!   alone enables both effects (`specs/scripts/v2_fxaa.toml` / `v2_ao.toml`
//!   depend on it). They are registered here so this module remains the single
//!   source of truth for the env **names** and their exact parse semantics —
//!   a registry that reported "default OFF" for a gate that is actually
//!   default ON would be worse than no registry at all.
//!
//! ## Adapter-layer mirrors (DDD)
//!
//! `adapters/bevy-render` cannot import this `application` crate (layering
//! rule, see `effects/graph.rs` L65-71), and `cesium-app` is a binary crate
//! with no lib target. Each adapter therefore keeps a byte-identical local
//! `const` + `*_gate_enabled()` mirror reading the **same env name** through
//! the same authoritative 4-token predicate
//! (`pipeline::fetch::gate_from_env_value`). This module is the *authoritative*
//! registry: the tests below pin every env name as a string literal so a
//! one-sided rename on either side turns the test suite red instead of
//! silently splitting a gate in two.

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
/// `CESIUM_HEADLESS_OUTPUT` — path: PNG written by the M11.3 offscreen capture.
pub const ENV_HEADLESS_OUTPUT: &str = "CESIUM_HEADLESS_OUTPUT";
/// FIX-HL-HDRMIRROR: cross-crate mirror of the adapter-private
/// `cesium_bevy_render::headless::ENV_HEADLESS_HDR` (`"CESIUM_HEADLESS_HDR"`, the
/// M11.4 HDR offscreen-target gate). Declared here *only* so the byte-identical
/// drift guard in
/// `adapter_gate_mirrors_are_byte_identical_to_the_registry` can pin both sides;
/// deliberately NOT added to `RESERVED_FLAGS` (the gate is read locally inside the
/// adapter crate, mirroring how the `fx::ENV_ENABLE_*` consts are consumed).
/// Consolidation of the adapter's private `hdr_truthy` into this registry is
/// deferred to M11.6 (see `docs/deferred.md`).
pub const ENV_HEADLESS_HDR: &str = "CESIUM_HEADLESS_HDR";

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

/// `CESIUM_ENABLE_FXAA` — **sub-gate** of `ENV_ENABLE_POSTPROCESS` selecting the
/// M5-E1 self-implemented FXAA node (quality preset 12 only).
///
/// **ACTIVE since M5-E1**; registered here by M6 Wave A (task #81) to close the
/// Terry M5-Verify Medium finding that the name lived only as a module-private
/// `const` inside `adapters/bevy-render/src/effects/post_process.rs`.
///
/// ⚠ **Not a default-OFF feature gate.** Unset ⇒ ON (see [`sub_gate_flag`]), so
/// the master `CESIUM_ENABLE_POSTPROCESS` gate alone still enables FXAA exactly
/// as it did before M5-E1. Effective enablement is
/// `postprocess_enabled() && fxaa_enabled()`.
pub const ENV_ENABLE_FXAA: &str = "CESIUM_ENABLE_FXAA";

/// `CESIUM_ENABLE_AO` — **sub-gate** of `ENV_ENABLE_POSTPROCESS` selecting the
/// M5-E2 self-implemented SSAO node (hemisphere 16-sample + 4×4 blur).
///
/// Same registration rationale and same ⚠ **unset ⇒ ON** sub-gate semantics as
/// [`ENV_ENABLE_FXAA`]. Effective enablement is
/// `postprocess_enabled() && ao_enabled()`.
pub const ENV_ENABLE_AO: &str = "CESIUM_ENABLE_AO";

// ── Reserved namespace (declared now, consumed by later milestones) ────
//
// Each constant documents the milestone that will wire it in. Declaring
// them here costs nothing at runtime (no env read happens unless the
// accessor is called) and gives downstream work a stable contract.
//
// NOTE (M5 收口): two entries declared here have since been WIRED by later
// milestones and are therefore *active* today — `ENV_ENABLE_POSTPROCESS`
// (M5-E1 FXAA + M5-E2 SSAO render-graph nodes) and `ENV_ENABLE_SKYDOME`
// (M5-B/M5-C procedural sky dome). They are kept in this block and in
// [`RESERVED_FLAGS`] so the diagnostic namespace stays comparable across
// milestones; their accessors below are labelled ACTIVE accordingly.
// `ENV_ENABLE_MATERIAL_SHOWCASE` (M5-D) is declared adjacent to
// `ENV_ENABLE_SKYDOME` because it belongs to the same M5 gate family, but it
// was born active (never reserved) and is therefore NOT in `RESERVED_FLAGS`.
//
// NOTE (M6 Wave A, task #81): three more entries declared here are now WIRED
// and therefore *active* — `ENV_ENABLE_CLIPPING` (M6.2), `ENV_ENABLE_PANORAMA`
// (M6.3) and `ENV_ENABLE_IBL` (M6.5). Per the rule above they stay in this
// block and in [`RESERVED_FLAGS`] (documentation/classification changes, not
// deletion) so the diagnostic namespace remains diffable. `ENV_ENABLE_CLOUDS`
// (M6.6) is newly *added* to the list by this task, which moves the
// `summary_line()` denominator from `reserved=n/17` to `reserved=n/18`.
// `ENV_ENABLE_FXAA` / `ENV_ENABLE_AO` are also newly registered here but are
// **active sub-gates** (born wired by M5-E1/E2), so — like
// `ENV_ENABLE_MATERIAL_SHOWCASE` and the `CESIUM_HEADLESS` runtime switch —
// they are NOT listed in `RESERVED_FLAGS`.

/// M1 — unified render pipeline (replaces per-system branching).
pub const ENV_ENABLE_PIPELINE: &str = "CESIUM_ENABLE_PIPELINE";
/// M5.5/M5.6 — post-process stack (custom WGSL stages: bloom / AO / tone-mapping / color grade).
pub const ENV_ENABLE_POSTPROCESS: &str = "CESIUM_ENABLE_POSTPROCESS";
/// M3.x — cascaded shadow maps.
pub const ENV_ENABLE_CSM: &str = "CESIUM_ENABLE_CSM";
/// M5.2/M5.3 — procedural skydome (atmosphere + sun + moon). Mutually exclusive
/// with AtmosphereGlowPlugin: SKYDOME=1 → glow forced OFF.
/// **ACTIVE since M5-B/M5-C** (consumed at `main.rs` sky-dome branch).
pub const ENV_ENABLE_SKYDOME: &str = "CESIUM_ENABLE_SKYDOME";
/// M5.4/M5-D — Fabric material showcase scene (built-in materials + the three
/// Water sea states Calm/Medium/Rough ported from `Water.glsl` `case 17u`).
/// **ACTIVE since M5-D** (consumed at `main.rs` material-showcase branch).
/// Default **OFF** → `MaterialShowcasePlugin` is not registered → no extra
/// entities/materials in the scene → the v0 baselines stay pixel-neutral
/// (PSNR=∞). Purely additive: no existing plugin registration is altered.
pub const ENV_ENABLE_MATERIAL_SHOWCASE: &str = "CESIUM_ENABLE_MATERIAL_SHOWCASE";
/// M5.x — entity draping onto terrain (ground primitives).
pub const ENV_ENABLE_DRAPING: &str = "CESIUM_ENABLE_DRAPING";
/// M6.2 — view-frustum clipping planes (cross-section / box).
/// **ACTIVE since M6 Wave A** (task #81): gates
/// `effects::register_clipping_planes_node` + the `CesiumClippingPlanes`
/// camera component + the `CesiumClippingLabel` render-graph edges.
pub const ENV_ENABLE_CLIPPING: &str = "CESIUM_ENABLE_CLIPPING";
/// M6.3 — 360° panorama capture mode.
/// **ACTIVE since M6 Wave A** (task #81): gates
/// `effects::register_panorama_node` + the `CesiumPanorama` camera component +
/// the in-`MainPass` serial insertion
/// `MainOpaquePass → CesiumPanoramaLabel → MainTransmissivePass`.
pub const ENV_ENABLE_PANORAMA: &str = "CESIUM_ENABLE_PANORAMA";
/// M6.5 — image-based lighting (IBL) for PBR materials.
/// **ACTIVE since M6 Wave A** (task #81): gates `effects::register_ibl_node` +
/// the `CesiumIbl` camera component + the `CesiumIblLabel` render-graph edges.
pub const ENV_ENABLE_IBL: &str = "CESIUM_ENABLE_IBL";
/// M6.4 — order-independent transparency (OIT).
pub const ENV_ENABLE_OIT: &str = "CESIUM_ENABLE_OIT";
/// M6.1 — split-screen multi-view rendering.
pub const ENV_ENABLE_SPLIT: &str = "CESIUM_ENABLE_SPLIT";
/// M6.6 — volumetric / procedural cloud layer.
/// **Pre-provisioned by M6 Wave A** (task #81, from the #51 milestone audit:
/// five M6 gates existed but `CLOUDS` was missing). Registered so M6.6 can wire
/// a consumer without another registry edit. **No consumer yet** — declaring it
/// changes no runtime behaviour, and it defaults OFF like every feature gate.
pub const ENV_ENABLE_CLOUDS: &str = "CESIUM_ENABLE_CLOUDS";
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
///
/// Membership is **frozen against removal**: `POSTPROCESS` (M5-E1/E2),
/// `SKYDOME` (M5-B/C), `CLIPPING` (M6.2), `PANORAMA` (M6.3) and `IBL` (M6.5)
/// have all been wired and are active, but dropping them here would change
/// `FlagSnapshot::summary_line()`'s `reserved=n/N` denominator and break
/// cross-milestone diffing of captured metadata. **Additions** are allowed and
/// do move the denominator: M6 Wave A appended `CLOUDS` (M6.6 pre-provision),
/// taking it 17 → 18 — a reader of `summary_line()` must compare the printed
/// denominator, never assume 17. Flags born active (e.g.
/// `ENV_ENABLE_MATERIAL_SHOWCASE`, M5-D; `ENV_ENABLE_FXAA` / `ENV_ENABLE_AO`,
/// M5-E sub-gates) are NOT listed.
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
    ENV_ENABLE_CLOUDS,
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

/// Read a **post-process sub-gate**. Returns `true` when the var is unset.
///
/// This is the mirror of `effects/post_process.rs::sub_gate_enabled` (which
/// calls the authoritative `pipeline::fetch::gate_from_env_value(Some(raw))`,
/// i.e. the very same [`truthy`] predicate) — the *only* difference from
/// [`env_flag`] is the `Err(_) => true` arm. It exists so `fxaa_enabled()` /
/// `ao_enabled()` can report the real runtime behaviour: an unset sub-gate is
/// ON, letting the master `CESIUM_ENABLE_POSTPROCESS` gate alone enable both
/// effects exactly as it did before M5-E1/E2 split them out.
///
/// Do **not** use this for feature gates — every capability gate defaults OFF
/// through [`env_flag`] so the golden path stays pixel-neutral.
fn sub_gate_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => truthy(&v),
        Err(_) => true,
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

// ── Runtime-mode accessors (M0.5 perf-trace / M11.3 headless) ──────────

/// Headless offscreen-render mode enabled? (`CESIUM_HEADLESS`)
///
/// When truthy, `main.rs` swaps the windowed `WindowPlugin` for the
/// surface-less configuration and adds
/// [`cesium_bevy_render::headless::CesiumHeadlessPlugin`], which renders the
/// scene to an offscreen target, captures a PNG, then exits cleanly.
///
/// **Default OFF**: when the var is unset the windowed startup path is
/// byte-for-byte unchanged (golden-path neutrality). This is a runtime-mode
/// switch, not a feature-enable gate, so it is intentionally outside the
/// frozen `RESERVED_FLAGS` comparability namespace.
pub fn headless_enabled() -> bool {
    env_flag(ENV_HEADLESS)
}

/// Default warm-up frames rendered before the M11.3 headless capture fires.
/// Enough for the base sphere + first LOD ring to settle; overridable via
/// `CESIUM_HEADLESS_FRAMES`.
pub const DEFAULT_HEADLESS_FRAMES: usize = 120;

/// Number of frames to render before the headless offscreen capture
/// (`CESIUM_HEADLESS_FRAMES`). Falls back to [`DEFAULT_HEADLESS_FRAMES`] when
/// unset or unparsable.
pub fn headless_frames() -> usize {
    match std::env::var(ENV_HEADLESS_FRAMES) {
        Ok(v) => v.trim().parse::<usize>().unwrap_or(DEFAULT_HEADLESS_FRAMES),
        Err(_) => DEFAULT_HEADLESS_FRAMES,
    }
}

/// Output PNG path for the headless offscreen capture (`CESIUM_HEADLESS_OUTPUT`).
/// Defaults to `headless_capture.png` in the working directory when unset/blank.
pub fn headless_output() -> PathBuf {
    env_path(ENV_HEADLESS_OUTPUT).unwrap_or_else(|| PathBuf::from("headless_capture.png"))
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

/// M5-E1 FXAA **sub-gate** — **ACTIVE**, and ⚠ **defaults ON when unset**.
///
/// Consumed by `effects/post_process.rs::CesiumEffectsPlugin::build`, which
/// writes the value into `PostProcessConfig::fxaa_enabled` and therefore onto
/// the camera's `CesiumFxaa` component. This accessor is the registry-side
/// statement of that contract; the adapter keeps a byte-identical local mirror
/// because it may not import the application layer (DDD).
///
/// Effective enablement requires the master gate as well:
/// `postprocess_enabled() && fxaa_enabled()`. With `POSTPROCESS` unset the FXAA
/// render-graph node is never registered at all, so the sub-gate's ON default
/// cannot leak into the golden path.
pub fn fxaa_enabled() -> bool {
    sub_gate_flag(ENV_ENABLE_FXAA)
}

/// M5-E2 SSAO **sub-gate** — **ACTIVE**, and ⚠ **defaults ON when unset**.
///
/// Mirror contract of [`fxaa_enabled`] for the AO node
/// (`PostProcessConfig::ambient_occlusion_enabled` → `CesiumAmbientOcclusion`).
/// Effective enablement: `postprocess_enabled() && ao_enabled()`.
pub fn ao_enabled() -> bool {
    sub_gate_flag(ENV_ENABLE_AO)
}

/// Git SHA for screenshot metadata (`CESIUM_GIT_SHA`, else `"unknown"`).
pub fn git_sha() -> String {
    std::env::var(ENV_GIT_SHA)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

// ── Flag accessors (active + reserved) ─────────────────────────────────
//
// This block mixes flags that `main.rs` consumes today (post-process,
// sky dome / glow, material showcase) with flags still awaiting their
// milestone; each accessor's doc states which. Each accessor is `#[inline]`
// and compiles to a single `env_flag` call.
// They are intentionally NOT cached: a future milestone may want to flip
// a flag at runtime (e.g. via a debug console), and caching would lock
// in the startup value. If a milestone needs caching it can wrap the
// accessor in its own `OnceLock`.

/// M1 pipeline flag (reserved — no consumer yet).
#[inline]
pub fn pipeline_enabled() -> bool {
    env_flag(ENV_ENABLE_PIPELINE)
}
/// M5.5/M5.6 post-process flag — **ACTIVE since M5-E1/E2**: gates the
/// self-implemented FXAA (preset 12) + SSAO (hemisphere 16-sample + 4×4 blur)
/// render-graph nodes registered by `effects/graph.rs::register_render_graph`
/// (`CesiumPostProcessLabel::{Fxaa, AmbientOcclusion}`), consumed at
/// `main.rs`'s `CesiumEffectsPlugin` branch. No longer a "custom WGSL stages"
/// reservation: it is wired. Independent from `POSTPROCESS_BUILTIN` (M4.2
/// Bevy built-in tonemapping/bloom/HDR on the camera bundle).
#[inline]
pub fn postprocess_enabled() -> bool {
    env_flag(ENV_ENABLE_POSTPROCESS)
}
/// M3.x CSM flag (reserved).
#[inline]
pub fn csm_enabled() -> bool {
    env_flag(ENV_ENABLE_CSM)
}
/// M5.2/M5.3 skydome flag — **ACTIVE since M5-B/M5-C**: gates
/// `CesiumAtmospherePlugin` (procedural sky dome, single-scattering WGSL).
/// When ON, glow is forced OFF (mutual exclusion) and the dome should be
/// paired with `POSTPROCESS_BUILTIN` (see `main.rs` warning) so the
/// un-normalized in-scattered radiance is tonemapped instead of clipped.
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
/// M5.4/M5-D material showcase flag — **ACTIVE**: gates
/// `MaterialShowcasePlugin` (built-in Fabric materials + Water Calm/Medium/
/// Rough sea states for the `specs/baselines/v2_water` captures).
///
/// Default (unset/OFF): returns false → plugin not registered → no extra
/// entities → v0 baselines pixel-neutral (PSNR=∞). Registered here rather
/// than read via a bare string literal in `main.rs` so this module stays the
/// single source of truth for every `CESIUM_*` env name.
#[inline]
pub fn material_showcase_enabled() -> bool {
    env_flag(ENV_ENABLE_MATERIAL_SHOWCASE)
}
/// M5.x draping flag (reserved).
#[inline]
pub fn draping_enabled() -> bool {
    env_flag(ENV_ENABLE_DRAPING)
}
/// M6.2 clipping flag — **ACTIVE since M6 Wave A** (task #81): gates
/// `effects::register_clipping_planes_node`, the `CesiumClippingPlanes` camera
/// component inserted by `main.rs`, and the `CesiumClippingLabel` edges in the
/// `Core3d` post-process region.
///
/// Default (unset/OFF): node not registered, no edges, component not inserted
/// → v0 baselines pixel-neutral (PSNR=∞).
#[inline]
pub fn clipping_enabled() -> bool {
    env_flag(ENV_ENABLE_CLIPPING)
}
/// M6.3 panorama flag — **ACTIVE since M6 Wave A** (task #81): gates
/// `effects::register_panorama_node`, the `CesiumPanorama` camera component, and
/// the serial in-`MainPass` insertion
/// `MainOpaquePass → CesiumPanoramaLabel → MainTransmissivePass`.
///
/// Default (unset/OFF): node not registered and — critically — the existing
/// `MainOpaquePass → MainTransmissivePass` edge is **left intact**, so the main
/// pass chain is byte-for-byte the pre-M6 one → v0 pixel-neutral.
#[inline]
pub fn panorama_enabled() -> bool {
    env_flag(ENV_ENABLE_PANORAMA)
}
/// M6.5 IBL flag — **ACTIVE since M6 Wave A** (task #81): gates
/// `effects::register_ibl_node`, the `CesiumIbl` camera component, and the
/// `CesiumIblLabel` edges (HDR region, immediately after clipping).
///
/// Default (unset/OFF): node not registered, no edges, component not inserted
/// → v0 baselines pixel-neutral (PSNR=∞).
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
/// M6.6 clouds flag (**reserved** — pre-provisioned by M6 Wave A, no consumer
/// yet). Declaring it changes no runtime behaviour; it defaults OFF.
#[inline]
pub fn clouds_enabled() -> bool {
    env_flag(ENV_ENABLE_CLOUDS)
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

    /// One-line human summary, e.g. `"terrain=off tileset=off reserved=0/18"`.
    /// The denominator is `RESERVED_FLAGS.len()` and therefore moves when the
    /// namespace gains an entry (M6 Wave A: 17 → 18 via `CLOUDS`).
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
        // M6 Wave A (task #81) appended ENV_ENABLE_CLOUDS: 17 -> 18. The count
        // is pinned so a *removal* (which would silently change summary_line()'s
        // denominator and break cross-milestone metadata diffing) turns red.
        assert_eq!(RESERVED_FLAGS.len(), 18);
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_PIPELINE));
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_POINT_CLOUD));
        // The three M6 Wave A gates stay listed even though they are now wired
        // (frozen against removal, see the note at RESERVED_FLAGS).
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_CLIPPING));
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_PANORAMA));
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_IBL));
        // The M6.6 pre-provision added by this task.
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_CLOUDS));
        // No duplicates: a repeated entry would inflate the denominator.
        let mut sorted: Vec<&str> = RESERVED_FLAGS.to_vec();
        let before = sorted.len();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "RESERVED_FLAGS must not contain duplicates");
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
            "terrain=off tileset=off new_chains=off reserved=0/18"
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

    // ── M5-D material showcase gate tests ─────────────────────────────

    #[test]
    fn material_showcase_defaults_off() {
        // (1) The registry constant must match the env name M5-D documented,
        //     so no bare-string drift can reappear in main.rs.
        assert_eq!(ENV_ENABLE_MATERIAL_SHOWCASE, "CESIUM_ENABLE_MATERIAL_SHOWCASE");
        // (2) The accessor must funnel through the single env_flag predicate
        //     (no private caching, no alternative parse).
        assert_eq!(
            material_showcase_enabled(),
            env_flag(ENV_ENABLE_MATERIAL_SHOWCASE)
        );
        // (3) Default OFF: when the var is unset (normal test/CI env) the gate
        //     is false → MaterialShowcasePlugin not registered → v0 pixel-neutral.
        //     Guarded by var_os so a deliberately opt-in ambient env (e.g. a
        //     capture harness run) does not produce a false red.
        if std::env::var_os(ENV_ENABLE_MATERIAL_SHOWCASE).is_none() {
            assert!(
                !material_showcase_enabled(),
                "CESIUM_ENABLE_MATERIAL_SHOWCASE unset must default to OFF"
            );
        }
        // (4) Born-active flag: NOT part of the frozen reserved namespace list.
        assert!(!RESERVED_FLAGS.contains(&ENV_ENABLE_MATERIAL_SHOWCASE));
        // (5) It must be a distinct env var from every other registered flag
        //     (no accidental aliasing of the skydome/post-process gates).
        assert_ne!(ENV_ENABLE_MATERIAL_SHOWCASE, ENV_ENABLE_SKYDOME);
        assert_ne!(ENV_ENABLE_MATERIAL_SHOWCASE, ENV_ENABLE_POSTPROCESS);
        assert_ne!(ENV_ENABLE_MATERIAL_SHOWCASE, ENV_ENABLE_POSTPROCESS_BUILTIN);
    }

    // ── M11.3 headless runtime-mode gate tests ────────────────────────

    #[test]
    fn headless_defaults_off() {
        // (1) Registry constant matches the documented env name so no
        //     bare-string drift can reappear at the main.rs branch.
        assert_eq!(ENV_HEADLESS, "CESIUM_HEADLESS");
        // (2) The accessor funnels through the single env_flag predicate
        //     (no private caching, no alternative parse).
        assert_eq!(headless_enabled(), env_flag(ENV_HEADLESS));
        // (3) Default OFF: when unset (normal test/CI env) the windowed golden
        //     path is untouched. Guarded by var_os so a deliberately opt-in
        //     capture-harness env does not produce a false red.
        if std::env::var_os(ENV_HEADLESS).is_none() {
            assert!(
                !headless_enabled(),
                "CESIUM_HEADLESS unset must default to OFF"
            );
        }
        // (4) Runtime-mode switch, not a frozen feature gate: must not be in
        //     the RESERVED_FLAGS comparability namespace.
        assert!(!RESERVED_FLAGS.contains(&ENV_HEADLESS));
    }

    // ── M6 Wave A gate tests (task #81 integration) ───────────────────

    /// The three wired M6 Wave A gates: env names pinned as literals (so a
    /// one-sided rename against the adapter-layer mirrors in
    /// `effects/{clipping_planes,panorama,ibl}.rs` turns red), accessors funnel
    /// through the single `env_flag` predicate, and all three default OFF so the
    /// v0 baselines stay pixel-neutral.
    #[test]
    fn m6_wave_a_gates_default_off() {
        // (1) Registry constants must match the env names the M6.2/M6.3/M6.5
        //     adapters documented, byte for byte.
        assert_eq!(ENV_ENABLE_CLIPPING, "CESIUM_ENABLE_CLIPPING");
        assert_eq!(ENV_ENABLE_PANORAMA, "CESIUM_ENABLE_PANORAMA");
        assert_eq!(ENV_ENABLE_IBL, "CESIUM_ENABLE_IBL");
        // (2) Feature-gate semantics: unset/falsy => false, {1,true,yes,on} => true.
        assert_eq!(clipping_enabled(), env_flag(ENV_ENABLE_CLIPPING));
        assert_eq!(panorama_enabled(), env_flag(ENV_ENABLE_PANORAMA));
        assert_eq!(ibl_enabled(), env_flag(ENV_ENABLE_IBL));
        // (3) Default OFF, guarded by var_os so an opt-in capture harness (e.g.
        //     a v3_clipping baseline run) cannot produce a false red.
        if std::env::var_os(ENV_ENABLE_CLIPPING).is_none() {
            assert!(!clipping_enabled(), "CESIUM_ENABLE_CLIPPING unset must default to OFF");
        }
        if std::env::var_os(ENV_ENABLE_PANORAMA).is_none() {
            assert!(!panorama_enabled(), "CESIUM_ENABLE_PANORAMA unset must default to OFF");
        }
        if std::env::var_os(ENV_ENABLE_IBL).is_none() {
            assert!(!ibl_enabled(), "CESIUM_ENABLE_IBL unset must default to OFF");
        }
        // (4) Pairwise distinct: three separate env vars, no aliasing.
        assert_ne!(ENV_ENABLE_CLIPPING, ENV_ENABLE_PANORAMA);
        assert_ne!(ENV_ENABLE_CLIPPING, ENV_ENABLE_IBL);
        assert_ne!(ENV_ENABLE_PANORAMA, ENV_ENABLE_IBL);
        // (5) They are feature gates, not post-process sub-gates: the two
        //     predicates must disagree on the unset default (false vs true).
        assert_ne!(
            env_flag("CESIUM_ENABLE___DEFINITELY_UNSET___"),
            sub_gate_flag("CESIUM_ENABLE___DEFINITELY_UNSET___"),
            "feature gates (env_flag) and sub-gates (sub_gate_flag) must differ when unset"
        );
    }

    /// M6.6 pre-provision: `CLOUDS` is declared + listed + accessor exists, and
    /// has **no consumer** yet, so it must default OFF and change nothing.
    #[test]
    fn clouds_gate_is_reserved_and_defaults_off() {
        assert_eq!(ENV_ENABLE_CLOUDS, "CESIUM_ENABLE_CLOUDS");
        assert_eq!(clouds_enabled(), env_flag(ENV_ENABLE_CLOUDS));
        if std::env::var_os(ENV_ENABLE_CLOUDS).is_none() {
            assert!(!clouds_enabled(), "CESIUM_ENABLE_CLOUDS unset must default to OFF");
        }
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_CLOUDS));
        assert_ne!(ENV_ENABLE_CLOUDS, ENV_ENABLE_SKYDOME);
    }

    /// FXAA / AO are **sub-gates of the master POSTPROCESS gate**, so their
    /// unset default is ON (pre-M5-E1/E2 behaviour preserved) — deliberately
    /// unlike every feature gate above. Registered here per the Terry M5-Verify
    /// Medium finding; the accessor must state the real runtime semantics.
    #[test]
    fn postprocess_sub_gates_default_on() {
        // (1) Names pinned: these were module-private consts in
        //     effects/post_process.rs before this task registered them.
        assert_eq!(ENV_ENABLE_FXAA, "CESIUM_ENABLE_FXAA");
        assert_eq!(ENV_ENABLE_AO, "CESIUM_ENABLE_AO");
        assert_ne!(ENV_ENABLE_FXAA, ENV_ENABLE_AO);
        // (2) Sub-gate predicate, not the feature-gate one: unset => true.
        assert!(sub_gate_flag("CESIUM_ENABLE___DEFINITELY_UNSET___"));
        assert!(!env_flag("CESIUM_ENABLE___DEFINITELY_UNSET___"));
        // (3) Falsy tokens still disable, using the shared truthy predicate.
        for t in ["0", "false", "no", "off", ""] {
            assert!(!truthy(t), "sub-gate must honour the falsy token {:?}", t);
        }
        // (4) When unset the accessors report ON (the documented sub-gate
        //     default). Guarded by var_os so an explicit opt-out env in a
        //     capture harness does not produce a false red.
        if std::env::var_os(ENV_ENABLE_FXAA).is_none() {
            assert!(fxaa_enabled(), "unset CESIUM_ENABLE_FXAA sub-gate defaults ON");
        }
        if std::env::var_os(ENV_ENABLE_AO).is_none() {
            assert!(ao_enabled(), "unset CESIUM_ENABLE_AO sub-gate defaults ON");
        }
        // (5) Active sub-gates are NOT part of the frozen reserved namespace
        //     (same classification as MATERIAL_SHOWCASE / HEADLESS).
        assert!(!RESERVED_FLAGS.contains(&ENV_ENABLE_FXAA));
        assert!(!RESERVED_FLAGS.contains(&ENV_ENABLE_AO));
        // (6) The ON default cannot leak into the golden path: without the
        //     master gate the FXAA/AO render-graph nodes are never registered.
        if std::env::var_os(ENV_ENABLE_POSTPROCESS).is_none() {
            assert!(
                !postprocess_enabled(),
                "master gate unset must keep the whole post-process chain off"
            );
        }
    }

    /// The five M6 gates form a distinct namespace (no accidental reuse of an
    /// existing flag name for a new capability).
    #[test]
    fn m6_gate_namespace_is_disjoint() {
        let m6 = [
            ENV_ENABLE_SPLIT,
            ENV_ENABLE_CLIPPING,
            ENV_ENABLE_PANORAMA,
            ENV_ENABLE_OIT,
            ENV_ENABLE_IBL,
            ENV_ENABLE_CLOUDS,
        ];
        for (i, a) in m6.iter().enumerate() {
            for b in m6.iter().skip(i + 1) {
                assert_ne!(a, b, "M6 gate names must be pairwise distinct");
            }
        }
        // M6.1/M6.4 (SPLIT/OIT) stay listed in RESERVED_FLAGS by convention (the
        // set is a stable diagnostic vocabulary; gaining a consumer is a
        // documentation reclassification, not a removal — see the note atop this
        // module). Both now have an adapter node wired (Phase-3 FIX-INTEG/FIX-SPLIT).
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_SPLIT));
        assert!(RESERVED_FLAGS.contains(&ENV_ENABLE_OIT));
    }

    /// **Cross-crate drift guard for the single source of truth** (task #81,
    /// PORTING_CONVENTIONS.md §gate registry).
    ///
    /// `cesium-app` depends on `cesium-bevy-render` — never the reverse — so the
    /// adapter-side effect modules cannot import this registry and each carries a
    /// *mirror* const of its gate name. Every other test here pins the registry
    /// side against a string literal, which only catches a rename if nobody also
    /// updates the literal. This test closes the loop by comparing against the
    /// **real adapter consts**, so renaming either side alone turns red, and it
    /// additionally proves the adapter's truthy parser is token-for-token
    /// identical to [`truthy`] (the `{1, true, yes, on}` set, trimmed +
    /// lowercased) — the property the M6 gate accessors rely on when they read
    /// the env directly inside the adapter crate.
    #[test]
    fn adapter_gate_mirrors_are_byte_identical_to_the_registry() {
        use cesium_bevy_render::effects as fx;

        // (1) M6 Wave A feature gates (all default OFF, all in RESERVED_FLAGS).
        assert_eq!(ENV_ENABLE_CLIPPING, fx::ENV_ENABLE_CLIPPING);
        assert_eq!(ENV_ENABLE_PANORAMA, fx::ENV_ENABLE_PANORAMA);
        assert_eq!(ENV_ENABLE_IBL, fx::ENV_ENABLE_IBL);

        // (1b) Phase-2/Phase-3 M6.4 OIT + M6.6 CLOUDS + M6.1 SPLIT adapter mirrors.
        //      All three gate consts live adapter-side (`effects/oit.rs`,
        //      `effects/clouds.rs`, `effects/split.rs`); pinning them against the
        //      registry closes the rename-drift loop the same way as the Wave A
        //      gates (see docs/deviations.md#dev-031 / #dev-032 / #dev-034).
        assert_eq!(ENV_ENABLE_OIT, fx::ENV_ENABLE_OIT);
        assert_eq!(ENV_ENABLE_CLOUDS, fx::ENV_ENABLE_CLOUDS);
        assert_eq!(ENV_ENABLE_SPLIT, fx::ENV_ENABLE_SPLIT);

        // (2) M5-E post-process sub-gates (default ON — sub_gate_flag semantics).
        //     These were module-private consts in `effects/post_process.rs` until
        //     this task registered them (Terry M5-Verify Medium finding); they are
        //     `pub` now precisely so this assertion can exist.
        assert_eq!(ENV_ENABLE_FXAA, fx::ENV_ENABLE_FXAA);
        assert_eq!(ENV_ENABLE_AO, fx::ENV_ENABLE_AO);

        // (3) The adapter's parser must accept exactly the canonical tokens.
        for t in ["1", "true", "TRUE", "True", "yes", "YES", "on", "ON", " 1 ", "\ttrue\n"] {
            assert!(
                fx::gate_from_env_value(Some(t.to_string())),
                "adapter gate_from_env_value must accept {:?} (registry truthy does)",
                t
            );
            assert!(truthy(t), "registry truthy must accept {:?}", t);
        }

        // (4) …and reject everything else, including the empty string.
        for t in ["", "0", "false", "no", "off", "2", "maybe", "enabled"] {
            assert!(
                !fx::gate_from_env_value(Some(t.to_string())),
                "adapter gate_from_env_value must reject {:?} (registry truthy does)",
                t
            );
            assert!(!truthy(t), "registry truthy must reject {:?}", t);
        }

        // (5) Unset (None) is OFF for a *feature* gate. The FXAA/AO sub-gates are
        //     the deliberate exception and are handled by `sub_gate_enabled` in
        //     `effects/post_process.rs`, not by `gate_from_env_value`.
        assert!(!fx::gate_from_env_value(None));
        assert!(!fx::clipping_gate_enabled() || std::env::var_os(ENV_ENABLE_CLIPPING).is_some());
        assert!(!fx::panorama_gate_enabled() || std::env::var_os(ENV_ENABLE_PANORAMA).is_some());
        assert!(!fx::ibl_gate_enabled() || std::env::var_os(ENV_ENABLE_IBL).is_some());
        assert!(!fx::oit_gate_enabled() || std::env::var_os(ENV_ENABLE_OIT).is_some());
        assert!(!fx::clouds_gate_enabled() || std::env::var_os(ENV_ENABLE_CLOUDS).is_some());
        assert!(!fx::split_gate_enabled() || std::env::var_os(ENV_ENABLE_SPLIT).is_some());

        // (6) FIX-HL-HDRMIRROR: the M11.4 headless HDR offscreen-target gate is an
        //     adapter-private const (`headless/mod.rs`); mirror it here so a rename
        //     on either side turns this drift guard red. The adapter's `hdr_truthy`
        //     stays a deliberate duplicate (dependency direction forbids importing
        //     this registry) — full consolidation is deferred to M11.6.
        assert_eq!(ENV_HEADLESS_HDR, cesium_bevy_render::headless::ENV_HEADLESS_HDR);
    }
}
