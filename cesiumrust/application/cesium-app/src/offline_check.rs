//! Headless offline self-check (`CESIUM_OFFLINE_SELFCHECK`).
//!
//! Proves — **without a window or GPU** — that the offline determinism wiring
//! is real:
//!
//! 1. the `OFFLINE_IMAGERY_ROOT` / `OFFLINE_TERRAIN_ROOT` fixtures (produced by
//!    `tools/gen_offline_assets`, M3.2) are read back successfully through the
//!    *actual* M3.1 fetchers (`FileTileFetcher` / `FileTerrainFetcher`), and
//! 2. `STRICT_OFFLINE=1` makes any `http(s)` request panic synchronously (no
//!    network fallback), the core offline-determinism guarantee.
//!
//! `main()` invokes [`run`] and exits *before* building the Bevy app when the
//! self-check flag is set, so this path is CI/headless friendly.
//!
//! The port methods return boxed futures that the offline fetchers resolve on
//! the first poll (synchronous `std::fs::read`); we drive them with a
//! [`Waker::noop`]-based [`block_on`] rather than starting a tokio runtime.

use cesium_network::{FileTerrainFetcher, FileTileFetcher, FileTileScheme, TerrainScheme};
use cesium_ports_driven::{TerrainProvider, TileFetcher};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::task::{Context, Poll, Waker};

use crate::feature_flags;

/// Heightmap-1.0 grid cell count (`65×65`); a decoded terrain tile must have
/// exactly this many positions.
const TERRAIN_POSITIONS: usize = 65 * 65;
/// PNG magic prefix (`\x89PNG`); proves an imagery tile is a real encoded PNG.
const PNG_MAGIC: [u8; 4] = [0x89, b'P', b'N', b'G'];

/// Drives an immediately-ready future to completion without a runtime.
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = Box::pin(fut);
    let mut cx = Context::from_waker(Waker::noop());
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!(
            "offline self-check: fetcher future returned Pending \
             (expected immediate Ready — offline IO must not block)"
        ),
    }
}

/// Canonicalizes `p`, falling back to the original when it does not exist yet
/// (so the subsequent read surfaces a clean NotFound rather than an IO error).
fn canon(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// Runs the offline self-check, printing a report. Returns the process exit
/// code: `0` when every check is green, `2` on any failure.
pub fn run() -> i32 {
    println!("[selfcheck] offline determinism self-check (headless, no GPU)");
    let strict = feature_flags::strict_offline();
    let imagery = feature_flags::offline_imagery_root();
    let terrain = feature_flags::offline_terrain_root();
    println!(
        "[selfcheck] STRICT_OFFLINE={} imagery_root={:?} terrain_root={:?}",
        strict, imagery, terrain
    );

    let mut checks = 0usize;
    let mut failures: Vec<String> = Vec::new();

    // ── Imagery read-back through FileTileFetcher (Xyz) ────────────────────
    match &imagery {
        Some(raw) => {
            let root = canon(raw);
            let fetcher = FileTileFetcher::new(&root, FileTileScheme::Xyz).with_strict_offline(strict);
            for url in ["0/0/0", "1/0/0", "3/5/2"] {
                match block_on(fetcher.fetch(url, 1.0)) {
                    Ok(bytes) if bytes.len() >= 4 && bytes[..4] == PNG_MAGIC => checks += 1,
                    Ok(_) => failures.push(format!("imagery '{url}': not a PNG (bad magic)")),
                    Err(e) => failures.push(format!("imagery '{url}': {e}")),
                }
            }
        }
        None => println!("[selfcheck] OFFLINE_IMAGERY_ROOT unset — imagery read-back skipped"),
    }

    // ── Terrain read-back through FileTerrainFetcher (Tms, from layer.json) ─
    match &terrain {
        Some(raw) => {
            let root = canon(raw);
            let layer = root.join("layer.json");
            // A bare (canonicalized) absolute path is accepted by from_layer_url.
            match FileTerrainFetcher::from_layer_url(
                &layer.display().to_string(),
                TerrainScheme::Tms,
                strict,
            ) {
                Ok(fetcher) => {
                    println!(
                        "[selfcheck] terrain layer.json maxzoom={}",
                        fetcher.maximum_level()
                    );
                    for (x, y, level) in [(0u32, 0u32, 0u32), (1, 0, 1), (2, 1, 2)] {
                        match block_on(fetcher.request_tile_geometry(x, y, level)) {
                            Ok(g) if g.positions.len() == TERRAIN_POSITIONS => checks += 1,
                            Ok(g) => failures.push(format!(
                                "terrain ({x},{y},{level}): {} positions (expected {TERRAIN_POSITIONS})",
                                g.positions.len()
                            )),
                            Err(e) => {
                                failures.push(format!("terrain ({x},{y},{level}): {e}"))
                            }
                        }
                    }
                }
                Err(e) => failures.push(format!("terrain from_layer_url: {e}")),
            }
        }
        None => println!("[selfcheck] OFFLINE_TERRAIN_ROOT unset — terrain read-back skipped"),
    }

    // ── STRICT_OFFLINE: http(s) must panic synchronously ────────────────────
    if strict {
        if strict_offline_panics_on_http(imagery.as_deref()) {
            println!("[selfcheck] STRICT_OFFLINE: http(s) fetch panicked as required");
            checks += 1;
        } else {
            failures.push("STRICT_OFFLINE: http(s) fetch did NOT panic".to_string());
        }
    } else {
        println!("[selfcheck] STRICT_OFFLINE unset — http-panic assertion skipped");
    }

    if failures.is_empty() {
        println!("[selfcheck] PASS — {checks} checks green");
        0
    } else {
        for f in &failures {
            eprintln!("[selfcheck] FAIL: {f}");
        }
        println!("[selfcheck] FAIL — {} checks green, {} failures", checks, failures.len());
        2
    }
}

/// Returns `true` when a STRICT_OFFLINE `FileTileFetcher` panics on an `https`
/// URL (the required no-network-fallback behavior). The panic hook is silenced
/// so the expected panic does not pollute the report.
fn strict_offline_panics_on_http(imagery_root: Option<&Path>) -> bool {
    let root = imagery_root.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let fetcher = FileTileFetcher::new(root, FileTileScheme::Xyz).with_strict_offline(true);
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        // Under STRICT_OFFLINE an http(s) URL panics synchronously *inside*
        // `fetch` (before the future is built / any IO happens). `drop` the
        // never-run future so clippy's `let_underscore_future` stays quiet.
        drop(fetcher.fetch("https://example.com/0/0/0.png", 1.0));
    }))
    .is_err();
    std::panic::set_hook(prev_hook);
    panicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_on_drives_immediately_ready_future() {
        assert_eq!(block_on(async { 42u32 }), 42);
    }

    #[test]
    fn strict_offline_fetcher_panics_on_https() {
        // No fixture needed: the panic fires synchronously inside `fetch`
        // before any disk access, so any root works.
        assert!(strict_offline_panics_on_http(None));
    }

    #[test]
    fn canon_of_missing_path_is_passthrough() {
        let p = Path::new("definitely/does/not/exist");
        assert_eq!(canon(p), p.to_path_buf());
    }
}
