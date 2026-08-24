//! Ported from `packages/engine/Source/Core/EllipsoidTerrainProvider.js` (202 lines).
//!
//! A very simple terrain provider that produces geometry by tessellating
//! an ellipsoidal surface.
//!
//! ## Method-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `constructor` | [`EllipsoidTerrainProvider::new`] | |
//! | `errorEvent` | [`EllipsoidTerrainProvider::error_event`] | |
//! | `credit` | — | always `undefined` in JS |
//! | `tilingScheme` | [`EllipsoidTerrainProvider::tiling_scheme`] | |
//! | `hasWaterMask` | [`EllipsoidTerrainProvider::has_water_mask`] | always `false` |
//! | `hasVertexNormals` | [`EllipsoidTerrainProvider::has_vertex_normals`] | always `false` |
//! | `availability` | [`EllipsoidTerrainProvider::availability`] | always `None` |
//! | `requestTileGeometry` | [`EllipsoidTerrainProvider::request_tile_geometry`] | returns a 16×16 flat heightmap |
//! | `getLevelMaximumGeometricError` | [`EllipsoidTerrainProvider::get_level_maximum_geometric_error`] | |
//! | `getTileDataAvailable` | [`EllipsoidTerrainProvider::get_tile_data_available`] | always `None` |
//! | `loadTileDataAvailability` | [`EllipsoidTerrainProvider::load_tile_data_availability`] | always `None` |

use crate::ellipsoid::Ellipsoid;
use crate::event::Event;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::heightmap_terrain_data::{HeightmapBuffer, HeightmapTerrainData, HeightmapTerrainDataOptions};
use crate::terrain_provider;
use crate::tiling_scheme::TilingScheme;

/// A very simple terrain provider that produces geometry by tessellating an
/// ellipsoidal surface.
///
/// Mirrors the JS `EllipsoidTerrainProvider`.
pub struct EllipsoidTerrainProvider {
    tiling_scheme: GeographicTilingScheme,
    level_zero_maximum_geometric_error: f64,
    error_event: Event,
}

impl EllipsoidTerrainProvider {
    /// Creates a new `EllipsoidTerrainProvider`.
    ///
    /// Mirrors the JS constructor.
    pub fn new(
        tiling_scheme: Option<GeographicTilingScheme>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let tiling_scheme = tiling_scheme.unwrap_or_else(|| {
            GeographicTilingScheme::new(ellipsoid, None, None, None)
        });

        let level_zero_maximum_geometric_error =
            terrain_provider::get_estimated_level_zero_geometric_error_for_a_heightmap(
                tiling_scheme.ellipsoid(),
                64.0,
                tiling_scheme.get_number_of_x_tiles_at_level(0),
            );

        Self {
            tiling_scheme,
            level_zero_maximum_geometric_error,
            error_event: Event::new(),
        }
    }

    /// Gets an event that is raised when the terrain provider encounters an
    /// asynchronous error.
    pub fn error_event(&self) -> &Event {
        &self.error_event
    }

    /// Gets the tiling scheme used by this provider.
    pub fn tiling_scheme(&self) -> &GeographicTilingScheme {
        &self.tiling_scheme
    }

    /// Gets a value indicating whether or not the provider includes a water mask.
    pub fn has_water_mask(&self) -> bool {
        false
    }

    /// Gets a value indicating whether or not the requested tiles include vertex normals.
    pub fn has_vertex_normals(&self) -> bool {
        false
    }

    /// Gets an object that can be used to determine availability of terrain
    /// from this provider.
    ///
    /// Always returns `None` for `EllipsoidTerrainProvider` (mirrors JS
    /// `availability: { get: function() { return undefined; } }`).
    pub fn availability(&self) -> Option<()> {
        None
    }

    /// Gets the maximum geometric error at a given level.
    pub fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        self.level_zero_maximum_geometric_error / (1i64 << level) as f64
    }

    /// Requests the geometry for a given tile.
    ///
    /// Mirrors `requestTileGeometry`: returns a 16×16 flat (all-zero)
    /// heightmap wrapped in [`HeightmapTerrainData`]. The JS method returns
    /// `Promise.resolve(...)`; the Rust port returns `Some(...)`
    /// synchronously (the promise is unwrapped).
    pub fn request_tile_geometry(
        &self,
        _x: i32,
        _y: i32,
        _level: i32,
    ) -> Option<HeightmapTerrainData> {
        let width = 16usize;
        let height = 16usize;
        Some(HeightmapTerrainData::new(HeightmapTerrainDataOptions {
            buffer: Some(HeightmapBuffer::U8(vec![0u8; width * height])),
            width: Some(width),
            height: Some(height),
            ..Default::default()
        }))
    }

    /// Determines whether data for a tile is available to be loaded.
    ///
    /// Always returns `None` (mirrors JS `undefined`).
    pub fn get_tile_data_available(&self, _x: i32, _y: i32, _level: i32) -> Option<bool> {
        None
    }

    /// Makes sure we load availability data for a tile.
    ///
    /// Always returns `None` (mirrors JS `undefined`).
    pub fn load_tile_data_availability(&self, _x: i32, _y: i32, _level: i32) -> Option<()> {
        None
    }
}

