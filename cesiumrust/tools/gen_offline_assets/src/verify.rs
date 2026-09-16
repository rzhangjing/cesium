//! Integration read-back verification.
//!
//! After generation we construct the *real* M3.1 offline fetchers
//! (`FileTileFetcher` / `FileTerrainFetcher`) and pull a handful of tiles back
//! through their port methods, asserting they decode successfully. This proves
//! the generated layout, byte structure, and y-order conventions align exactly
//! with the fetchers' decode logic — the whole point of the fixture.
//!
//! The port methods return boxed futures, but the offline fetchers resolve them
//! synchronously on the first poll (plain `std::fs::read`). We drive them with a
//! minimal [`Waker::noop`]-based [`block_on`] rather than pulling in tokio, so
//! this tool never starts an async runtime.

use cesium_network::{FileTerrainFetcher, FileTileFetcher, FileTileScheme, TerrainScheme};
use cesium_ports_driven::{TerrainProvider, TileFetcher};
use std::future::Future;
use std::path::Path;
use std::task::{Context, Poll, Waker};

/// Drives an immediately-ready future to completion without a runtime.
///
/// The offline fetchers build futures that are `Ready` on the first poll (they
/// wrap a synchronous `std::fs::read`). A `Pending` result would mean the
/// fetcher contract changed, so we fail loudly instead of spinning.
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = Box::pin(fut);
    let mut cx = Context::from_waker(Waker::noop());
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!(
            "gen_offline_assets: offline future returned Pending \
             (expected immediate Ready — the fetcher must not block)"
        ),
    }
}

/// Reads back a representative set of imagery and terrain tiles through the
/// M3.1 fetchers. Returns the number of assertions that passed.
///
/// # Errors
///
/// Returns a human-readable error string on the first tile that fails to read
/// or decode, so a misaligned fixture surfaces immediately.
pub fn verify(
    imagery_root: &Path,
    terrain_root: &Path,
    terrain_max_level: u32,
) -> Result<usize, String> {
    let mut checks = 0usize;

    // --- imagery: FileTileFetcher (Xyz) -------------------------------------
    let imagery = FileTileFetcher::new(imagery_root, FileTileScheme::Xyz);
    for url in ["0/0/0", "0/1/0", "1/0/0", "1/2/1", "3/5/2", "3/15/7"] {
        let data = block_on(imagery.fetch(url, 1.0))
            .map_err(|e| format!("imagery fetch '{url}' failed: {e}"))?;
        if data.is_empty() {
            return Err(format!("imagery fetch '{url}' returned empty bytes"));
        }
        // PNG magic number — proves the file is a real encoded PNG, not garbage.
        if data.len() < 8 || data[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
            return Err(format!("imagery fetch '{url}' is not a PNG (bad magic)"));
        }
        checks += 1;
    }

    // --- terrain: FileTerrainFetcher (Tms) via explicit root -----------------
    let terrain = FileTerrainFetcher::new(terrain_root, TerrainScheme::Tms, terrain_max_level);
    for (x, y, level) in [(0u32, 0u32, 0u32), (1, 0, 1), (2, 1, 2), (10, 3, 4)] {
        if !terrain.get_availability(x, y, level) {
            return Err(format!(
                "terrain get_availability({x},{y},{level}) == false (missing on disk)"
            ));
        }
        let geom = block_on(terrain.request_tile_geometry(x, y, level))
            .map_err(|e| format!("terrain request_tile_geometry({x},{y},{level}) failed: {e}"))?;
        if geom.positions.len() != crate::generate::TERRAIN_GRID_SIZE.pow(2) {
            return Err(format!(
                "terrain ({x},{y},{level}) decoded {} positions (expected {})",
                geom.positions.len(),
                crate::generate::TERRAIN_GRID_SIZE.pow(2)
            ));
        }
        checks += 1;
    }

    // --- terrain: FileTerrainFetcher::from_layer_url (maxzoom wiring) --------
    let layer_url = format!(
        "file:///{}",
        terrain_root
            .join("layer.json")
            .display()
            .to_string()
            .replace('\\', "/")
    );
    let from_layer = FileTerrainFetcher::from_layer_url(&layer_url, TerrainScheme::Tms, true)
        .map_err(|e| format!("from_layer_url('{layer_url}') failed: {e}"))?;
    if from_layer.maximum_level() != terrain_max_level {
        return Err(format!(
            "layer.json maxzoom read as {} (expected {terrain_max_level})",
            from_layer.maximum_level()
        ));
    }
    checks += 1;

    Ok(checks)
}
