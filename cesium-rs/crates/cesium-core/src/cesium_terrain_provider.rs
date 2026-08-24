//! Ported from `packages/engine/Source/Core/CesiumTerrainProvider.js` (1409 lines).
//!
//! A [`TerrainProvider`] that accesses terrain data in a Cesium terrain format
//! (quantized-mesh or heightmap-1.0) described by a `layer.json` document.
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `LayerInformation` | [`LayerInformation`] | `availabilityPromiseCache` not modeled (see DEVIATION 4) |
//! | `TerrainProviderBuilder` | [`TerrainProviderBuilder`] | identical field set (minus JS-only plumbing) |
//! | `TerrainProviderBuilder.prototype.build` | [`TerrainProviderBuilder::build`] | identical |
//! | `parseMetadataSuccess` | [`parse_metadata_success`] | identical logic incl. y-flip and parentUrl recursion |
//! | `parseMetadataFailure` | [`parse_metadata_failure`] | retry flow not modeled (`TileProviderError.retry` is always false) |
//! | `metadataSuccess` | [`metadata_success`] | identical |
//! | `requestLayerJson` | [`request_layer_json`] | 404 → default heightmap metadata fallback preserved |
//! | `CesiumTerrainProvider` constructor | [`CesiumTerrainProvider::with_options`] | identical option defaults |
//! | `QuantizedMeshExtensionIds` | [`QUANTIZED_MESH_EXTENSION_IDS`] | identical constants |
//! | `getRequestHeader` | [`get_request_header`] | identical Accept strings |
//! | `createHeightmapTerrainData` | [`CesiumTerrainProvider::create_heightmap_terrain_data`] | identical layout |
//! | `createQuantizedMeshTerrainData` | [`CesiumTerrainProvider::create_quantized_mesh_terrain_data`] | identical binary layout incl. extensions loop |
//! | `requestTileGeometry` (prototype + free fn) | [`CesiumTerrainProvider::request_tile_geometry`] / [`CesiumTerrainProvider::request_tile_geometry_from_layer`] | DEVIATION 2/3 |
//! | properties (`errorEvent`, `credit`, `tilingScheme`, `hasWaterMask`, ...) | accessor methods | `hasWaterMask` = `_hasWaterMask && _requestWaterMask` etc. |
//! | `getLevelMaximumGeometricError` | [`CesiumTerrainProvider::get_level_maximum_geometric_error`] | identical |
//! | `fromIonAssetId` | [`CesiumTerrainProvider::from_ion_asset_id`] | DEVIATION 5 |
//! | `fromUrl` | [`CesiumTerrainProvider::from_url`] / [`CesiumTerrainProvider::from_resource`] / [`CesiumTerrainProvider::from_ion_resource`] | DEVIATION 1/5 |
//! | `getTileDataAvailable` | [`CesiumTerrainProvider::get_tile_data_available`] | identical |
//! | `loadTileDataAvailability` | [`CesiumTerrainProvider::load_tile_data_availability`] | DEVIATION 4 |
//! | `getAvailabilityTile` | [`get_availability_tile`] | takes `availabilityLevels` directly (JS takes the layer object) |
//! | `checkLayer` | [`CesiumTerrainProvider::check_layer`] | DEVIATION 4 |
//! | `CesiumTerrainProvider._getAvailabilityTile` | [`get_availability_tile`] | exported for tests, same as JS |
//!
//! # DEVIATIONS
//!
//! 1. HTTP access goes through the injected [`ResourceBackend`]
//!    (native reqwest / WASM fetch / mock for specs) instead of the global
//!    `loadWithXhr`; `fromUrl` accepts a `Resource` or `&str` (JS also
//!    accepts promises, which Rust callers resolve before the call).
//! 2. `RequestScheduler` throttling is not applied: JS `requestTileGeometry`
//!    may return `undefined` when too many requests are pending; the Rust
//!    port always issues the request.
//! 3. The JS deferred-retry path of `requestTileGeometry` (multi-layer with
//!    not-yet-loaded availability, retried via `setTimeout`) returns a
//!    `RuntimeError` instead of re-scheduling.
//! 4. `checkLayer` never issues availability-tile requests for non-top layers
//!    (JS caches them in `LayerInformation.availabilityPromiseCache`); the
//!    `result` boolean is still computed identically, so
//!    `getTileDataAvailable` semantics match; `loadTileDataAvailability`
//!    therefore always returns `None`.
//! 5. `fromIonAssetId`/ion resources: endpoint-credit propagation
//!    (`resource.credits`) and the token-refresh retry callback are not
//!    ported; ion extension negotiation (query `extensions=` vs Accept
//!    header) is preserved.
//! 6. `QuantizedMeshTerrainData` receives no `center` option (not modeled in
//!    the Rust port) and terrain-data `credits` are passed as HTML strings.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::attribute_compression::AttributeCompression;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::check;
use crate::credit::Credit;
use crate::ellipsoid::Ellipsoid;
use crate::event::Event;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::heightmap_terrain_data::{
    HeightmapBuffer, HeightmapStructureOptions, HeightmapTerrainData, HeightmapTerrainDataOptions,
};
use crate::heightmap_tessellator::HeightmapStructure;
use crate::ion_resource::IonResource;
use crate::oriented_bounding_box::OrientedBoundingBox;
use crate::quantized_mesh_terrain_data::{
    QuantizedMeshTerrainData, QuantizedMeshTerrainDataOptions,
};
use crate::rectangle::Rectangle;
use crate::resource::{
    DerivedResourceOptions, Resource, ResourceBackend, ResourceError,
};
use crate::runtime_error::RuntimeError;
use crate::terrain_provider::{
    get_estimated_level_zero_geometric_error_for_a_heightmap, TerrainProvider,
};
use crate::tile_availability::TileAvailability;
use crate::tile_provider_error::TileProviderError;
use crate::tiling_scheme::TilingScheme;
use crate::web_mercator_tiling_scheme::WebMercatorTilingScheme;

/// Identifiers for quantized-mesh tile extensions.
///
/// Mirrors the private `QuantizedMeshExtensionIds` object.
pub const QUANTIZED_MESH_EXTENSION_IDS: QuantizedMeshExtensionIds = QuantizedMeshExtensionIds {
    oct_vertex_normals: 1,
    water_mask: 2,
    metadata: 4,
};

/// Mirrors `QuantizedMeshExtensionIds`.
pub struct QuantizedMeshExtensionIds {
    /// Oct-encoded per-vertex normals extension.
    pub oct_vertex_normals: u8,
    /// Water mask extension.
    pub water_mask: u8,
    /// JSON metadata extension.
    pub metadata: u8,
}

/// Which tiling scheme the `layer.json` projection selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TilingSchemeKind {
    /// `EPSG:4326` (or unspecified): 2×1 level-zero tiles.
    Geographic,
    /// `EPSG:3857`: 1×1 level-zero tiles.
    WebMercator,
}

impl TilingSchemeKind {
    fn make(&self, ellipsoid: Ellipsoid) -> Box<dyn TilingScheme> {
        match self {
            Self::Geographic => Box::new(GeographicTilingScheme::new(
                Some(ellipsoid),
                None,
                Some(2),
                Some(1),
            )),
            Self::WebMercator => Box::new(WebMercatorTilingScheme::new(
                Some(ellipsoid),
                Some(1),
                Some(1),
                None,
                None,
            )),
        }
    }
}

/// Information about one layer of a terrain tileset.
///
/// Mirrors the private `LayerInformation` constructor.
pub struct LayerInformation {
    /// The resource used to fetch the layer's tiles.
    pub resource: Resource,
    /// The layer version (substituted into `{version}` templates).
    pub version: Option<String>,
    /// True if this layer uses the heightmap-1.0 format.
    pub is_heightmap: bool,
    /// The tile URL templates from `layer.json`.
    pub tile_url_templates: Vec<String>,
    /// The tile availability of this layer, if reported.
    pub availability: Option<TileAvailability>,
    /// Whether the layer provides vertex normals.
    pub has_vertex_normals: bool,
    /// Whether the layer provides a water mask.
    pub has_water_mask: bool,
    /// Whether the layer provides metadata.
    pub has_metadata: bool,
    /// The `metadataAvailability` value of the layer, if any.
    pub availability_levels: Option<i32>,
    /// Tracks which availability-containing tiles have been loaded.
    pub availability_tiles_loaded: Option<TileAvailability>,
    /// False for the legacy big-endian `vertexnormals` extension.
    pub little_endian_extension_size: bool,
    /// True when the layer resource is an ion endpoint without an external
    /// type (extensions are requested via query parameters). DEVIATION: JS
    /// checks `resource._ionEndpoint` at request time.
    pub uses_ion_query: bool,
}

/// The tile whose metadata extension contains availability information for a
/// given tile. Mirrors the `{level, x, y}` object of `getAvailabilityTile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvailabilityTile {
    /// The level of the availability tile.
    pub level: i32,
    /// The x coordinate of the availability tile.
    pub x: i32,
    /// The y coordinate of the availability tile.
    pub y: i32,
}

/// Computes the availability tile containing tile `(x, y, level)`.
///
/// Mirrors `getAvailabilityTile(layer, x, y, level)` (exported in JS as
/// `CesiumTerrainProvider._getAvailabilityTile`); the JS `layer` argument is
/// only used for `layer.availabilityLevels`, which is passed directly here.
pub fn get_availability_tile(
    availability_levels: i32,
    x: i32,
    y: i32,
    level: i32,
) -> Option<AvailabilityTile> {
    if level == 0 {
        return None;
    }

    let parent_level = if level % availability_levels == 0 {
        level - availability_levels
    } else {
        (level / availability_levels) * availability_levels
    };
    let divisor = 1i32 << (level - parent_level);
    let parent_x = x / divisor;
    let parent_y = y / divisor;

    Some(AvailabilityTile {
        level: parent_level,
        x: parent_x,
        y: parent_y,
    })
}

/// Initialization options for [`CesiumTerrainProvider`].
///
/// Mirrors `CesiumTerrainProvider.ConstructorOptions`.
#[derive(Default, Clone)]
pub struct CesiumTerrainProviderOptions {
    /// Request per-vertex normals from the server, if available.
    pub request_vertex_normals: Option<bool>,
    /// Request per-tile water masks from the server, if available.
    pub request_water_mask: Option<bool>,
    /// Request per-tile metadata from the server, if available.
    pub request_metadata: Option<bool>,
    /// The ellipsoid (defaults to WGS84).
    pub ellipsoid: Option<Ellipsoid>,
    /// A credit for the data source.
    pub credit: Option<String>,
}

/// The terrain data returned by [`CesiumTerrainProvider::request_tile_geometry`].
///
/// DEVIATION: JS returns `HeightmapTerrainData` or `QuantizedMeshTerrainData`
/// polymorphically; Rust models the two cases as an enum.
pub enum TerrainTileData {
    /// A heightmap-1.0 tile.
    Heightmap(HeightmapTerrainData),
    /// A quantized-mesh tile.
    QuantizedMesh(QuantizedMeshTerrainData),
}

impl std::fmt::Debug for TerrainTileData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Heightmap(_) => write!(f, "TerrainTileData::Heightmap(..)"),
            Self::QuantizedMesh(_) => write!(f, "TerrainTileData::QuantizedMesh(..)"),
        }
    }
}

/// Used to track creation details while fetching initial metadata.
///
/// Mirrors the private `TerrainProviderBuilder` constructor.
struct TerrainProviderBuilder {
    // Mirrors the JS builder fields; `requestVertexNormals`/`requestMetadata`
    // are carried for parity but only `requestWaterMask` is re-applied by
    // `build` (the heightmap-1.0 branch sets it to true).
    #[allow(dead_code)]
    request_vertex_normals: bool,
    request_water_mask: bool,
    #[allow(dead_code)]
    request_metadata: bool,
    ellipsoid: Ellipsoid,

    heightmap_width: i32,
    heightmap_structure: Option<HeightmapStructure>,
    has_water_mask: bool,
    has_metadata: bool,
    has_vertex_normals: bool,
    scheme: Option<String>,

    last_resource: Resource,
    layer_json_resource: Resource,
    previous_error: Option<TileProviderError>,
    availability: Option<TileAvailability>,
    tiling_scheme_kind: Option<TilingSchemeKind>,
    level_zero_maximum_geometric_error: f64,
    layers: Vec<LayerInformation>,
    attribution: String,
    /// Per-level list of `[startX, startY, endX, endY]` availability ranges
    /// (JS sparse array `overallAvailability`).
    overall_availability: Vec<Vec<[i32; 4]>>,
    /// Mirrors the JS `overallMaxZoom`. `None` mirrors the JS `NaN` state
    /// (`Math.max(0, undefined)` when no layer.json provides `maxzoom`):
    /// `level > NaN` is always false, so no max-zoom bound applies.
    overall_max_zoom: Option<i32>,
    tile_credits: Vec<Credit>,
    /// See [`LayerInformation::uses_ion_query`].
    uses_ion_query: bool,
}

impl TerrainProviderBuilder {
    fn new(
        options: &CesiumTerrainProviderOptions,
        last_resource: Resource,
        layer_json_resource: Resource,
        uses_ion_query: bool,
    ) -> Self {
        Self {
            request_vertex_normals: options.request_vertex_normals.unwrap_or(false),
            request_water_mask: options.request_water_mask.unwrap_or(false),
            request_metadata: options.request_metadata.unwrap_or(true),
            ellipsoid: options.ellipsoid.unwrap_or(Ellipsoid::WGS84),
            heightmap_width: 65,
            heightmap_structure: None,
            has_water_mask: false,
            has_metadata: false,
            has_vertex_normals: false,
            scheme: None,
            last_resource,
            layer_json_resource,
            previous_error: None,
            availability: None,
            tiling_scheme_kind: None,
            level_zero_maximum_geometric_error: 0.0,
            layers: Vec::new(),
            attribution: String::new(),
            overall_availability: Vec::new(),
            overall_max_zoom: None,
            tile_credits: Vec::new(),
            uses_ion_query,
        }
    }

    /// Completes provider creation based on builder values.
    ///
    /// Mirrors `TerrainProviderBuilder.prototype.build(provider)`.
    fn build(self, provider: &mut CesiumTerrainProvider) {
        provider.heightmap_width = self.heightmap_width;
        provider.scheme = self.scheme;

        // DEVIATION: ion resources have a `credits` property JS concatenates
        // here; endpoint credits are not modeled in the Rust port.
        provider.tile_credits = self.tile_credits;
        provider.availability = self.availability;
        provider.tiling_scheme = self
            .tiling_scheme_kind
            .map(|kind| kind.make(self.ellipsoid))
            .unwrap_or_else(|| {
                Box::new(GeographicTilingScheme::new(None, None, None, None))
                    as Box<dyn TilingScheme>
            });
        provider.request_water_mask = self.request_water_mask;
        provider.level_zero_maximum_geometric_error = self.level_zero_maximum_geometric_error;
        provider.heightmap_structure = self.heightmap_structure;
        provider.layers = self.layers;

        provider.has_water_mask = self.has_water_mask;
        provider.has_vertex_normals = self.has_vertex_normals;
        provider.has_metadata = self.has_metadata;
    }
}

/// A [`TerrainProvider`] that accesses terrain data in a Cesium terrain
/// format (quantized-mesh or heightmap-1.0).
///
/// Use [`from_url`](Self::from_url), [`from_resource`](Self::from_resource),
/// [`from_ion_resource`](Self::from_ion_resource) or
/// [`from_ion_asset_id`](Self::from_ion_asset_id) to construct. Mirrors
/// CesiumJS `CesiumTerrainProvider` (1409 lines).
pub struct CesiumTerrainProvider {
    heightmap_width: i32,
    heightmap_structure: Option<HeightmapStructure>,
    has_water_mask: bool,
    has_vertex_normals: bool,
    has_metadata: bool,
    scheme: Option<String>,
    ellipsoid: Ellipsoid,

    request_vertex_normals: bool,
    request_water_mask: bool,
    request_metadata: bool,

    error_event: Event,
    credit: Option<Credit>,

    availability: Option<TileAvailability>,
    tiling_scheme: Box<dyn TilingScheme>,
    level_zero_maximum_geometric_error: f64,
    layers: Vec<LayerInformation>,
    tile_credits: Vec<Credit>,
}

impl CesiumTerrainProvider {
    /// Creates a new, unconfigured CesiumTerrainProvider.
    ///
    /// DEVIATION: JS forbids calling the constructor directly (use
    /// `fromUrl`/`fromIonAssetId`); the Rust port keeps a default
    /// construction path (geographic tiling scheme, no layers) so the type
    /// remains usable in generic contexts.
    pub fn new() -> Self {
        Self::with_options(&CesiumTerrainProviderOptions::default())
    }

    /// Mirrors the JS constructor body (`options = options ?? {}`).
    fn with_options(options: &CesiumTerrainProviderOptions) -> Self {
        let credit = options.credit.as_ref().map(|c| Credit::new(c, false));
        Self {
            heightmap_width: 0,
            heightmap_structure: None,
            has_water_mask: false,
            has_vertex_normals: false,
            has_metadata: false,
            scheme: None,
            ellipsoid: options.ellipsoid.unwrap_or(Ellipsoid::WGS84),
            request_vertex_normals: options.request_vertex_normals.unwrap_or(false),
            request_water_mask: options.request_water_mask.unwrap_or(false),
            request_metadata: options.request_metadata.unwrap_or(true),
            error_event: Event::new(),
            credit,
            availability: None,
            tiling_scheme: Box::new(GeographicTilingScheme::new(None, None, None, None)),
            level_zero_maximum_geometric_error: 0.0,
            layers: Vec::new(),
            tile_credits: Vec::new(),
        }
    }

    /// Creates a provider from a Cesium ion asset ID.
    ///
    /// Mirrors `CesiumTerrainProvider.fromIonAssetId(assetId, options)`.
    ///
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `asset_id` is
    /// `None` (JS `Check.defined("assetId", assetId)`).
    pub async fn from_ion_asset_id<B: ResourceBackend + ?Sized>(
        asset_id: Option<u64>,
        options: Option<CesiumTerrainProviderOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::defined("assetId", asset_id.as_ref());
        }
        //>>includeEnd('debug');

        let ion = IonResource::from_asset_id(asset_id.unwrap(), None, backend)
            .await
            .map_err(|e| RuntimeError::new(Some(&e.to_string())))?;
        Self::from_ion_resource(ion, options, backend).await
    }

    /// Creates a provider from a URL string.
    ///
    /// Mirrors `CesiumTerrainProvider.fromUrl(url, options)` for the string
    /// case (DEVIATION: promises are resolved by the caller; the `Resource`
    /// case is [`from_resource`](Self::from_resource)).
    ///
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `url` is `None`
    /// (JS `Check.defined("url", url)`).
    pub async fn from_url<B: ResourceBackend + ?Sized>(
        url: Option<&str>,
        options: Option<CesiumTerrainProviderOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::defined("url", url);
        }
        //>>includeEnd('debug');

        let resource = Resource::create_if_needed(url.unwrap());
        Self::from_resource(resource, options, backend).await
    }

    /// Creates a provider from a [`Resource`] (mirrors the `Resource` case of
    /// JS `fromUrl`).
    pub async fn from_resource<B: ResourceBackend + ?Sized>(
        resource: Resource,
        options: Option<CesiumTerrainProviderOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        Self::from_resource_inner(resource, options, backend, false).await
    }

    /// Creates a provider from an [`IonResource`] (mirrors the JS `fromUrl`
    /// behavior for ion endpoints, including the query-parameter extension
    /// negotiation).
    pub async fn from_ion_resource<B: ResourceBackend + ?Sized>(
        ion: IonResource,
        options: Option<CesiumTerrainProviderOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        let uses_ion_query = !ion.is_external();
        Self::from_resource_inner(ion.resource, options, backend, uses_ion_query).await
    }

    /// Shared `fromUrl` body.
    async fn from_resource_inner<B: ResourceBackend + ?Sized>(
        mut resource: Resource,
        options: Option<CesiumTerrainProviderOptions>,
        backend: &B,
        uses_ion_query: bool,
    ) -> Result<Self, RuntimeError> {
        let options = options.unwrap_or_default();

        resource.append_forward_slash();

        let layer_json_resource = resource.get_derived_resource("layer.json");
        let mut builder = TerrainProviderBuilder::new(
            &options,
            resource,
            layer_json_resource,
            uses_ion_query,
        );

        request_layer_json(&mut builder, backend).await?;

        let mut provider = Self::with_options(&options);
        builder.build(&mut provider);

        Ok(provider)
    }

    // ── Properties (mirror `Object.defineProperties`) ──────────────────

    /// Gets the event raised when the provider encounters an asynchronous
    /// error (JS `errorEvent`).
    pub fn error_event(&self) -> &Event {
        &self.error_event
    }

    /// Gets the credit to display when this terrain provider is active
    /// (JS `credit`).
    pub fn credit(&self) -> Option<&Credit> {
        self.credit.as_ref()
    }

    /// Whether the provider includes a water mask (JS `hasWaterMask` getter:
    /// `_hasWaterMask && _requestWaterMask`).
    pub fn has_water_mask(&self) -> bool {
        self.has_water_mask && self.request_water_mask
    }

    /// Whether the requested tiles include vertex normals (JS
    /// `hasVertexNormals` getter).
    pub fn has_vertex_normals(&self) -> bool {
        self.has_vertex_normals && self.request_vertex_normals
    }

    /// Whether the requested tiles include metadata (JS `hasMetadata`
    /// getter).
    pub fn has_metadata(&self) -> bool {
        self.has_metadata && self.request_metadata
    }

    /// Whether the client requests vertex normals (JS `requestVertexNormals`).
    pub fn request_vertex_normals(&self) -> bool {
        self.request_vertex_normals
    }

    /// Whether the client requests water masks (JS `requestWaterMask`).
    pub fn request_water_mask(&self) -> bool {
        self.request_water_mask
    }

    /// Whether the client requests metadata (JS `requestMetadata`).
    pub fn request_metadata(&self) -> bool {
        self.request_metadata
    }

    /// The availability object of the provider, if any (JS `availability`).
    pub fn availability(&self) -> Option<&TileAvailability> {
        self.availability.as_ref()
    }

    /// The layers of this provider (JS private `_layers`, used by specs).
    pub fn layers(&self) -> &[LayerInformation] {
        &self.layers
    }

    /// The per-tile credits (JS private `_tileCredits`, used by specs).
    pub fn tile_credits(&self) -> &[Credit] {
        &self.tile_credits
    }

    /// The tile coordinate scheme (`"tms"` / `"slippyMap"` / unset).
    pub fn scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }

    /// The ellipsoid used by this provider.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Gets the maximum geometric error allowed in a tile at a given level.
    ///
    /// Mirrors `getLevelMaximumGeometricError(level)`.
    pub fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        self.level_zero_maximum_geometric_error / (1i64 << level) as f64
    }

    /// Requests the geometry for a given tile.
    ///
    /// Mirrors `requestTileGeometry(x, y, level, request)`; DEVIATION: the
    /// JS `request`/scheduler throttling is not modeled (module DEVIATION 2),
    /// and the multi-layer availability retry is awaited inline instead of
    /// via `setTimeout`/promise caching (module DEVIATION 3/4).
    pub async fn request_tile_geometry<B: ResourceBackend + ?Sized>(
        &mut self,
        x: i32,
        y: i32,
        level: i32,
        backend: &B,
    ) -> Result<Option<TerrainTileData>, RuntimeError> {
        let layer_count = self.layers.len();

        let layer_to_use: Option<usize> = if layer_count == 1 {
            // Optimized path for single layers.
            Some(0)
        } else {
            let mut selected: Option<usize>;
            let mut unknown_availability = false;
            loop {
                selected = None;
                let mut issued = false;
                for i in 0..layer_count {
                    let available = match &self.layers[i].availability {
                        None => true,
                        Some(availability) => availability.is_tile_available(level, x, y),
                    };
                    if available {
                        selected = Some(i);
                        break;
                    }

                    let Some(tile) =
                        self.find_unloaded_availability_tile(i, x, y, level)
                    else {
                        continue;
                    };
                    if i == 0 {
                        // Top layer: we can't know yet since the
                        // availability is not yet loaded.
                        unknown_availability = true;
                        continue;
                    }
                    // For cutout terrain, if this isn't the top layer the
                    // availability tiles may never get loaded, so request it
                    // here. DEVIATION: JS caches the promise in
                    // `availabilityPromiseCache`; the Rust port awaits the
                    // load sequentially.
                    self.request_tile_geometry_from_layer(
                        tile.x,
                        tile.y,
                        tile.level,
                        Some(i),
                        backend,
                    )
                    .await?;
                    issued = true;
                    break;
                }
                if selected.is_some() || !issued {
                    break;
                }
            }

            if selected.is_none() && unknown_availability {
                // DEVIATION: JS keeps retrying on the next event loop tick;
                // the Rust port reports the unresolved state instead.
                return Err(RuntimeError::new(Some(
                    "Terrain tile availability is not yet loaded",
                )));
            }
            selected
        };

        self.request_tile_geometry_from_layer(x, y, level, layer_to_use, backend)
            .await
    }

    /// Walks the availability-tile chain of a layer (mirrors the `while
    /// (defined(tile))` loop of JS `checkLayer`) and returns the first tile
    /// that is available but not yet loaded.
    fn find_unloaded_availability_tile(
        &self,
        layer_index: usize,
        x: i32,
        y: i32,
        level: i32,
    ) -> Option<AvailabilityTile> {
        let layer = &self.layers[layer_index];
        let availability_levels = layer.availability_levels?;
        let availability = layer.availability.as_ref()?;
        let availability_tiles_loaded = layer.availability_tiles_loaded.as_ref()?;

        let mut tile = get_availability_tile(availability_levels, x, y, level);
        while let Some(t) = tile {
            if availability.is_tile_available(t.level, t.x, t.y)
                && !availability_tiles_loaded.is_tile_available(t.level, t.x, t.y)
            {
                return Some(t);
            }
            tile = get_availability_tile(availability_levels, t.x, t.y, t.level);
        }
        None
    }

    /// The body of JS `requestTileGeometry(provider, x, y, level, layerToUse,
    /// request)` (the overridden free function).
    async fn request_tile_geometry_from_layer<B: ResourceBackend + ?Sized>(
        &mut self,
        x: i32,
        y: i32,
        level: i32,
        layer_to_use: Option<usize>,
        backend: &B,
    ) -> Result<Option<TerrainTileData>, RuntimeError> {
        let Some(layer_index) = layer_to_use else {
            return Err(RuntimeError::new(Some("Terrain tile doesn't exist")));
        };

        // Snapshot the layer fields used below (the fetch needs `&mut self`
        // for the availability updates after the response arrives).
        let layer = &self.layers[layer_index];
        let url_templates = layer.tile_url_templates.clone();
        if url_templates.is_empty() {
            return Ok(None);
        }
        let version = layer.version.clone();
        let has_vertex_normals = layer.has_vertex_normals;
        let has_water_mask = layer.has_water_mask;
        let has_metadata = layer.has_metadata;
        let little_endian_extension_size = layer.little_endian_extension_size;
        let uses_ion_query = layer.uses_ion_query;
        let base_resource = layer.resource.clone_resource();

        // The TileMapService scheme counts from the bottom left.
        let terrain_y = if self.scheme.is_none()
            || self.scheme.as_deref() == Some("tms")
        {
            let y_tiles = self.tiling_scheme.get_number_of_y_tiles_at_level(level);
            y_tiles - y - 1
        } else {
            y
        };

        let mut extension_list: Vec<&str> = Vec::new();
        if self.request_vertex_normals && has_vertex_normals {
            extension_list.push(if little_endian_extension_size {
                "octvertexnormals"
            } else {
                "vertexnormals"
            });
        }
        if self.request_water_mask && has_water_mask {
            extension_list.push("watermask");
        }
        if self.request_metadata && has_metadata {
            extension_list.push("metadata");
        }

        let url =
            url_templates[((x + terrain_y + level).rem_euclid(url_templates.len() as i32))
                as usize]
                .clone();

        let headers: HashMap<String, String>;
        let mut query: Option<HashMap<String, String>> = None;
        if uses_ion_query {
            // ion uses query parameters to request extensions.
            if !extension_list.is_empty() {
                let mut params = HashMap::new();
                params.insert("extensions".to_string(), extension_list.join("-"));
                query = Some(params);
            }
            headers = get_request_header(None);
        } else {
            // All other terrain servers.
            headers = get_request_header(Some(&extension_list));
        }

        let mut template_values = HashMap::new();
        if let Some(version) = &version {
            template_values.insert("version".to_string(), version.clone());
        }
        template_values.insert("z".to_string(), level.to_string());
        template_values.insert("x".to_string(), x.to_string());
        template_values.insert("y".to_string(), terrain_y.to_string());

        let mut tile_resource = base_resource.get_derived_resource_with_options(
            DerivedResourceOptions {
                url: Some(&url),
                template_values: Some(&template_values),
                query_parameters: query.as_ref(),
                headers: Some(&headers),
                ..Default::default()
            },
        );

        let buffer = tile_resource
            .fetch_array_buffer(backend)
            .await
            .map_err(|e| RuntimeError::new(Some(&e.to_string())))?;

        let Some(buffer) = buffer else {
            return Err(RuntimeError::new(Some("Mesh buffer doesn't exist.")));
        };

        if self.heightmap_structure.is_some() {
            Ok(Some(TerrainTileData::Heightmap(
                self.create_heightmap_terrain_data(&buffer),
            )))
        } else {
            Ok(Some(TerrainTileData::QuantizedMesh(
                self.create_quantized_mesh_terrain_data(
                    &buffer,
                    level,
                    x,
                    y,
                    layer_index,
                )?,
            )))
        }
    }

    /// Mirrors `createHeightmapTerrainData(provider, buffer, level, x, y)`.
    fn create_heightmap_terrain_data(&self, buffer: &[u8]) -> HeightmapTerrainData {
        let width = self.heightmap_width as usize;
        let count = width * width;
        let height_byte_len = count * 2;

        let mut height_buffer = vec![0u16; count];
        for i in 0..count {
            height_buffer[i] =
                u16::from_le_bytes([buffer[2 * i], buffer[2 * i + 1]]);
        }

        let child_tile_mask = buffer[height_byte_len] as i32;
        let water_mask = buffer[height_byte_len + 1..].to_vec();

        let structure = self.heightmap_structure.unwrap();
        HeightmapTerrainData::new(HeightmapTerrainDataOptions {
            buffer: Some(HeightmapBuffer::U16(height_buffer)),
            child_tile_mask: Some(child_tile_mask),
            water_mask: Some(water_mask),
            width: Some(width),
            height: Some(width),
            structure: Some(HeightmapStructureOptions {
                height_scale: Some(structure.height_scale),
                height_offset: Some(structure.height_offset),
                elements_per_height: Some(structure.elements_per_height),
                stride: Some(structure.stride),
                element_multiplier: Some(structure.element_multiplier),
                is_big_endian: Some(structure.is_big_endian),
                lowest_encoded_height: structure.lowest_encoded_height,
                highest_encoded_height: structure.highest_encoded_height,
            }),
            // DEVIATION: JS passes `credits: provider._tileCredits`; the Rust
            // HeightmapTerrainData options do not model credits.
            ..Default::default()
        })
    }

    /// Mirrors `createQuantizedMeshTerrainData(provider, buffer, level, x, y,
    /// layer)` including the extension parsing loop.
    fn create_quantized_mesh_terrain_data(
        &mut self,
        buffer: &[u8],
        level: i32,
        x: i32,
        y: i32,
        layer_index: usize,
    ) -> Result<QuantizedMeshTerrainData, RuntimeError> {
        let little_endian_extension_size =
            self.layers[layer_index].little_endian_extension_size;

        let mut pos = 0usize;
        let too_short =
            || RuntimeError::new(Some("Quantized-mesh tile buffer is too short."));

        let read_f64 = |buffer: &[u8], pos: usize| -> Result<f64, RuntimeError> {
            let bytes: [u8; 8] = buffer.get(pos..pos + 8).ok_or_else(too_short)?.try_into().unwrap();
            Ok(f64::from_le_bytes(bytes))
        };
        let read_f32 = |buffer: &[u8], pos: usize| -> Result<f32, RuntimeError> {
            let bytes: [u8; 4] = buffer.get(pos..pos + 4).ok_or_else(too_short)?.try_into().unwrap();
            Ok(f32::from_le_bytes(bytes))
        };
        let read_u32 = |buffer: &[u8], pos: usize, little_endian: bool| -> Result<u32, RuntimeError> {
            let bytes: [u8; 4] = buffer.get(pos..pos + 4).ok_or_else(too_short)?.try_into().unwrap();
            Ok(if little_endian {
                u32::from_le_bytes(bytes)
            } else {
                u32::from_be_bytes(bytes)
            })
        };

        let cartesian3_elements = 3usize;
        let bounding_sphere_elements = cartesian3_elements + 1;
        let cartesian3_length = 8 * cartesian3_elements;
        let bounding_sphere_length = 8 * bounding_sphere_elements;
        let encoded_vertex_elements = 3usize;
        let encoded_vertex_length = 2 * encoded_vertex_elements;
        let triangle_elements = 3usize;
        let mut bytes_per_index = 2usize;
        let mut triangle_length = bytes_per_index * triangle_elements;

        let center = Cartesian3::new(
            read_f64(buffer, pos)?,
            read_f64(buffer, pos + 8)?,
            read_f64(buffer, pos + 16)?,
        );
        pos += cartesian3_length;

        let minimum_height = read_f32(buffer, pos)? as f64;
        pos += 4;
        let maximum_height = read_f32(buffer, pos)? as f64;
        pos += 4;

        let bounding_sphere = BoundingSphere::new(
            Cartesian3::new(
                read_f64(buffer, pos)?,
                read_f64(buffer, pos + 8)?,
                read_f64(buffer, pos + 16)?,
            ),
            read_f64(buffer, pos + cartesian3_length)?,
        );
        pos += bounding_sphere_length;

        let horizon_occlusion_point = Cartesian3::new(
            read_f64(buffer, pos)?,
            read_f64(buffer, pos + 8)?,
            read_f64(buffer, pos + 16)?,
        );
        pos += cartesian3_length;

        let vertex_count = read_u32(buffer, pos, true)? as usize;
        pos += 4;
        let vertex_byte_len = vertex_count * encoded_vertex_length;
        if buffer.len() < pos + vertex_byte_len {
            return Err(too_short());
        }
        let mut encoded_vertex_buffer = vec![0u16; vertex_count * encoded_vertex_elements];
        for i in 0..vertex_count * encoded_vertex_elements {
            encoded_vertex_buffer[i] = u16::from_le_bytes([
                buffer[pos + 2 * i],
                buffer[pos + 2 * i + 1],
            ]);
        }
        pos += vertex_byte_len;

        if vertex_count > 64 * 1024 {
            // More than 64k vertices, so indices are 32-bit.
            bytes_per_index = 4;
            triangle_length = bytes_per_index * triangle_elements;
        }

        // Decode the vertex buffer.
        let mut u_buffer: Vec<u16> = encoded_vertex_buffer[0..vertex_count].to_vec();
        let mut v_buffer: Vec<u16> =
            encoded_vertex_buffer[vertex_count..2 * vertex_count].to_vec();
        let mut height_buffer: Vec<u16> =
            encoded_vertex_buffer[vertex_count * 2..3 * vertex_count].to_vec();

        AttributeCompression::zig_zag_delta_decode(
            &mut u_buffer,
            &mut v_buffer,
            Some(&mut height_buffer),
        );
        // Write the decoded values back into the encoded buffer (JS decodes
        // the subarrays in place).
        for i in 0..vertex_count {
            encoded_vertex_buffer[i] = u_buffer[i];
            encoded_vertex_buffer[vertex_count + i] = v_buffer[i];
            encoded_vertex_buffer[2 * vertex_count + i] = height_buffer[i];
        }

        // Skip over any additional padding that was added for 2/4 byte
        // alignment.
        if pos % bytes_per_index != 0 {
            pos += bytes_per_index - (pos % bytes_per_index);
        }

        let triangle_count = read_u32(buffer, pos, true)? as usize;
        pos += 4;
        let index_count = triangle_count * triangle_elements;
        let mut indices =
            read_index_array(buffer, pos, vertex_count, index_count)?;
        pos += triangle_count * triangle_length;

        // High water mark decoding based on decompressIndices_ in
        // webgl-loader's loader.js (Copyright 2012 Google Inc., Apache 2.0).
        let mut highest = 0u32;
        for index in indices.iter_mut() {
            let code = *index;
            *index = highest - code;
            if code == 0 {
                highest += 1;
            }
        }

        let west_vertex_count = read_u32(buffer, pos, true)? as usize;
        pos += 4;
        let west_indices =
            read_index_array(buffer, pos, vertex_count, west_vertex_count)?;
        pos += west_vertex_count * bytes_per_index;

        let south_vertex_count = read_u32(buffer, pos, true)? as usize;
        pos += 4;
        let south_indices =
            read_index_array(buffer, pos, vertex_count, south_vertex_count)?;
        pos += south_vertex_count * bytes_per_index;

        let east_vertex_count = read_u32(buffer, pos, true)? as usize;
        pos += 4;
        let east_indices =
            read_index_array(buffer, pos, vertex_count, east_vertex_count)?;
        pos += east_vertex_count * bytes_per_index;

        let north_vertex_count = read_u32(buffer, pos, true)? as usize;
        pos += 4;
        let north_indices =
            read_index_array(buffer, pos, vertex_count, north_vertex_count)?;
        pos += north_vertex_count * bytes_per_index;

        let mut encoded_normals: Option<Vec<u8>> = None;
        let mut water_mask: Option<Vec<u8>> = None;
        while pos < buffer.len() {
            let extension_id = buffer[pos];
            pos += 1;
            let extension_length =
                read_u32(buffer, pos, little_endian_extension_size)? as usize;
            pos += 4;

            if extension_id == QUANTIZED_MESH_EXTENSION_IDS.oct_vertex_normals
                && self.request_vertex_normals
            {
                encoded_normals =
                    Some(buffer[pos..pos + vertex_count * 2].to_vec());
            } else if extension_id == QUANTIZED_MESH_EXTENSION_IDS.water_mask
                && self.request_water_mask
            {
                water_mask = Some(buffer[pos..pos + extension_length].to_vec());
            } else if extension_id == QUANTIZED_MESH_EXTENSION_IDS.metadata
                && self.request_metadata
            {
                let string_length = read_u32(buffer, pos, true)? as usize;
                if string_length > 0 {
                    let json_start = pos + 4;
                    let json_bytes = buffer
                        .get(json_start..json_start + string_length)
                        .ok_or_else(too_short)?;
                    if let Ok(metadata) = serde_json::from_slice::<Value>(json_bytes) {
                        if let Some(available_tiles) =
                            metadata.get("available").and_then(|v| v.as_array())
                        {
                            for (offset, ranges_at_level) in
                                available_tiles.iter().enumerate()
                            {
                                let available_level = level + offset as i32 + 1;
                                let y_tiles = self
                                    .tiling_scheme
                                    .get_number_of_y_tiles_at_level(available_level);
                                let Some(ranges) = ranges_at_level.as_array() else {
                                    continue;
                                };
                                for range in ranges {
                                    let (start_x, start_y, end_x, end_y) =
                                        range_fields(range);
                                    let y_start = y_tiles - end_y - 1;
                                    let y_end = y_tiles - start_y - 1;
                                    if let Some(availability) =
                                        self.availability.as_mut()
                                    {
                                        availability.add_available_tile_range(
                                            available_level,
                                            start_x,
                                            y_start,
                                            end_x,
                                            y_end,
                                        );
                                    }
                                    if let Some(layer_availability) =
                                        self.layers[layer_index].availability.as_mut()
                                    {
                                        layer_availability.add_available_tile_range(
                                            available_level,
                                            start_x,
                                            y_start,
                                            end_x,
                                            y_end,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(loaded) =
                    self.layers[layer_index].availability_tiles_loaded.as_mut()
                {
                    loaded.add_available_tile_range(level, x, y, x, y);
                }
            }
            pos += extension_length;
        }

        let skirt_height = self.get_level_maximum_geometric_error(level) * 5.0;

        // The skirt is not included in the OBB computation (see JS comment).
        let mut rectangle = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
        self.tiling_scheme
            .tile_xy_to_rectangle(x, y, level, &mut rectangle);
        let ellipsoid = *self.tiling_scheme.ellipsoid();
        let oriented_bounding_box = OrientedBoundingBox::from_rectangle(
            Some(&rectangle),
            Some(minimum_height),
            Some(maximum_height),
            Some(ellipsoid),
            None,
        );

        // DEVIATION: JS uses `provider.availability` unconditionally; the
        // Rust port falls back to an empty mask when availability is absent.
        let child_tile_mask = self
            .availability
            .as_ref()
            .map(|a| a.compute_child_mask_for_tile(level, x, y) as i32)
            .unwrap_or(0);

        // DEVIATION: the parsed `center` is not modeled by the Rust
        // `QuantizedMeshTerrainData` options; credits are passed as HTML
        // strings.
        let _ = center;
        let credits: Vec<String> = self
            .tile_credits
            .iter()
            .map(|c| c.html().to_string())
            .collect();

        Ok(QuantizedMeshTerrainData::new(QuantizedMeshTerrainDataOptions {
            minimum_height: Some(minimum_height),
            maximum_height: Some(maximum_height),
            bounding_sphere: Some(bounding_sphere),
            oriented_bounding_box: Some(oriented_bounding_box),
            horizon_occlusion_point: Some(horizon_occlusion_point),
            quantized_vertices: Some(encoded_vertex_buffer),
            encoded_normals,
            indices: Some(indices),
            west_indices: Some(west_indices),
            south_indices: Some(south_indices),
            east_indices: Some(east_indices),
            north_indices: Some(north_indices),
            west_skirt_height: Some(skirt_height),
            south_skirt_height: Some(skirt_height),
            east_skirt_height: Some(skirt_height),
            north_skirt_height: Some(skirt_height),
            child_tile_mask: Some(child_tile_mask),
            water_mask,
            credits: Some(credits),
            ..Default::default()
        }))
    }

    /// Determines whether data for a tile is available to be loaded.
    ///
    /// Mirrors `getTileDataAvailable(x, y, level)`.
    pub fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool> {
        let Some(availability) = &self.availability else {
            return None;
        };
        if level > availability.maximum_level() {
            return Some(false);
        }

        if availability.is_tile_available(level, x, y) {
            // If the tile is listed as available, then we are done.
            return Some(true);
        }
        if !self.has_metadata {
            // If we don't have any layers with the metadata extension then we
            // don't have this tile.
            return Some(false);
        }

        let count = self.layers.len();
        for i in 0..count {
            let layer_result = self.check_layer(x, y, level, i, i == 0);
            if layer_result {
                // There is a layer that may or may not have the tile.
                return None;
            }
        }

        Some(false)
    }

    /// Makes sure we load availability data for a tile.
    ///
    /// Mirrors `loadTileDataAvailability(x, y, level)`; DEVIATION: the Rust
    /// port never issues deferred availability-tile requests (module
    /// DEVIATION 4), so there is never anything to wait on and this always
    /// returns `None` after the JS guard checks.
    pub fn load_tile_data_availability(&self, x: i32, y: i32, level: i32) -> Option<()> {
        let Some(availability) = &self.availability else {
            return None;
        };
        if level > availability.maximum_level()
            || availability.is_tile_available(level, x, y)
            || !self.has_metadata
        {
            // We know the tile is either available or not available so
            // nothing to wait on.
            return None;
        }

        let count = self.layers.len();
        for i in 0..count {
            let _layer_result = self.check_layer(x, y, level, i, i == 0);
            // DEVIATION: JS returns the first `layerResult.promise` here; the
            // Rust `check_layer` never issues requests, so no promise exists.
        }
        None
    }

    /// Mirrors `checkLayer(provider, x, y, level, layer, topLayer)`; returns
    /// the JS `result` boolean. DEVIATION: availability-tile requests for
    /// non-top layers are issued inline by `request_tile_geometry` rather
    /// than cached in `availabilityPromiseCache` (module DEVIATION 4).
    fn check_layer(
        &self,
        x: i32,
        y: i32,
        level: i32,
        layer_index: usize,
        _top_layer: bool,
    ) -> bool {
        if self.layers[layer_index].availability_levels.is_none() {
            // It's definitely not in this layer.
            return false;
        }
        // The availability tile is available, but not loaded, so there is
        // still a chance that it may become available at some point.
        self.find_unloaded_availability_tile(layer_index, x, y, level)
            .is_some()
    }
}

impl Default for CesiumTerrainProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TerrainProvider for CesiumTerrainProvider {
    fn tiling_scheme(&self) -> &dyn TilingScheme {
        self.tiling_scheme.as_ref()
    }

    fn has_water_mask(&self) -> bool {
        CesiumTerrainProvider::has_water_mask(self)
    }

    fn has_vertex_normals(&self) -> bool {
        CesiumTerrainProvider::has_vertex_normals(self)
    }

    fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        CesiumTerrainProvider::get_level_maximum_geometric_error(self, level)
    }

    fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool> {
        CesiumTerrainProvider::get_tile_data_available(self, x, y, level)
    }
}

/// Reads a run of indices (16-bit when `vertex_count <= 65536`, else 32-bit)
/// from the buffer, mirroring `IndexDatatype.createTypedArrayFromArrayBuffer`
/// followed by the provider's widening into the options' `Vec<u32>`.
fn read_index_array(
    buffer: &[u8],
    pos: usize,
    vertex_count: usize,
    count: usize,
) -> Result<Vec<u32>, RuntimeError> {
    let mut result = vec![0u32; count];
    if vertex_count >= 65536 {
        for i in 0..count {
            let start = pos + 4 * i;
            let bytes: [u8; 4] = buffer
                .get(start..start + 4)
                .ok_or_else(|| {
                    RuntimeError::new(Some("Quantized-mesh tile buffer is too short."))
                })?
                .try_into()
                .unwrap();
            result[i] = u32::from_le_bytes(bytes);
        }
    } else {
        for i in 0..count {
            let start = pos + 2 * i;
            let bytes: [u8; 2] = buffer
                .get(start..start + 2)
                .ok_or_else(|| {
                    RuntimeError::new(Some("Quantized-mesh tile buffer is too short."))
                })?
                .try_into()
                .unwrap();
            result[i] = u16::from_le_bytes(bytes) as u32;
        }
    }
    Ok(result)
}

/// Extracts `startX/startY/endX/endY` from a layer.json availability range.
fn range_fields(range: &Value) -> (i32, i32, i32, i32) {
    let get = |name: &str| -> i32 {
        range
            .get(name)
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32
    };
    (get("startX"), get("startY"), get("endX"), get("endY"))
}

/// Mirrors `getRequestHeader(extensionsList)`.
fn get_request_header(extensions_list: Option<&[&str]>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    let accept = match extensions_list {
        None => "application/vnd.quantized-mesh,application/octet-stream;q=0.9,*/*;q=0.01"
            .to_string(),
        Some(list) if list.is_empty() => {
            "application/vnd.quantized-mesh,application/octet-stream;q=0.9,*/*;q=0.01"
                .to_string()
        }
        Some(list) => format!(
            "application/vnd.quantized-mesh;extensions={},application/octet-stream;q=0.9,*/*;q=0.01",
            list.join("-")
        ),
    };
    headers.insert("Accept".to_string(), accept);
    headers
}

/// Mirrors `parseMetadataSuccess(terrainProviderBuilder, data, provider)`.
fn parse_metadata_success<'a, B: ResourceBackend + ?Sized + 'a>(
    builder: &'a mut TerrainProviderBuilder,
    data: &'a Value,
    backend: &'a B,
) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + 'a>> {
    Box::pin(async move {
        let format = match data.get("format").and_then(|v| v.as_str()) {
            None => {
                return report_builder_error(
                    builder,
                    "The tile format is not specified in the layer.json file.",
                );
            }
            Some(format) => format.to_string(),
        };

        let tiles: Vec<String> = data
            .get("tiles")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if tiles.is_empty() {
            return report_builder_error(
                builder,
                "The layer.json file does not specify any tile URL templates.",
            );
        }

        let mut has_vertex_normals = false;
        let mut has_water_mask = false;
        let mut has_metadata = false;
        let mut little_endian_extension_size = true;
        let mut is_heightmap = false;
        if format == "heightmap-1.0" {
            is_heightmap = true;
            if builder.heightmap_structure.is_none() {
                builder.heightmap_structure = Some(HeightmapStructure {
                    height_scale: 1.0 / 5.0,
                    height_offset: -1000.0,
                    elements_per_height: 1,
                    stride: 1,
                    element_multiplier: 256.0,
                    is_big_endian: false,
                    lowest_encoded_height: Some(0.0),
                    highest_encoded_height: Some(256.0 * 256.0 - 1.0),
                });
            }
            has_water_mask = true;
            builder.request_water_mask = true;
        } else if !format.starts_with("quantized-mesh-1.") {
            return report_builder_error(
                builder,
                &format!("The tile format \"{format}\" is invalid or not supported."),
            );
        }

        let tile_url_templates = tiles;

        // JS: `const maxZoom = data.maxzoom;` (may be undefined) followed by
        // `overallMaxZoom = Math.max(overallMaxZoom, maxZoom)`; when `maxzoom`
        // is undefined the result is NaN (no layer provided a maxzoom).
        let max_zoom = data.get("maxzoom").and_then(|v| v.as_i64()).map(|v| v as i32);
        if let Some(max_zoom_value) = max_zoom {
            builder.overall_max_zoom =
                Some(builder.overall_max_zoom.map_or(max_zoom_value, |z| z.max(max_zoom_value)));
        }

        // Keeps track of which of the availability containing tiles have been
        // loaded.
        let projection = data.get("projection").and_then(|v| v.as_str());
        let kind = if projection.is_none() || projection == Some("EPSG:4326") {
            TilingSchemeKind::Geographic
        } else if projection == Some("EPSG:3857") {
            TilingSchemeKind::WebMercator
        } else {
            return report_builder_error(
                builder,
                &format!(
                    "The projection \"{}\" is invalid or not supported.",
                    projection.unwrap()
                ),
            );
        };
        builder.tiling_scheme_kind = Some(kind);

        let tiling_scheme = kind.make(builder.ellipsoid);
        builder.level_zero_maximum_geometric_error =
            get_estimated_level_zero_geometric_error_for_a_heightmap(
                tiling_scheme.ellipsoid(),
                builder.heightmap_width as f64,
                tiling_scheme.get_number_of_x_tiles_at_level(0),
            );

        let scheme = data.get("scheme").and_then(|v| v.as_str());
        if scheme.is_none() || scheme == Some("tms") || scheme == Some("slippyMap") {
            builder.scheme = scheme.map(|s| s.to_string());
        } else {
            return report_builder_error(
                builder,
                &format!(
                    "The scheme \"{}\" is invalid or not supported.",
                    scheme.unwrap()
                ),
            );
        }

        let mut availability_tiles_loaded: Option<TileAvailability> = None;

        // The vertex normals defined in the 'octvertexnormals' extension are
        // identical to the original contents of the 'vertexnormals'
        // extension. 'vertexnormals' is deprecated (its extensionLength was
        // incorrectly big endian); maintain backwards compatibility by
        // setting littleEndianExtensionSize to false. Always prefer
        // 'octvertexnormals'.
        let extensions: Vec<String> = data
            .get("extensions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if extensions.iter().any(|e| e == "octvertexnormals") {
            has_vertex_normals = true;
        } else if extensions.iter().any(|e| e == "vertexnormals") {
            has_vertex_normals = true;
            little_endian_extension_size = false;
        }
        if extensions.iter().any(|e| e == "watermask") {
            has_water_mask = true;
        }
        if extensions.iter().any(|e| e == "metadata") {
            has_metadata = true;
        }

        let availability_levels = data
            .get("metadataAvailability")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        let available_tiles = data.get("available").and_then(|v| v.as_array());
        let mut availability: Option<TileAvailability> = None;
        if let (Some(available_tiles), None) = (available_tiles, availability_levels) {
            let mut avail = TileAvailability::new(
                kind.make(builder.ellipsoid),
                available_tiles.len() as i32,
            );
            for (level, ranges_at_level) in available_tiles.iter().enumerate() {
                let y_tiles = tiling_scheme.get_number_of_y_tiles_at_level(level as i32);
                while builder.overall_availability.len() <= level {
                    builder.overall_availability.push(Vec::new());
                }

                let Some(ranges) = ranges_at_level.as_array() else {
                    continue;
                };
                for range in ranges {
                    let (start_x, start_y, end_x, end_y) = range_fields(range);
                    let y_start = y_tiles - end_y - 1;
                    let y_end = y_tiles - start_y - 1;
                    builder.overall_availability[level].push([
                        start_x, y_start, end_x, y_end,
                    ]);
                    avail.add_available_tile_range(
                        level as i32, start_x, y_start, end_x, y_end,
                    );
                }
            }
            availability = Some(avail);
        } else if availability_levels.is_some() {
            // JS: `new TileAvailability(tilingScheme, maxZoom)` with `maxZoom`
            // possibly undefined (NaN): no quadtree depth bound.
            let max_zoom_or_nan = max_zoom.unwrap_or(i32::MAX);
            availability_tiles_loaded =
                Some(TileAvailability::new(kind.make(builder.ellipsoid), max_zoom_or_nan));
            availability = Some(TileAvailability::new(kind.make(builder.ellipsoid), max_zoom_or_nan));
            if builder.overall_availability.is_empty() {
                builder.overall_availability.push(Vec::new());
            }
            builder.overall_availability[0] = vec![[0, 0, 1, 0]];
            availability
                .as_mut()
                .unwrap()
                .add_available_tile_range(0, 0, 0, 1, 0);
        }

        builder.has_water_mask = builder.has_water_mask || has_water_mask;
        builder.has_vertex_normals = builder.has_vertex_normals || has_vertex_normals;
        builder.has_metadata = builder.has_metadata || has_metadata;

        if let Some(attribution) = data.get("attribution").and_then(|v| v.as_str()) {
            if !builder.attribution.is_empty() {
                builder.attribution.push(' ');
            }
            builder.attribution.push_str(attribution);
        }

        builder.layers.push(LayerInformation {
            resource: builder.last_resource.clone_resource(),
            version: data
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            is_heightmap,
            tile_url_templates,
            availability,
            has_vertex_normals,
            has_water_mask,
            has_metadata,
            availability_levels,
            availability_tiles_loaded,
            little_endian_extension_size,
            uses_ion_query: builder.uses_ion_query,
        });

        if let Some(parent_url) = data.get("parentUrl").and_then(|v| v.as_str()) {
            if builder.layers.last().unwrap().availability.is_none() {
                // JS: console.log("A layer.json can't have a parentUrl if it
                // does't have an available array.");
                eprintln!(
                    "A layer.json can't have a parentUrl if it does't have an available array."
                );
                return Ok(());
            }

            builder.last_resource =
                builder.last_resource.get_derived_resource(parent_url);
            // Terrain always expects a directory.
            builder.last_resource.append_forward_slash();
            builder.layer_json_resource =
                builder.last_resource.get_derived_resource("layer.json");
            return request_layer_json(builder, backend).await;
        }

        Ok(())
    })
}

/// Mirrors `parseMetadataFailure(terrainProviderBuilder, error, provider)`.
///
/// DEVIATION: the JS retry flow (`previousError.retry`) is not modeled; the
/// Rust `TileProviderError.retry` is always false, so this always errors.
fn parse_metadata_failure(
    builder: &mut TerrainProviderBuilder,
    error: Option<&ResourceError>,
) -> Result<(), RuntimeError> {
    let mut message = format!(
        "An error occurred while accessing {}.",
        builder.layer_json_resource.url()
    );
    if let Some(error) = error {
        message.push_str(&format!("\n{error}"));
    }

    builder.previous_error =
        Some(TileProviderError::report_error(builder.previous_error.take(), message.clone(), None, None, None));

    // If we can retry, do so. Otherwise throw the error.
    if builder.previous_error.as_ref().unwrap().retry {
        // DEVIATION: the async retry (re-requesting layer.json) is not
        // modeled; fall through to the error.
        unreachable!("TileProviderError.retry is never set in the Rust port");
    }

    Err(RuntimeError::new(Some(&message)))
}

/// Reports an error through `TileProviderError` and yields a `RuntimeError`
/// (mirrors the repeated `reportError` + `throw` blocks of
/// `parseMetadataSuccess`).
fn report_builder_error(
    builder: &mut TerrainProviderBuilder,
    message: &str,
) -> Result<(), RuntimeError> {
    builder.previous_error = Some(TileProviderError::report_error(
        builder.previous_error.take(),
        message.to_string(),
        None,
        None,
        None,
    ));
    Err(RuntimeError::new(Some(message)))
}

/// Mirrors `metadataSuccess(terrainProviderBuilder, data, provider)`.
fn metadata_success<'a, B: ResourceBackend + ?Sized + 'a>(
    builder: &'a mut TerrainProviderBuilder,
    data: &'a Value,
    backend: &'a B,
) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + 'a>> {
    Box::pin(async move {
        parse_metadata_success(builder, data, backend).await?;

        let length = builder.overall_availability.len();
        if length > 0 {
            let kind = builder.tiling_scheme_kind.unwrap_or(TilingSchemeKind::Geographic);
            let mut availability = TileAvailability::new(
                kind.make(builder.ellipsoid),
                // JS NaN behaves as "no bound" (both `level > NaN` and the
                // quadtree depth guard `node.level < NaN` are false); i32::MAX
                // reproduces that here.
                builder.overall_max_zoom.unwrap_or(i32::MAX),
            );
            for level in 0..length {
                let level_ranges = builder.overall_availability[level].clone();
                for range in &level_ranges {
                    availability.add_available_tile_range(
                        level as i32,
                        range[0],
                        range[1],
                        range[2],
                        range[3],
                    );
                }
            }
            builder.availability = Some(availability);
        }

        if !builder.attribution.is_empty() {
            let layer_json_credit = Credit::new(&builder.attribution, false);
            builder.tile_credits.push(layer_json_credit);
        }

        Ok(())
    })
}

/// Mirrors `requestLayerJson(terrainProviderBuilder, provider)`.
fn request_layer_json<'a, B: ResourceBackend + ?Sized + 'a>(
    builder: &'a mut TerrainProviderBuilder,
    backend: &'a B,
) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + 'a>> {
    Box::pin(async move {
        let result = builder.layer_json_resource.fetch_json(backend).await;
        match result {
            Ok(Some(data)) => metadata_success(builder, &data, backend).await,
            Ok(None) => parse_metadata_failure(builder, None),
            Err(error) => {
                // If the metadata is not found, assume this is a
                // pre-metadata heightmap tileset.
                if matches!(error, ResourceError::HttpError { status: 404, .. }) {
                    let default = serde_json::json!({
                        "tilejson": "2.1.0",
                        "format": "heightmap-1.0",
                        "version": "1.0.0",
                        "scheme": "tms",
                        "tiles": ["{z}/{x}/{y}.terrain?v={version}"],
                    });
                    parse_metadata_success(builder, &default, backend).await?;
                    return Ok(());
                }

                parse_metadata_failure(builder, Some(&error))
            }
        }
    })
}
