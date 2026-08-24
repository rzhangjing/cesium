//! Terrain-provider bridge for the globe surface tile provider (Track B4-5).
//!
//! Mirrors the CesiumJS `GlobeSurfaceTileProvider` ↔ `TerrainProvider`
//! contract (`requestTileGeometry` / `getLevelMaximumGeometricError` /
//! `getTileDataAvailable` / `maximumLevel`).
//!
//! DEVIATION (B4-5): CesiumJS threads promises through the load queues; the
//! Rust port drives tile requests synchronously inside the frame (the
//! [`GlobeTerrainFetcher`] API is blocking), which matches the single-frame
//! synchronous loading semantics already registered on `QuadtreePrimitive`.

use cesium_core::cesium_terrain_provider::{CesiumTerrainProvider, TerrainTileData};
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::resource::ResourceError;
use cesium_core::runtime_error::RuntimeError;
use cesium_core::terrain_provider::TerrainProvider;
use cesium_core::tiling_scheme::TilingScheme;
use cesium_core::web_mercator_tiling_scheme::WebMercatorTilingScheme;

use crate::file_resource_backend::FileResourceBackend;

/// Outcome of one terrain tile geometry request.
///
/// failed/placeholder discipline (cesiumrust pitfall checkpoint): the three
/// outcomes MUST stay distinct —
/// - [`NoData`](Self::NoData): deterministic absence (file missing / known
///   unavailable); the tile may inherit ancestor geometry permanently.
/// - [`Transient`](Self::Transient): a recoverable IO failure; retry with a
///   cooldown and NEVER stamp as permanent no-data.
pub enum TerrainGeometryOutcome {
    /// The tile geometry arrived (heightmap or quantized-mesh payload).
    Data(TerrainTileData),
    /// Deterministic absence of data for this tile.
    NoData,
    /// A transient failure; retry on a later frame.
    Transient(String),
}

/// Blocking bridge between the globe surface tile provider and a terrain
/// provider (object-safe, so tests can substitute flaky/mock fetchers).
pub trait GlobeTerrainFetcher {
    /// Builds a fresh tiling scheme matching the provider's scheme (the
    /// quadtree and the mesh tessellators need owned instances).
    fn make_tiling_scheme(&self) -> Box<dyn TilingScheme>;
    /// The level-zero maximum geometric error (meters), mirroring
    /// `TerrainProvider.getLevelMaximumGeometricError(0)`.
    fn level_zero_maximum_geometric_error(&self) -> f64;
    /// The deepest level the provider serves (`None` = unbounded), mirroring
    /// CesiumJS `tileProvider.maximumLevel`.
    fn maximum_level(&self) -> Option<i32>;
    /// Mirrors `TerrainProvider.getTileDataAvailable`.
    fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool>;
    /// Mirrors `TerrainProvider.requestTileGeometry` (blocking).
    fn request_tile_geometry(&mut self, x: i32, y: i32, level: i32) -> TerrainGeometryOutcome;
}

/// A [`GlobeTerrainFetcher`] backed by [`CesiumTerrainProvider`] reading
/// `file://` URLs (offline: no network access).
pub struct FileTerrainFetcher {
    provider: CesiumTerrainProvider,
    backend: FileResourceBackend,
    is_geographic: bool,
    maximum_level: Option<i32>,
}

impl FileTerrainFetcher {
    /// Loads the provider from a `layer.json` URL (blocking).
    pub fn from_url(layer_json_url: &str) -> Result<Self, RuntimeError> {
        let backend = FileResourceBackend::new();
        let provider =
            block_on_sync(CesiumTerrainProvider::from_url(Some(layer_json_url), None, &backend))?;
        Ok(Self::from_provider(provider, backend))
    }

    /// Wraps an already-constructed provider.
    pub fn from_provider(
        provider: CesiumTerrainProvider,
        backend: FileResourceBackend,
    ) -> Self {
        // The `TilingScheme` trait is not downcastable; a geographic scheme's
        // rectangle is radian-bounded (|north| ≤ π/2) while a WebMercator
        // rectangle is expressed in meters (|north| ≈ 2.0e7).
        let is_geographic = provider.tiling_scheme().rectangle().north.abs() < 4.0;
        let maximum_level = provider.availability().map(|a| a.maximum_level());
        Self {
            provider,
            backend,
            is_geographic,
            maximum_level,
        }
    }
}

impl GlobeTerrainFetcher for FileTerrainFetcher {
    fn make_tiling_scheme(&self) -> Box<dyn TilingScheme> {
        if self.is_geographic {
            Box::new(GeographicTilingScheme::new(None, None, None, None))
        } else {
            Box::new(WebMercatorTilingScheme::new(None, None, None, None, None))
        }
    }

    fn level_zero_maximum_geometric_error(&self) -> f64 {
        self.provider.get_level_maximum_geometric_error(0)
    }

    fn maximum_level(&self) -> Option<i32> {
        self.maximum_level
    }

    fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool> {
        self.provider.get_tile_data_available(x, y, level)
    }

    fn request_tile_geometry(&mut self, x: i32, y: i32, level: i32) -> TerrainGeometryOutcome {
        let result = block_on_sync(self.provider.request_tile_geometry(
            x,
            y,
            level,
            &self.backend,
        ));
        match result {
            Ok(Some(data)) => TerrainGeometryOutcome::Data(data),
            Ok(None) => TerrainGeometryOutcome::NoData,
            Err(error) => classify_terrain_error(&error),
        }
    }
}

/// Classifies a provider error into the deterministic vs. transient classes
/// the tile pipeline needs (cesiumrust pitfall checkpoint): a 404 is
/// deterministic no-data; everything else is transient and must be retried.
fn classify_terrain_error(error: &RuntimeError) -> TerrainGeometryOutcome {
    let message = error.message.clone();
    let is_404 = message.contains("HTTP 404") || message.contains("File not found");
    if is_404 {
        TerrainGeometryOutcome::NoData
    } else {
        TerrainGeometryOutcome::Transient(message)
    }
}

/// Classifies a raw [`ResourceError`] (used by callers that bypass the
/// provider, e.g. availability tile fetches).
#[allow(dead_code)]
pub(crate) fn classify_resource_error(error: &ResourceError) -> TerrainGeometryOutcome {
    match error {
        ResourceError::HttpError { status: 404, .. } => TerrainGeometryOutcome::NoData,
        other => TerrainGeometryOutcome::Transient(other.to_string()),
    }
}

/// Drives a future to completion on the current thread without an executor.
///
/// The terrain provider's async chain resolves entirely through synchronous
/// steps (local file reads), so a no-op-waker poll loop always converges;
/// the loop is capped defensively against unexpected pending futures.
fn block_on_sync<F: std::future::Future>(future: F) -> F::Output {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let mut context = Context::from_waker(Waker::noop());
    let mut future = pin!(future);
    for _ in 0..64 {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => {}
        }
    }
    panic!("terrain provider future never became ready (synchronous local-file chain expected)")
}
