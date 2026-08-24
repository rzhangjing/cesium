//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTerrainProvider.js`.
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `TerrainState` | [`TerrainState`] constants | identical values |
//! | `TerrainCache` (`add`/`get`/`tidy`) | [`TerrainCache`] | timestamps via [`std::time::Instant`] (DEVIATION 3) |
//! | `GoogleEarthEnterpriseTerrainProvider` constructor | [`GoogleEarthEnterpriseTerrainProvider::new_internal`] | identical tiling scheme / level-zero error |
//! | `fromMetadata` | [`GoogleEarthEnterpriseTerrainProvider::from_metadata`] | identical |
//! | `computeChildMask` | [`compute_child_mask`] | identical |
//! | `requestTileGeometry` | [`GoogleEarthEnterpriseTerrainProvider::request_tile_geometry`] | promise dedup collapsed (DEVIATION 2) |
//! | `getLevelMaximumGeometricError` | [`GoogleEarthEnterpriseTerrainProvider::get_level_maximum_geometric_error`] | identical |
//! | `getTileDataAvailable` | [`GoogleEarthEnterpriseTerrainProvider::get_tile_data_available`] / [`GoogleEarthEnterpriseTerrainProvider::get_tile_data_available_async`] | sync trait method skips the async `populateSubtree` kick (DEVIATION 4) |
//! | `loadTileDataAvailability` | [`GoogleEarthEnterpriseTerrainProvider::load_tile_data_availability`] | identical (returns `()`) |
//! | `buildTerrainResource` | [`build_terrain_resource`] | identical URL |
//!
//! # DEVIATIONS
//!
//! 1. HTTP access goes through the injected [`ResourceBackend`] instead of
//!    XHR; `Request`/`RequestState` throttling objects are not modeled
//!    ("throttled" results map to `Ok(None)` from the backend, `undefined`
//!    return from `request_tile_geometry`).
//! 2. The JS `_terrainPromises`/`_terrainRequests` in-flight deduplication
//!    maps (shared promises between sibling children) are unnecessary in the
//!    sequential await model; each call fetches/decodes inline.
//! 3. [`TerrainCache`] timestamps use [`std::time::Instant`] instead of
//!    [`crate::julian_date::JulianDate`]; the 10-second tidy semantics are
//!    identical.
//! 4. The synchronous [`crate::terrain_provider::TerrainProvider`] trait
//!    method cannot kick off the async `metadata.populateSubtree` request;
//!    use [`GoogleEarthEnterpriseTerrainProvider::get_tile_data_available_async`]
//!    for the full JS behavior.
//! 5. The JS `TaskProcessor("decodeGoogleEarthEnterprisePacket")` worker is
//!    invoked synchronously in-process via
//!    [`crate::decode_google_earth_enterprise_packet`] (same as the metadata
//!    module).
//! 6. JS accepts `Credit|string` for `options.credit`; the Rust port takes an
//!    [`Option<Credit>`] (construct strings via [`Credit::new`]).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::check;
use crate::credit::Credit;
use crate::decode_google_earth_enterprise_packet::{
    decode_google_earth_enterprise_packet, GeePacketResult, GeePacketType,
};
use crate::ellipsoid::Ellipsoid;
use crate::event::Event;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::google_earth_enterprise_metadata::GoogleEarthEnterpriseMetadata;
use crate::google_earth_enterprise_terrain_data::{
    GoogleEarthEnterpriseTerrainData, GoogleEarthEnterpriseTerrainDataOptions,
};
use crate::google_earth_enterprise_tile_information::GoogleEarthEnterpriseTileInformation;
use crate::heightmap_terrain_data::{
    HeightmapBuffer, HeightmapBufferType, HeightmapTerrainData, HeightmapTerrainDataOptions,
};
use crate::math::CesiumMath;
use crate::rectangle::Rectangle;
use crate::resource::{DerivedResourceOptions, Resource, ResourceBackend};
use crate::runtime_error::RuntimeError;
use crate::terrain_provider::TerrainProvider;
use crate::tiling_scheme::TilingScheme;

/// Mirrors the module-level `TerrainState`.
pub mod TerrainState {
    /// Terrain availability not yet known.
    pub const UNKNOWN: u32 = 0;
    /// No terrain available.
    pub const NONE: u32 = 1;
    /// Terrain available on this tile itself.
    pub const SELF: u32 = 2;
    /// Terrain available on the parent tile.
    pub const PARENT: u32 = 3;
}

/// The tile data returned by
/// [`GoogleEarthEnterpriseTerrainProvider::request_tile_geometry`].
///
/// DEVIATION: JS returns `GoogleEarthEnterpriseTerrainData` or
/// `HeightmapTerrainData` polymorphically; Rust models the two cases as an
/// enum.
pub enum GoogleEarthEnterpriseTerrainTileData {
    /// A real GEE terrain tile.
    Google(GoogleEarthEnterpriseTerrainData),
    /// A flat ellipsoid heightmap (ancestors have no terrain yet).
    Heightmap(HeightmapTerrainData),
}

impl std::fmt::Debug for GoogleEarthEnterpriseTerrainTileData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Google(_) => write!(f, "GoogleEarthEnterpriseTerrainTileData::Google(..)"),
            Self::Heightmap(_) => write!(f, "GoogleEarthEnterpriseTerrainTileData::Heightmap(..)"),
        }
    }
}

/// Mirrors `TerrainCache` (module DEVIATION 3).
pub struct TerrainCache {
    terrain_cache: HashMap<String, (Vec<u8>, Instant)>,
    last_tidy: Instant,
}

impl TerrainCache {
    fn new() -> Self {
        Self {
            terrain_cache: HashMap::new(),
            last_tidy: Instant::now(),
        }
    }

    /// Mirrors `TerrainCache.prototype.add`.
    pub fn add(&mut self, quad_key: &str, buffer: Vec<u8>) {
        self.terrain_cache
            .insert(quad_key.to_string(), (buffer, Instant::now()));
    }

    /// Mirrors `TerrainCache.prototype.get` (removes the entry on hit).
    pub fn get(&mut self, quad_key: &str) -> Option<Vec<u8>> {
        self.terrain_cache.remove(quad_key).map(|(buffer, _)| buffer)
    }

    /// Mirrors `TerrainCache.prototype.tidy`.
    pub fn tidy(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_tidy) > Duration::from_secs(10) {
            self.terrain_cache
                .retain(|_, (_, timestamp)| now.duration_since(*timestamp) <= Duration::from_secs(10));
            self.last_tidy = now;
        }
    }
}

/// Mirrors `GoogleEarthEnterpriseTerrainProvider.ConstructorOptions`.
#[derive(Default)]
pub struct GoogleEarthEnterpriseTerrainProviderOptions {
    /// The ellipsoid. If not specified, the default ellipsoid is used.
    pub ellipsoid: Option<Ellipsoid>,
    /// A credit for the data source, which is displayed on the canvas
    /// (DEVIATION 6).
    pub credit: Option<Credit>,
}

/// Provides tiled terrain using the Google Earth Enterprise REST API.
///
/// To construct a provider, call
/// [`GoogleEarthEnterpriseTerrainProvider::from_metadata`] (mirrors the JS
/// "do not call the constructor directly" notice).
pub struct GoogleEarthEnterpriseTerrainProvider {
    tiling_scheme: GeographicTilingScheme,
    credit: Option<Credit>,
    /// Pulled from Google's documentation.
    level_zero_maximum_geometric_error: f64,
    terrain_cache: TerrainCache,
    error_event: Event,
    metadata: GoogleEarthEnterpriseMetadata,
}

impl GoogleEarthEnterpriseTerrainProvider {
    /// Mirrors the JS constructor (private; use [`Self::from_metadata`]).
    fn new_internal(options: Option<GoogleEarthEnterpriseTerrainProviderOptions>) -> Self {
        let options = options.unwrap_or_default();

        let tiling_scheme = GeographicTilingScheme::new(
            options.ellipsoid,
            Some(Rectangle::new(
                -CesiumMath::PI,
                -CesiumMath::PI,
                CesiumMath::PI,
                CesiumMath::PI,
            )),
            Some(2),
            Some(2),
        );

        Self {
            tiling_scheme,
            credit: options.credit,
            level_zero_maximum_geometric_error: 40075.16,
            terrain_cache: TerrainCache::new(),
            error_event: Event::new(),
            metadata: GoogleEarthEnterpriseMetadata::new(Resource::new(String::new())),
        }
    }

    /// Creates a GoogleEarthTerrainProvider from GoogleEarthEnterpriseMetadata.
    ///
    /// Mirrors `GoogleEarthEnterpriseTerrainProvider.fromMetadata`.
    ///
    /// # Panics
    ///
    /// Debug builds panic when `metadata` is undefined (mirrors `Check.defined`).
    pub fn from_metadata(
        metadata: Option<GoogleEarthEnterpriseMetadata>,
        options: Option<GoogleEarthEnterpriseTerrainProviderOptions>,
    ) -> Result<Self, RuntimeError> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::defined("metadata", metadata.as_ref());
        }
        //>>includeEnd('debug');
        let metadata = metadata.unwrap();

        if !metadata.terrain_present {
            return Err(RuntimeError::new(Some(&format!(
                "The server {} doesn't have terrain",
                metadata.url()
            ))));
        }

        let mut provider = Self::new_internal(options);
        provider.metadata = metadata;

        Ok(provider)
    }

    /// Gets the name of the Google Earth Enterprise server url hosting the
    /// imagery.
    pub fn url(&self) -> String {
        self.metadata.url()
    }

    /// Gets the tiling scheme used by this provider.
    pub fn tiling_scheme(&self) -> &GeographicTilingScheme {
        &self.tiling_scheme
    }

    /// Gets an event that is raised when the imagery provider encounters an
    /// asynchronous error.
    pub fn error_event(&self) -> &Event {
        &self.error_event
    }

    /// Gets the credit to display when this terrain provider is active.
    pub fn credit(&self) -> Option<&Credit> {
        self.credit.as_ref()
    }

    /// Gets a value indicating whether or not the provider includes a water
    /// mask. Always false.
    pub fn has_water_mask(&self) -> bool {
        false
    }

    /// Gets a value indicating whether or not the requested tiles include
    /// vertex normals. Always false.
    pub fn has_vertex_normals(&self) -> bool {
        false
    }

    /// Gets an object that can be used to determine availability of terrain
    /// from this provider. Always `None` (JS `undefined`).
    pub fn availability(&self) -> Option<()> {
        None
    }

    /// The shared metadata object (mirrors `_metadata`).
    pub fn metadata(&self) -> &GoogleEarthEnterpriseMetadata {
        &self.metadata
    }

    /// Gets the maximum geometric error allowed in a tile at a given level.
    ///
    /// Mirrors `getLevelMaximumGeometricError(level)`.
    pub fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        self.level_zero_maximum_geometric_error / (1 << level) as f64
    }

    /// Requests the geometry for a given tile.
    ///
    /// Mirrors `requestTileGeometry(x, y, level, request)` (DEVIATIONS 1/2/5).
    /// Returns `Ok(None)` when the backend reports throttling (JS returns
    /// `undefined`).
    pub async fn request_tile_geometry<B: ResourceBackend + ?Sized>(
        &mut self,
        x: i32,
        y: i32,
        level: i32,
        backend: &B,
    ) -> Result<Option<GoogleEarthEnterpriseTerrainTileData>, RuntimeError> {
        let quad_key = GoogleEarthEnterpriseMetadata::tile_xy_to_quad_key(x, y, level);
        let info = self
            .metadata
            .get_tile_information_from_quad_key(&quad_key)
            .and_then(|i| i);

        // Check if this tile is even possibly available
        let Some(info) = info else {
            return Err(RuntimeError::new(Some("Terrain tile doesn't exist")));
        };

        let mut terrain_state = info.terrain_state;
        if terrain_state.is_none() {
            // First time we have tried to load this tile, so set terrain
            // state to UNKNOWN
            terrain_state = Some(TerrainState::UNKNOWN);
            set_terrain_state(&self.metadata, &quad_key, Some(TerrainState::UNKNOWN));
        }
        let terrain_state = terrain_state.unwrap();

        // If its in the cache, return it
        if let Some(buffer) = self.terrain_cache.get(&quad_key) {
            return Ok(Some(build_terrain_data(
                buffer,
                &quad_key,
                &info,
                &self.metadata,
            )));
        }

        // Clean up the cache
        self.terrain_cache.tidy();

        // We have a tile, check to see if no ancestors have terrain or that
        // we know for sure it doesn't
        if !info.ancestor_has_terrain {
            // We haven't reached a level with terrain, so return the
            // ellipsoid
            return Ok(Some(GoogleEarthEnterpriseTerrainTileData::Heightmap(
                HeightmapTerrainData::new(HeightmapTerrainDataOptions {
                    buffer: Some(HeightmapBuffer::zeroed(HeightmapBufferType::Uint8, 16 * 16)),
                    width: Some(16),
                    height: Some(16),
                    ..Default::default()
                }),
            )));
        } else if terrain_state == TerrainState::NONE {
            // Already have info and there isn't any terrain here
            return Err(RuntimeError::new(Some("Terrain tile doesn't exist")));
        }

        // Figure out where we are getting the terrain and what version
        let mut q = quad_key.clone();
        let mut terrain_version: i32 = -1;
        match terrain_state {
            TerrainState::SELF => {
                // We have terrain and have retrieved it before
                terrain_version = info.terrain_version as i32;
            }
            TerrainState::PARENT => {
                // We have terrain in our parent
                q.pop();
                if let Some(Some(parent_info)) =
                    self.metadata.get_tile_information_from_quad_key(&q)
                {
                    terrain_version = parent_info.terrain_version as i32;
                }
            }
            TerrainState::UNKNOWN => {
                // We haven't tried to retrieve terrain yet
                if info.has_terrain() {
                    terrain_version = info.terrain_version as i32; // We should have terrain
                } else {
                    q.pop();
                    if let Some(Some(parent_info)) =
                        self.metadata.get_tile_information_from_quad_key(&q)
                    {
                        if parent_info.has_terrain() {
                            // Try checking in the parent
                            terrain_version = parent_info.terrain_version as i32;
                        }
                    }
                }
            }
            _ => {}
        }

        // We can't figure out where to get the terrain
        if terrain_version < 0 {
            return Err(RuntimeError::new(Some("Terrain tile doesn't exist")));
        }

        // Load that terrain (DEVIATION 2: no shared-promise deduplication).
        let load_result: Result<Option<GoogleEarthEnterpriseTerrainTileData>, RuntimeError> =
            async {
                let mut resource = build_terrain_resource(&self.metadata, &q, terrain_version);
                let terrain = resource
                    .fetch_array_buffer(backend)
                    .await
                    .map_err(|e| RuntimeError::new(Some(&format!("{e}"))))?;
                let Some(terrain) = terrain else {
                    // Throttled (JS returns undefined)
                    return Ok(None);
                };

            let key = self.metadata.key.clone().unwrap_or_default();
            let mut buffer = terrain;
            let terrain_tiles = match decode_google_earth_enterprise_packet(
                &key,
                &mut buffer,
                GeePacketType::Terrain,
                &q,
            ) {
                Ok(GeePacketResult::Terrain(tiles)) => tiles,
                Ok(_) => return Err(RuntimeError::new(Some("Failed to load terrain."))),
                Err(e) => return Err(RuntimeError::new(Some(&e.message))),
            };

            // Add requested tile and mark it as SELF
            let requested_info = self
                .metadata
                .get_tile_information_from_quad_key(&q)
                .and_then(|i| i)
                .ok_or_else(|| RuntimeError::new(Some("Failed to load terrain.")))?;
            set_terrain_state(&self.metadata, &q, Some(TerrainState::SELF));
            let provider = requested_info.terrain_provider;
            let mut tiles = terrain_tiles;
            if tiles.is_empty() {
                return Err(RuntimeError::new(Some("Failed to load terrain.")));
            }
            self.terrain_cache.add(&q, tiles.remove(0));

            // Add children to cache
            for j in 0..tiles.len() {
                let child_key = format!("{q}{j}");
                let child = self
                    .metadata
                    .get_tile_information_from_quad_key(&child_key)
                    .and_then(|i| i);
                if child.is_some() {
                    self.terrain_cache.add(&child_key, tiles[j].clone());
                    update_tile_info(&self.metadata, &child_key, |child| {
                        child.terrain_state = Some(TerrainState::PARENT);
                        if child.terrain_provider == 0 {
                            child.terrain_provider = provider;
                        }
                    });
                }
            }

            let buffer = self
                .terrain_cache
                .get(&quad_key)
                .ok_or_else(|| RuntimeError::new(Some("Failed to load terrain.")))?;
            let info = self
                .metadata
                .get_tile_information_from_quad_key(&quad_key)
                .and_then(|i| i)
                .ok_or_else(|| RuntimeError::new(Some("Failed to load terrain.")))?;
            Ok(Some(build_terrain_data(
                buffer,
                &quad_key,
                &info,
                &self.metadata,
            )))
        }
        .await;

        match load_result {
            Ok(data) => Ok(data),
            Err(error) => {
                set_terrain_state(&self.metadata, &quad_key, Some(TerrainState::NONE));
                Err(error)
            }
        }
    }

    /// Determines whether data for a tile is available to be loaded
    /// (synchronous flavor; DEVIATION 4: the JS `populateSubtree` kick is not
    /// issued here, but the `false` answer for the not-yet-known case is
    /// still reported).
    pub fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool> {
        let (available, _) = self.get_tile_data_available_core(x, y, level);
        Some(available.unwrap_or(false))
    }

    /// Full JS `getTileDataAvailable` behavior: when the tile is not yet
    /// known but valid, requests the subtree metadata (awaited inline,
    /// DEVIATION 1) and reports `false` for now.
    pub async fn get_tile_data_available_async<B: ResourceBackend + ?Sized>(
        &self,
        x: i32,
        y: i32,
        level: i32,
        backend: &B,
    ) -> Result<bool, RuntimeError> {
        let (available, needs_populate) = self.get_tile_data_available_core(x, y, level);
        if let Some(available) = available {
            return Ok(available);
        }

        if needs_populate {
            // We will need this tile, so request metadata and return false
            // for now (DEVIATION 1: JS fires the request without awaiting).
            self.metadata.populate_subtree_xy(x, y, level, backend).await?;
        }
        Ok(false)
    }

    /// Shared body of `getTileDataAvailable`. Returns `(known answer,
    /// needs populateSubtree kick)`.
    fn get_tile_data_available_core(
        &self,
        x: i32,
        y: i32,
        level: i32,
    ) -> (Option<bool>, bool) {
        let quad_key = GoogleEarthEnterpriseMetadata::tile_xy_to_quad_key(x, y, level);

        let info = self.metadata.get_tile_information(x, y, level);
        // JS `info === null` → false
        if matches!(info, Some(None)) {
            return (Some(false), false);
        }

        if let Some(Some(info)) = info {
            if !info.ancestor_has_terrain {
                return (Some(true), false); // We'll just return the ellipsoid
            }

            let terrain_state = info.terrain_state;
            if terrain_state == Some(TerrainState::NONE) {
                return (Some(false), false); // Terrain is not available
            }

            if terrain_state.is_none() || terrain_state == Some(TerrainState::UNKNOWN) {
                set_terrain_state(&self.metadata, &quad_key, Some(TerrainState::UNKNOWN));
                if !info.has_terrain() {
                    let parent_key = quad_key[..quad_key.len() - 1].to_string();
                    let parent_info = self
                        .metadata
                        .get_tile_information_from_quad_key(&parent_key)
                        .and_then(|i| i);
                    if parent_info.is_none() || !parent_info.as_ref().unwrap().has_terrain() {
                        return (Some(false), false);
                    }
                }
            }

            return (Some(true), false);
        }

        // `info === undefined`
        let needs_populate = self.metadata.is_valid(&quad_key);
        (None, needs_populate)
    }

    /// Makes sure we load availability data for a tile.
    ///
    /// Mirrors `loadTileDataAvailability` (always returns `undefined`).
    pub fn load_tile_data_availability(&self, _x: i32, _y: i32, _level: i32) {}
}

impl TerrainProvider for GoogleEarthEnterpriseTerrainProvider {
    fn tiling_scheme(&self) -> &dyn TilingScheme {
        &self.tiling_scheme
    }

    fn has_water_mask(&self) -> bool {
        GoogleEarthEnterpriseTerrainProvider::has_water_mask(self)
    }

    fn has_vertex_normals(&self) -> bool {
        GoogleEarthEnterpriseTerrainProvider::has_vertex_normals(self)
    }

    fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        GoogleEarthEnterpriseTerrainProvider::get_level_maximum_geometric_error(self, level)
    }

    fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool> {
        GoogleEarthEnterpriseTerrainProvider::get_tile_data_available(self, x, y, level)
    }
}

// If the tile has its own terrain, then you can just use its child bitmask.
// If it was requested using its parent then you need to check all of its
// children to see if they have terrain.
//
// Mirrors `computeChildMask(quadKey, info, metadata)`.
fn compute_child_mask(
    quad_key: &str,
    info: &GoogleEarthEnterpriseTileInformation,
    metadata: &GoogleEarthEnterpriseMetadata,
) -> u32 {
    let mut child_mask = info.get_child_bitmask();
    if info.terrain_state == Some(TerrainState::PARENT) {
        child_mask = 0;
        for i in 0..4 {
            let child = metadata
                .get_tile_information_from_quad_key(&format!("{quad_key}{i}"))
                .and_then(|c| c);
            if let Some(child) = child {
                if child.has_terrain() {
                    child_mask |= 1 << i;
                }
            }
        }
    }

    child_mask
}

/// Builds the `GoogleEarthEnterpriseTerrainData` for a cached buffer
/// (mirrors the two JS `new GoogleEarthEnterpriseTerrainData({...})` sites).
fn build_terrain_data(
    buffer: Vec<u8>,
    quad_key: &str,
    info: &GoogleEarthEnterpriseTileInformation,
    metadata: &GoogleEarthEnterpriseMetadata,
) -> GoogleEarthEnterpriseTerrainTileData {
    let credit = metadata.providers.get(&info.terrain_provider).cloned();
    GoogleEarthEnterpriseTerrainTileData::Google(GoogleEarthEnterpriseTerrainData::new(
        GoogleEarthEnterpriseTerrainDataOptions {
            buffer: Some(buffer),
            child_tile_mask: Some(compute_child_mask(quad_key, info, metadata)),
            credits: credit.map(|c| vec![c]),
            negative_altitude_exponent_bias: Some(metadata.negative_altitude_exponent_bias as f64),
            negative_elevation_threshold: Some(metadata.negative_altitude_threshold),
            ..Default::default()
        },
    ))
}

/// Mirrors `buildTerrainResource(terrainProvider, quadKey, version, request)`.
fn build_terrain_resource(
    metadata: &GoogleEarthEnterpriseMetadata,
    quad_key: &str,
    version: i32,
) -> Resource {
    let version = if version > 0 { version } else { 1 };
    metadata
        .resource
        .clone_resource()
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(&format!("flatfile?f1c-0{quad_key}-t.{version}")),
            ..Default::default()
        })
}

/// Writes `terrainState` back into the shared metadata tile info (JS mutates
/// the shared object in place).
fn set_terrain_state(
    metadata: &GoogleEarthEnterpriseMetadata,
    quad_key: &str,
    state: Option<u32>,
) {
    update_tile_info(metadata, quad_key, |info| info.terrain_state = state);
}

/// Applies a mutation to the shared tile info entry for `quad_key` (no-op
/// when the entry is missing or null, mirroring JS `defined` guards).
fn update_tile_info(
    metadata: &GoogleEarthEnterpriseMetadata,
    quad_key: &str,
    f: impl FnOnce(&mut GoogleEarthEnterpriseTileInformation),
) {
    if let Some(Some(info)) = metadata.tile_info.borrow_mut().get_mut(quad_key) {
        f(info);
    }
}
