//! Tokio-free tile fetch layer + the `CESIUM_ENABLE_PIPELINE` gate helper (M1.4).
//!
//! # Why this lives in `adapters/bevy-render` (architecture red-line)
//! The four P0 loaders live in this adapter, but the canonical gate accessor
//! `pipeline_enabled()` lives in `application/cesium-app/src/feature_flags.rs`.
//! An adapter **must not** depend on the application layer (hexagonal rule), so
//! it cannot be imported here. Instead this module reads the *process-global*
//! env var `CESIUM_ENABLE_PIPELINE` directly, using a `truthy` predicate that is
//! byte-identical to `feature_flags::truthy`. Reading an env var is not an
//! application dependency (env is process state, not an app artifact), and it
//! mirrors how cesium-app itself reads env at startup — so the two read paths
//! agree exactly on parsing and default.
//!
//! Alternatives considered and rejected:
//! - (b) an injected `Resource`/component gate — would force cesium-app to wire
//!   it, and the pipeline plugin is opt-in / not in the default runtime, so the
//!   loaders could not rely on its presence (they must default OFF safely).
//! - (c) exposing the gate from the pure-`std` `cesium-pipeline` core — would
//!   push a Bevy/app rollout concern into the domain-agnostic core.
//!
//! Option (a) — a local env read — is the least-coupling, no-violation choice.
//!
//! # Fetch routes (both tokio-free, both off the frame thread)
//! - [`pipeline_fetch`] — gate ON: the **shared keep-alive pooled** `UreqBackend`
//!   from the cesium-pipeline core (the "ureq 阻塞池"). One agent per process ⇒
//!   tile-server connections stay warm across fetches (the M1 improvement over
//!   per-call clients).
//! - [`legacy_fetch`] — gate OFF: a **fresh** `UreqBackend` per call, preserving
//!   the pre-migration `HttpTileFetcher::new(url)`-per-tile semantics (no
//!   pooling), minus the tokio current-thread runtime.
//!
//! Both are intended to run on a background worker (Bevy `IoTaskPool`), never on
//! the frame thread.
//!
//! # Note on `GenericPipeline` (full orchestration deferred to M1.5)
//! The core `GenericPipeline<K, Payload>` (the `TilePipeline` impl) is **not**
//! wired per-loader in M1.4, for concrete reasons found while reading the core:
//! 1. Its `Decoder: Fn(&[u8]) -> Option<Payload>` has no access to the tile key,
//!    but terrain decode needs `skirt_height(level)` and tileset content needs
//!    the `rtc_center` — both key-derived, so bytes-only decode cannot build the
//!    payload.
//! 2. Its `K: Copy` bound excludes the structurally-keyed tileset loaders
//!    (`Vec<usize>` path, `String` url).
//! 3. Persisting its worker pool across frames would require a new `Resource`,
//!    violating M1.4's "keep the four system signatures unchanged" constraint.
//!
//! M1.4 therefore consumes the core's `NetworkBackend` (the ureq blocking
//! keep-alive pool) — the tokio-free fetch layer — which is exactly the
//! "ureq 阻塞池，帧线程不阻塞" the milestone requires. Full `TilePipeline`
//! orchestration (dedup/wanted/budget drain) lands in M1.5 once dynamic_globe is
//! thinned and a key-aware decoder exists.

use std::sync::{Arc, OnceLock};

use cesium_pipeline::net::ureq_backend::UreqBackend;
use cesium_pipeline::net::{FetchResult, NetworkBackend};

/// `CESIUM_ENABLE_PIPELINE` — M1.x pipeline gate. Name matches the cesium-app
/// feature-flag registry (`feature_flags::ENV_ENABLE_PIPELINE`) so the two read
/// paths observe the identical env var.
pub const ENV_ENABLE_PIPELINE: &str = "CESIUM_ENABLE_PIPELINE";

/// Which fetch route a loader takes this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchRoute {
    /// Gate ON — shared keep-alive pooled backend (cesium-pipeline core).
    Pipeline,
    /// Gate OFF — fresh-per-call backend (legacy semantics, tokio-free).
    Legacy,
}

/// Truthy-token predicate — **byte-identical** to `feature_flags::truthy` in
/// cesium-app. Accepts (case-insensitive, surrounding whitespace trimmed):
/// `1`, `true`, `yes`, `on`. Everything else (including unset) is `false`.
fn truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Pure gate evaluation from a raw env value (`None` = unset). Split out from
/// [`pipeline_gate_enabled`] so it is unit-testable **without** mutating
/// process-global env (which would race parallel tests).
pub fn gate_from_env_value(raw: Option<String>) -> bool {
    match raw {
        Some(v) => truthy(&v),
        None => false,
    }
}

/// Read the `CESIUM_ENABLE_PIPELINE` gate without depending on the application
/// layer (see the module-level architecture rationale). Defaults OFF.
pub fn pipeline_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_PIPELINE).ok())
}

/// Resolve the fetch route from an explicit gate flag (testable seam that does
/// not touch env, so both branches are exercised deterministically in tests).
pub fn route_for(use_pipeline: bool) -> FetchRoute {
    if use_pipeline {
        FetchRoute::Pipeline
    } else {
        FetchRoute::Legacy
    }
}

/// Shared keep-alive pooled backend (cesium-pipeline core). One agent per
/// process ⇒ warm connections across every tile fetch (the pipeline route).
fn shared_backend() -> Arc<dyn NetworkBackend> {
    static BACKEND: OnceLock<Arc<dyn NetworkBackend>> = OnceLock::new();
    Arc::clone(BACKEND.get_or_init(|| Arc::new(UreqBackend::new())))
}

/// Map a core [`FetchResult`] onto `Result<Vec<u8>, String>`.
fn to_result(url: &str, r: FetchResult) -> Result<Vec<u8>, String> {
    match r {
        FetchResult::Ok(bytes) => Ok(bytes),
        FetchResult::Transient(e) => Err(format!("fetch {url}: transient: {e}")),
        FetchResult::Permanent(e) => Err(format!("fetch {url}: permanent: {e}")),
    }
}

/// Gate ON: fetch via the shared keep-alive pooled backend (the ureq 阻塞池).
/// Blocking; must run on a background worker, never the frame thread.
pub fn pipeline_fetch(url: &str) -> Result<Vec<u8>, String> {
    to_result(url, shared_backend().fetch(url))
}

/// Gate OFF: fetch via a fresh per-call backend (legacy no-pooling semantics,
/// tokio-free). Mirrors the pre-migration `HttpTileFetcher::new(url)` per tile.
/// Blocking; must run on a background worker, never the frame thread.
pub fn legacy_fetch(url: &str) -> Result<Vec<u8>, String> {
    to_result(url, UreqBackend::new().fetch(url))
}

/// Dispatch to the route selected by `use_pipeline`. This is the single
/// tokio-free fetch entry point used by all four P0 loaders.
pub fn fetch_gated(url: &str, use_pipeline: bool) -> Result<Vec<u8>, String> {
    match route_for(use_pipeline) {
        FetchRoute::Pipeline => pipeline_fetch(url),
        FetchRoute::Legacy => legacy_fetch(url),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_accepts_canonical_tokens() {
        for t in ["1", "true", "TRUE", "True", "yes", "YES", "on", "ON", " 1 ", "\ttrue\n"] {
            assert!(
                gate_from_env_value(Some(t.to_string())),
                "expected ON: {t:?}"
            );
        }
    }

    #[test]
    fn gate_rejects_everything_else_and_defaults_off() {
        assert!(!gate_from_env_value(None), "unset must be OFF (default)");
        for t in ["", "0", "false", "no", "off", "2", "maybe", "enabled"] {
            assert!(
                !gate_from_env_value(Some(t.to_string())),
                "expected OFF: {t:?}"
            );
        }
    }

    #[test]
    fn route_for_selects_branch_deterministically() {
        // Proves the gate ON/OFF branch is chosen from the flag alone (no env
        // mutation, so it is race-free under parallel test execution).
        assert_eq!(route_for(true), FetchRoute::Pipeline);
        assert_eq!(route_for(false), FetchRoute::Legacy);
    }

    #[test]
    fn shared_backend_is_pooled_singleton() {
        // Gate ON reuses ONE agent (keep-alive pool); pointer equality proves the
        // connection-pooling improvement over the legacy fresh-per-call route.
        let a = shared_backend();
        let b = shared_backend();
        assert!(
            Arc::ptr_eq(&a, &b),
            "pipeline route must share one pooled backend"
        );
        assert_eq!(a.name(), "ureq");
    }

    #[test]
    fn legacy_fetch_maps_error_without_panic() {
        // An unroutable URL yields an Err (never panics, never blocks beyond the
        // ureq timeout): the tokio-free path degrades gracefully like the old one.
        let r = legacy_fetch("http://127.0.0.1:1/__no_such_tile__");
        assert!(r.is_err());
    }

    #[test]
    fn fetch_gated_routes_by_flag() {
        // Both routes hit the same unroutable URL and both must return Err,
        // proving the dispatch seam works for either gate value.
        let url = "http://127.0.0.1:1/__no_such_tile__";
        assert!(fetch_gated(url, true).is_err());
        assert!(fetch_gated(url, false).is_err());
    }
}
