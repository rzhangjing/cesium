//! Ported from `packages/engine/Source/Core/CustomHeightmapTerrainProvider.js`.
//!
//! A simple [`TerrainProvider`] that gets height values from a callback
//! function. It can be used for procedurally generated terrain or as a way to
//! load custom heightmap data without creating a subclass of
//! [`TerrainProvider`].
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `CustomHeightmapTerrainProvider.GeometryCallback` | [`HeightmapCallback`] | sync closure; promises resolve before the call (DEVIATION 1) |
//! | `CustomHeightmapTerrainProvider` constructor | [`CustomHeightmapTerrainProvider::new`] | identical option handling incl. debug checks |
//! | `errorEvent` / `credit` / `tilingScheme` / `hasWaterMask` / `hasVertexNormals` / `availability` / `width` / `height` | accessor methods | `availability` is always `None` |
//! | `requestTileGeometry` | [`CustomHeightmapTerrainProvider::request_tile_geometry`] | DEVIATION 2 |
//! | `getLevelMaximumGeometricError` | [`CustomHeightmapTerrainProvider::get_level_maximum_geometric_error`] | identical |
//! | `getTileDataAvailable` | [`CustomHeightmapTerrainProvider::get_tile_data_available`] | always `None` |
//! | `loadTileDataAvailability` | [`CustomHeightmapTerrainProvider::load_tile_data_availability`] | always `None` |
//!
//! # DEVIATIONS
//!
//! 1. The JS callback may return a promise to a typed array; the Rust
//!    callback is synchronous (async callers resolve their data before
//!    invoking the provider). The JS `number[]` case (converted to
//!    `Float64Array`) maps to [`HeightmapBuffer::F64`].
//! 2. JS `requestTileGeometry` returns `undefined` when the callback returns
//!    `undefined`; the Rust port models that as `Ok(None)`. The JS `request`
//!    parameter / `RequestScheduler` plumbing is not modeled.

use crate::check;
use crate::credit::Credit;
use crate::ellipsoid::Ellipsoid;
use crate::event::Event;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::heightmap_terrain_data::{HeightmapBuffer, HeightmapTerrainData, HeightmapTerrainDataOptions};
use crate::terrain_provider::{get_estimated_level_zero_geometric_error_for_a_heightmap, TerrainProvider};
use crate::tiling_scheme::TilingScheme;

/// The callback used by [`CustomHeightmapTerrainProvider`] to request tile
/// heights. Mirrors `CustomHeightmapTerrainProvider.GeometryCallback`;
/// returning `None` mirrors the JS `undefined` result (the globe renders the
/// parent tile).
pub type HeightmapCallback = Box<dyn Fn(i32, i32, i32) -> Option<HeightmapBuffer>>;

/// Options for [`CustomHeightmapTerrainProvider::new`].
///
/// Mirrors `CustomHeightmapTerrainProvider.ConstructorOptions`.
pub struct CustomHeightmapTerrainProviderOptions {
    /// The callback function for requesting tile geometry.
    pub callback: Option<HeightmapCallback>,
    /// The number of columns per heightmap tile.
    pub width: Option<usize>,
    /// The number of rows per heightmap tile.
    pub height: Option<usize>,
    /// The tiling scheme specifying how the ellipsoidal surface is broken
    /// into tiles. Defaults to a [`GeographicTilingScheme`].
    pub tiling_scheme: Option<Box<dyn TilingScheme>>,
    /// The ellipsoid. If `tiling_scheme` is specified, this parameter is
    /// ignored and the tiling scheme's ellipsoid is used instead.
    pub ellipsoid: Option<Ellipsoid>,
    /// A credit for the data source, which is displayed on the canvas.
    pub credit: Option<String>,
}

impl Default for CustomHeightmapTerrainProviderOptions {
    fn default() -> Self {
        Self {
            callback: None,
            width: None,
            height: None,
            tiling_scheme: None,
            ellipsoid: None,
            credit: None,
        }
    }
}

/// A terrain provider that gets height values from a callback function.
pub struct CustomHeightmapTerrainProvider {
    callback: HeightmapCallback,
    tiling_scheme: Box<dyn TilingScheme>,
    width: usize,
    height: usize,
    level_zero_maximum_geometric_error: f64,
    error_event: Event,
    credit: Option<Credit>,
}

impl CustomHeightmapTerrainProvider {
    /// Creates a new `CustomHeightmapTerrainProvider`.
    ///
    /// Mirrors the JS constructor, including the debug checks for
    /// `options.callback`, `options.width` and `options.height`.
    ///
    /// # Panics
    ///
    /// Panics with a `DeveloperError` when `callback`, `width` or `height`
    /// is not provided.
    pub fn new(options: Option<CustomHeightmapTerrainProviderOptions>) -> Self {
        let options = options.unwrap_or_default();

        //>>includeStart('debug', pragmas.debug);
        check::defined("options.callback", options.callback.as_ref());
        check::defined("options.width", options.width.as_ref());
        check::defined("options.height", options.height.as_ref());
        //>>includeEnd('debug');

        let callback = options.callback.unwrap();

        let tiling_scheme = options.tiling_scheme.unwrap_or_else(|| {
            Box::new(GeographicTilingScheme::new(
                Some(options.ellipsoid.unwrap_or(Ellipsoid::WGS84)),
                None,
                None,
                None,
            ))
        });

        let width = options.width.unwrap();
        let height = options.height.unwrap();
        let max_tile_dimensions = width.max(height);

        let level_zero_maximum_geometric_error =
            get_estimated_level_zero_geometric_error_for_a_heightmap(
                tiling_scheme.ellipsoid(),
                max_tile_dimensions as f64,
                tiling_scheme.get_number_of_x_tiles_at_level(0),
            );

        let credit = options.credit.map(|credit| Credit::new(&credit, false));

        Self {
            callback,
            tiling_scheme,
            width,
            height,
            level_zero_maximum_geometric_error,
            error_event: Event::new(),
            credit,
        }
    }

    /// Gets an event that is raised when the terrain provider encounters an
    /// asynchronous error.
    pub fn error_event(&self) -> &Event {
        &self.error_event
    }

    /// Gets the credit to display when this terrain provider is active.
    pub fn credit(&self) -> Option<&Credit> {
        self.credit.as_ref()
    }

    /// Gets the number of columns per heightmap tile.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Gets the number of rows per heightmap tile.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Gets the availability object; always `None` for this provider.
    pub fn availability(&self) -> Option<&crate::tile_availability::TileAvailability> {
        None
    }

    /// Requests the geometry for a given tile. The result includes terrain
    /// data and indicates that all child tiles are available.
    ///
    /// Mirrors `requestTileGeometry`; returns `Ok(None)` when the callback
    /// returns `None` (JS `undefined`).
    pub fn request_tile_geometry(
        &self,
        x: i32,
        y: i32,
        level: i32,
    ) -> Option<HeightmapTerrainData> {
        let buffer = (self.callback)(x, y, level)?;

        Some(HeightmapTerrainData::new(HeightmapTerrainDataOptions {
            buffer: Some(buffer),
            width: Some(self.width),
            height: Some(self.height),
            ..Default::default()
        }))
    }

    /// Makes sure we load availability data for a tile; always `None` for
    /// this provider (mirrors the JS `undefined` return).
    pub fn load_tile_data_availability(&self, _x: i32, _y: i32, _level: i32) -> Option<()> {
        None
    }
}

impl TerrainProvider for CustomHeightmapTerrainProvider {
    fn tiling_scheme(&self) -> &dyn TilingScheme {
        self.tiling_scheme.as_ref()
    }

    fn has_water_mask(&self) -> bool {
        false
    }

    fn has_vertex_normals(&self) -> bool {
        false
    }

    fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        self.level_zero_maximum_geometric_error / (1 << level) as f64
    }

    fn get_tile_data_available(&self, _x: i32, _y: i32, _level: i32) -> Option<bool> {
        None
    }
}
