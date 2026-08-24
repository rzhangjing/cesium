//! Ported from `packages/engine/Source/Core/ArcGISTiledElevationTerrainProvider.js`.
//!
//! A [`TerrainProvider`] that produces terrain geometry by tessellating height
//! maps retrieved from Elevation Tiles of an ArcGIS ImageService.
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `ALL_CHILDREN` | [`ALL_CHILDREN`] | identical |
//! | `TerrainProviderBuilder` | [`TerrainProviderBuilder`] | tiling scheme kept as [`SchemeSpec`] (built once per consumer since `TileAvailability` owns its scheme) |
//! | `TerrainProviderBuilder.prototype.build` | [`TerrainProviderBuilder::build`] | identical |
//! | `parseMetadataSuccess` | [`parse_metadata_success`] | identical logic (`console.log` → `println!`) |
//! | `requestMetadata` | [`request_metadata`] | identical error reporting |
//! | `ArcGISTiledElevationTerrainProvider` constructor | [`ArcGISTiledElevationTerrainProvider::new_internal`] | private (call `from_url`) |
//! | `fromUrl` | [`ArcGISTiledElevationTerrainProvider::from_url`] / [`ArcGISTiledElevationTerrainProvider::from_resource`] | DEVIATION 1 |
//! | `requestTileGeometry` | [`ArcGISTiledElevationTerrainProvider::request_tile_geometry`] | DEVIATION 2 |
//! | `isTileAvailable` | [`ArcGISTiledElevationTerrainProvider::is_tile_available`] | identical |
//! | `getLevelMaximumGeometricError` | [`ArcGISTiledElevationTerrainProvider::get_level_maximum_geometric_error`] | identical |
//! | `getTileDataAvailable` | [`ArcGISTiledElevationTerrainProvider::get_tile_data_available`] | DEVIATION 3 |
//! | `loadTileDataAvailability` | [`ArcGISTiledElevationTerrainProvider::load_tile_data_availability`] | identical (`undefined`) |
//! | `findRange` | [`find_range`] | identical |
//! | `computeAvailability` | [`compute_availability`] | identical |
//! | `requestAvailability` | [`ArcGISTiledElevationTerrainProvider::request_availability`] | DEVIATION 2 |
//! | properties | accessor methods / `TerrainProvider` impl | identical values |
//!
//! # DEVIATIONS
//!
//! 1. `fromUrl` accepts a `Resource` or `&str` (JS also accepts promises,
//!    which Rust callers resolve before the call). HTTP access goes through
//!    the injected [`ResourceBackend`].
//! 2. `Request`/`RequestScheduler` throttling and cancellation are not
//!    modeled; the in-flight availability cache only dedupes by URL.
//! 3. The JS `getTileDataAvailable` kicks off an asynchronous availability
//!    load when unknown; the Rust trait method is synchronous and returns
//!    `None` (unknown) in that case — availability loading happens inside
//!    [`ArcGISTiledElevationTerrainProvider::request_tile_geometry`].
//! 4. JS `options.credit` is ignored by the JS implementation (the credit
//!    always comes from the metadata `copyrightText`); the Rust options do
//!    the same.

use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::Value;

use crate::cartesian2::Cartesian2;
use crate::check;
use crate::credit::Credit;
use crate::ellipsoid::Ellipsoid;
use crate::event::Event;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::heightmap_encoding::HeightmapEncoding;
use crate::heightmap_terrain_data::{
    HeightmapBuffer, HeightmapStructureOptions, HeightmapTerrainData,
    HeightmapTerrainDataOptions,
};
use crate::rectangle::Rectangle;
use crate::resource::{DerivedResourceOptions, Resource, ResourceBackend};
use crate::runtime_error::RuntimeError;
use crate::terrain_provider::{
    get_estimated_level_zero_geometric_error_for_a_heightmap, TerrainProvider,
};
use crate::tile_availability::TileAvailability;
use crate::tile_provider_error::TileProviderError;
use crate::tiling_scheme::TilingScheme;
use crate::web_mercator_tiling_scheme::WebMercatorTilingScheme;

const ALL_CHILDREN: i32 = 15;

/// Initialization options for [`ArcGISTiledElevationTerrainProvider`].
///
/// Mirrors `ArcGISTiledElevationTerrainProvider.ConstructorOptions`.
#[derive(Default, Clone)]
pub struct ArcGISTiledElevationTerrainProviderOptions {
    /// The authorization token to use to connect to the service.
    pub token: Option<String>,
    /// The ellipsoid. If not specified, the default ellipsoid is used.
    pub ellipsoid: Option<Ellipsoid>,
}

/// A data range returned by [`find_range`] (mirrors the anonymous object).
struct AvailabilityRange {
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
}

/// Captures the tiling-scheme parameters parsed from the metadata so that
/// independent scheme instances can be built for the provider and for the
/// [`TileAvailability`] objects (which own their scheme).
#[derive(Clone)]
enum SchemeSpec {
    Geographic(Rectangle),
    WebMercator(Cartesian2, Cartesian2),
}

impl SchemeSpec {
    fn build(&self, ellipsoid: Ellipsoid) -> Box<dyn TilingScheme> {
        match self {
            SchemeSpec::Geographic(rectangle) => Box::new(GeographicTilingScheme::new(
                Some(ellipsoid),
                Some(rectangle.clone()),
                None,
                None,
            )),
            SchemeSpec::WebMercator(southwest, northeast) => Box::new(
                WebMercatorTilingScheme::new(
                    Some(ellipsoid),
                    None,
                    None,
                    Some(*southwest),
                    Some(*northeast),
                ),
            ),
        }
    }
}

/// Used to track creation details while fetching initial metadata.
///
/// Mirrors the private `TerrainProviderBuilder`.
struct TerrainProviderBuilder {
    ellipsoid: Ellipsoid,
    credit: Option<Credit>,
    scheme_spec: Option<SchemeSpec>,
    height: usize,
    width: usize,
    encoding: HeightmapEncoding,
    lod_count: i32,
    has_availability: bool,
    tiles_available: Option<TileAvailability>,
    tiles_availability_loaded: Option<TileAvailability>,
    level_zero_maximum_geometric_error: f64,
    terrain_data_structure: Option<HeightmapStructureOptions>,
}

impl TerrainProviderBuilder {
    fn new(options: &ArcGISTiledElevationTerrainProviderOptions) -> Self {
        Self {
            ellipsoid: options.ellipsoid.unwrap_or(Ellipsoid::WGS84),
            credit: None,
            scheme_spec: None,
            height: 0,
            width: 0,
            encoding: HeightmapEncoding::None,
            lod_count: 0,
            has_availability: false,
            tiles_available: None,
            tiles_availability_loaded: None,
            level_zero_maximum_geometric_error: 0.0,
            terrain_data_structure: None,
        }
    }

    fn build(self, provider: &mut ArcGISTiledElevationTerrainProvider) {
        provider.credit = self.credit;
        provider.tiling_scheme = self.scheme_spec.map(|spec| spec.build(self.ellipsoid));
        provider.height = self.height;
        provider.width = self.width;
        provider.encoding = self.encoding;
        provider.lod_count = self.lod_count;
        provider.has_availability = self.has_availability;
        provider.tiles_available = self.tiles_available;
        provider.tiles_availability_loaded = self.tiles_availability_loaded;
        provider.level_zero_maximum_geometric_error = self.level_zero_maximum_geometric_error;
        provider.terrain_data_structure = self.terrain_data_structure;
    }
}

/// Mirrors `parseMetadataSuccess(terrainProviderBuilder, metadata)`.
fn parse_metadata_success(
    builder: &mut TerrainProviderBuilder,
    metadata: &Value,
) -> Result<(), RuntimeError> {
    if let Some(copyright_text) = metadata.get("copyrightText").and_then(|v| v.as_str()) {
        builder.credit = Some(Credit::new(copyright_text, false));
    }

    let spatial_reference = metadata.get("spatialReference");
    let wkid = spatial_reference
        .and_then(|sr| sr.get("latestWkid"))
        .or_else(|| spatial_reference.and_then(|sr| sr.get("wkid")))
        .and_then(|v| v.as_i64());
    let extent = metadata.get("extent");
    let xmin = extent.and_then(|e| e.get("xmin")).and_then(|v| v.as_f64());
    let ymin = extent.and_then(|e| e.get("ymin")).and_then(|v| v.as_f64());
    let xmax = extent.and_then(|e| e.get("xmax")).and_then(|v| v.as_f64());
    let ymax = extent.and_then(|e| e.get("ymax")).and_then(|v| v.as_f64());
    let (xmin, ymin, xmax, ymax) = match (xmin, ymin, xmax, ymax) {
        (Some(xmin), Some(ymin), Some(xmax), Some(ymax)) => (xmin, ymin, xmax, ymax),
        _ => return Err(RuntimeError::new(Some("Invalid extent"))),
    };

    if wkid == Some(4326) {
        builder.scheme_spec = Some(SchemeSpec::Geographic(Rectangle::from_degrees(
            xmin, ymin, xmax, ymax,
        )));
    } else if wkid == Some(3857) {
        // Clamp extent to EPSG 3857 bounds
        let epsg3857_bounds = std::f64::consts::PI * builder.ellipsoid.maximum_radius();
        let mut xmax = xmax;
        let mut ymax = ymax;
        let mut xmin = xmin;
        let mut ymin = ymin;
        if xmax > epsg3857_bounds {
            xmax = epsg3857_bounds;
        }
        if ymax > epsg3857_bounds {
            ymax = epsg3857_bounds;
        }
        if xmin < -epsg3857_bounds {
            xmin = -epsg3857_bounds;
        }
        if ymin < -epsg3857_bounds {
            ymin = -epsg3857_bounds;
        }

        builder.scheme_spec = Some(SchemeSpec::WebMercator(
            Cartesian2::new(xmin, ymin),
            Cartesian2::new(xmax, ymax),
        ));
    } else {
        return Err(RuntimeError::new(Some("Invalid spatial reference")));
    }

    let tile_info = match metadata.get("tileInfo") {
        Some(tile_info) if !tile_info.is_null() => tile_info,
        _ => return Err(RuntimeError::new(Some("tileInfo is required"))),
    };

    let rows = tile_info.get("rows").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    let cols = tile_info.get("cols").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    builder.width = rows + 1;
    builder.height = cols + 1;
    builder.encoding =
        if tile_info.get("format").and_then(|v| v.as_str()) == Some("LERC") {
            HeightmapEncoding::Lerc
        } else {
            HeightmapEncoding::None
        };
    builder.lod_count = tile_info
        .get("lods")
        .and_then(|v| v.as_array())
        .map(|lods| lods.len() as i32 - 1)
        .unwrap_or(-1);

    let tiling_scheme = builder
        .scheme_spec
        .as_ref()
        .unwrap()
        .build(builder.ellipsoid);

    let has_availability = metadata
        .get("capabilities")
        .and_then(|v| v.as_str())
        .map(|capabilities| capabilities.contains("Tilemap"))
        .unwrap_or(false);
    builder.has_availability = has_availability;
    if has_availability {
        let mut tiles_available = TileAvailability::new(
            builder.scheme_spec.as_ref().unwrap().build(builder.ellipsoid),
            builder.lod_count,
        );
        tiles_available.add_available_tile_range(
            0,
            0,
            0,
            tiling_scheme.get_number_of_x_tiles_at_level(0),
            tiling_scheme.get_number_of_y_tiles_at_level(0),
        );
        builder.tiles_available = Some(tiles_available);
        builder.tiles_availability_loaded = Some(TileAvailability::new(
            builder.scheme_spec.as_ref().unwrap().build(builder.ellipsoid),
            builder.lod_count,
        ));
    }

    builder.level_zero_maximum_geometric_error =
        get_estimated_level_zero_geometric_error_for_a_heightmap(
            tiling_scheme.ellipsoid(),
            builder.width as f64,
            tiling_scheme.get_number_of_x_tiles_at_level(0),
        );

    if metadata
        .get("bandCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(1)
        > 1
    {
        println!(
            "ArcGISTiledElevationTerrainProvider: Terrain data has more than 1 band. Using the first one."
        );
    }

    let min_values = metadata.get("minValues");
    let max_values = metadata.get("maxValues");
    if min_values.is_some() && max_values.is_some() {
        builder.terrain_data_structure = Some(HeightmapStructureOptions {
            element_multiplier: Some(1.0),
            lowest_encoded_height: min_values
                .and_then(|v| v.get(0))
                .and_then(|v| v.as_f64()),
            highest_encoded_height: max_values
                .and_then(|v| v.get(0))
                .and_then(|v| v.as_f64()),
            ..Default::default()
        });
    } else {
        builder.terrain_data_structure = Some(HeightmapStructureOptions {
            element_multiplier: Some(1.0),
            ..Default::default()
        });
    }

    Ok(())
}

/// Mirrors `requestMetadata(terrainProviderBuilder, metadataResource, provider)`.
async fn request_metadata<B: ResourceBackend + ?Sized>(
    builder: &mut TerrainProviderBuilder,
    metadata_resource: &mut Resource,
    backend: &B,
) -> Result<(), RuntimeError> {
    match metadata_resource.fetch_json(backend).await {
        Ok(Some(metadata)) => match parse_metadata_success(builder, &metadata) {
            Ok(()) => Ok(()),
            Err(error) => {
                let message = format!(
                    "An error occurred while accessing {}.",
                    metadata_resource.url()
                );
                TileProviderError::report_error(None, message, None, None, None);
                Err(error)
            }
        },
        _ => {
            let message = format!(
                "An error occurred while accessing {}.",
                metadata_resource.url()
            );
            TileProviderError::report_error(None, message.clone(), None, None, None);
            Err(RuntimeError::new(Some(&message)))
        }
    }
}

/// A [`TerrainProvider`] that produces terrain geometry by tessellating height
/// maps retrieved from Elevation Tiles of an ArcGIS ImageService.
pub struct ArcGISTiledElevationTerrainProvider {
    resource: Option<Resource>,
    credit: Option<Credit>,
    tiling_scheme: Option<Box<dyn TilingScheme>>,
    level_zero_maximum_geometric_error: f64,
    terrain_data_structure: Option<HeightmapStructureOptions>,
    width: usize,
    height: usize,
    encoding: HeightmapEncoding,
    lod_count: i32,
    has_availability: bool,
    tiles_available: Option<TileAvailability>,
    tiles_availability_loaded: Option<TileAvailability>,
    available_cache: RefCell<HashMap<String, ()>>,
    error_event: Event,
}

impl ArcGISTiledElevationTerrainProvider {
    /// Mirrors the private JS constructor (call [`from_url`] instead).
    fn new_internal() -> Self {
        Self {
            resource: None,
            credit: None,
            tiling_scheme: None,
            level_zero_maximum_geometric_error: 0.0,
            terrain_data_structure: None,
            width: 0,
            height: 0,
            encoding: HeightmapEncoding::None,
            lod_count: 0,
            has_availability: false,
            tiles_available: None,
            tiles_availability_loaded: None,
            available_cache: RefCell::new(HashMap::new()),
            error_event: Event::new(),
        }
    }

    /// Creates a `TerrainProvider` that produces terrain geometry by
    /// tessellating height maps retrieved from Elevation Tiles of an ArcGIS
    /// ImageService.
    ///
    /// Mirrors `ArcGISTiledElevationTerrainProvider.fromUrl`.
    ///
    /// # Panics
    ///
    /// Panics with a `DeveloperError` when `url` is not provided.
    pub async fn from_url<B: ResourceBackend + ?Sized>(
        url: Option<&str>,
        options: Option<ArcGISTiledElevationTerrainProviderOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        //>>includeStart('debug', pragmas.debug);
        check::defined("url", url);
        //>>includeEnd('debug');

        Self::from_resource(Resource::new(url.unwrap().to_string()), options, backend).await
    }

    /// `fromUrl` overload accepting a [`Resource`] (JS `Resource.createIfNeeded`).
    pub async fn from_resource<B: ResourceBackend + ?Sized>(
        mut resource: Resource,
        options: Option<ArcGISTiledElevationTerrainProviderOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        let options = options.unwrap_or_default();

        resource.append_forward_slash();
        if let Some(token) = &options.token {
            let mut query_parameters = HashMap::new();
            query_parameters.insert("token".to_string(), token.clone());
            resource = resource.get_derived_resource_with_options(DerivedResourceOptions {
                query_parameters: Some(&query_parameters),
                ..Default::default()
            });
        }

        let mut query_parameters = HashMap::new();
        query_parameters.insert("f".to_string(), "pjson".to_string());
        let mut metadata_resource = resource.get_derived_resource_with_options(
            DerivedResourceOptions {
                query_parameters: Some(&query_parameters),
                ..Default::default()
            },
        );

        let mut terrain_provider_builder = TerrainProviderBuilder::new(&options);
        request_metadata(&mut terrain_provider_builder, &mut metadata_resource, backend).await?;

        let mut provider = Self::new_internal();
        terrain_provider_builder.build(&mut provider);
        provider.resource = Some(resource);

        Ok(provider)
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

    /// Gets an object that can be used to determine availability of terrain
    /// from this provider; `None` if availability information is not
    /// available.
    pub fn availability(&self) -> Option<&TileAvailability> {
        self.tiles_available.as_ref()
    }

    /// Whether the provider parses the `Tilemap` capability and tracks tile
    /// availability (exposed for spec parity with the JS `_hasAvailability`
    /// private field).
    pub fn has_availability(&self) -> bool {
        self.has_availability
    }

    /// Requests the geometry for a given tile. The result includes terrain
    /// data and indicates that all child tiles are available.
    ///
    /// Mirrors `requestTileGeometry` (DEVIATION 2: no `Request` throttling).
    pub async fn request_tile_geometry<B: ResourceBackend + ?Sized>(
        &mut self,
        x: i32,
        y: i32,
        level: i32,
        backend: &B,
    ) -> Result<Option<HeightmapTerrainData>, RuntimeError> {
        let mut tile_resource = self
            .resource
            .as_ref()
            .unwrap()
            .clone_resource()
            .get_derived_resource_with_options(DerivedResourceOptions {
                url: Some(&format!("tile/{level}/{y}/{x}")),
                ..Default::default()
            });

        let has_availability = self.has_availability;
        if has_availability && self.is_tile_available(level + 1, x * 2, y * 2).is_none() {
            // We need to load child availability
            self.request_availability(level + 1, x * 2, y * 2, backend)
                .await?;
        }

        let buffer = match tile_resource.fetch_array_buffer(backend).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(None),
            Err(error) => return Err(RuntimeError::new(Some(&format!("{error}")))),
        };

        let child_tile_mask = if has_availability {
            self.tiles_available
                .as_ref()
                .unwrap()
                .compute_child_mask_for_tile(level, x, y) as i32
        } else {
            ALL_CHILDREN
        };

        Ok(Some(HeightmapTerrainData::new(
            HeightmapTerrainDataOptions {
                buffer: Some(HeightmapBuffer::U8(buffer)),
                width: Some(self.width),
                height: Some(self.height),
                child_tile_mask: Some(child_tile_mask),
                structure: self.terrain_data_structure.clone(),
                encoding: Some(self.encoding),
                ..Default::default()
            },
        )))
    }

    /// Mirrors `isTileAvailable(that, level, x, y)`.
    fn is_tile_available(&self, level: i32, x: i32, y: i32) -> Option<bool> {
        if !self.has_availability {
            return None;
        }

        let tiles_availability_loaded = self.tiles_availability_loaded.as_ref().unwrap();
        let tiles_available = self.tiles_available.as_ref().unwrap();

        if level > self.lod_count {
            return Some(false);
        }

        // Check if tiles are known to be available
        if tiles_available.is_tile_available(level, x, y) {
            return Some(true);
        }

        // or to not be available
        if tiles_availability_loaded.is_tile_available(level, x, y) {
            return Some(false);
        }

        None
    }

    /// Mirrors `requestAvailability(that, level, x, y)` (DEVIATION 2: the
    /// in-flight cache stores no `Request`; it only dedupes by URL).
    async fn request_availability<B: ResourceBackend + ?Sized>(
        &mut self,
        level: i32,
        x: i32,
        y: i32,
        backend: &B,
    ) -> Result<Option<bool>, RuntimeError> {
        if !self.has_availability {
            return Ok(None);
        }

        // Fetch 128x128 availability list, so we make the minimum amount of
        // requests
        let x_offset = (x / 128) * 128;
        let y_offset = (y / 128) * 128;

        let dim = (1 << level).min(128);
        let url = format!("tilemap/{level}/{y_offset}/{x_offset}/{dim}/{dim}");

        let available_cache = self.available_cache.borrow_mut();
        if available_cache.contains_key(&url) {
            return Ok(None);
        }
        drop(available_cache);
        self.available_cache
            .borrow_mut()
            .insert(url.clone(), ());

        let mut tilemap_resource = self
            .resource
            .as_ref()
            .unwrap()
            .clone_resource()
            .get_derived_resource_with_options(DerivedResourceOptions {
                url: Some(&url),
                ..Default::default()
            });

        let result = match tilemap_resource.fetch_json(backend).await {
            Ok(Some(result)) => {
                let data: Vec<i64> = result
                    .get("data")
                    .and_then(|v| v.as_array())
                    .map(|values| {
                        values
                            .iter()
                            .map(|v| v.as_i64().unwrap_or(0))
                            .collect()
                    })
                    .unwrap_or_default();
                let available = compute_availability(x_offset, y_offset, dim, dim, &data);

                // Mark whole area as having availability loaded
                self.tiles_availability_loaded
                    .as_mut()
                    .unwrap()
                    .add_available_tile_range(level, x_offset, y_offset, x_offset + dim, y_offset + dim);

                let tiles_available = self.tiles_available.as_mut().unwrap();
                for range in &available {
                    tiles_available.add_available_tile_range(
                        level,
                        range.start_x,
                        range.start_y,
                        range.end_x,
                        range.end_y,
                    );
                }

                // Conveniently return availability of original tile
                self.is_tile_available(level, x, y)
            }
            Ok(None) => None,
            Err(error) => {
                self.available_cache.borrow_mut().remove(&url);
                return Err(RuntimeError::new(Some(&format!("{error}"))));
            }
        };

        self.available_cache.borrow_mut().remove(&url);
        Ok(result)
    }

    /// Makes sure we load availability data for a tile; always `None` for
    /// this provider (mirrors the JS `undefined` return).
    pub fn load_tile_data_availability(&self, _x: i32, _y: i32, _level: i32) -> Option<()> {
        None
    }
}

impl TerrainProvider for ArcGISTiledElevationTerrainProvider {
    fn tiling_scheme(&self) -> &dyn TilingScheme {
        self.tiling_scheme.as_ref().unwrap().as_ref()
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

    fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool> {
        // DEVIATION 3: the JS version additionally kicks off an asynchronous
        // availability load when unknown; that happens inside
        // `request_tile_geometry` in this port.
        self.is_tile_available(level, x, y)
    }
}

/// Mirrors `findRange(origin, width, height, data)`.
fn find_range(
    origin: &Cartesian2,
    width: i32,
    height: i32,
    data: &[i64],
) -> (Vec<Cartesian2>, AvailabilityRange, i64) {
    let ox = origin.x as i32;
    let oy = origin.y as i32;
    let end_col = width - 1;
    let end_row = height - 1;

    let value = data[oy as usize * width as usize + ox as usize];
    let mut ending_indices: Vec<Cartesian2> = Vec::new();
    let mut range = AvailabilityRange {
        start_x: ox,
        start_y: oy,
        end_x: 0,
        end_y: 0,
    };

    let mut corner = Cartesian2::new((ox + 1) as f64, (oy + 1) as f64);
    let mut done_x = false;
    let mut done_y = false;
    while !(done_x && done_y) {
        // We want to use the original value when checking Y,
        //  so get it before it possibly gets incremented
        let mut end_x = corner.x;

        // If we no longer move in the Y direction we need to check the
        // corner tile in X pass
        let end_y = if done_y { corner.y + 1.0 } else { corner.y };

        // Check X range
        if !done_x {
            let mut y = oy;
            while (y as f64) < end_y {
                if data[y as usize * width as usize + corner.x as usize] != value {
                    done_x = true;
                    break;
                }
                y += 1;
            }

            if done_x {
                ending_indices.push(Cartesian2::new(corner.x, oy as f64));

                // Use the last good column so we can continue with Y
                corner.x -= 1.0;
                end_x -= 1.0;
                range.end_x = corner.x as i32;
            } else if corner.x == end_col as f64 {
                range.end_x = corner.x as i32;
                done_x = true;
            } else {
                corner.x += 1.0;
            }
        }

        // Check Y range - The corner tile is checked here
        if !done_y {
            let col = corner.y as i32 * width;
            let mut x = ox;
            while (x as f64) <= end_x {
                if data[col as usize + x as usize] != value {
                    done_y = true;
                    break;
                }
                x += 1;
            }

            if done_y {
                ending_indices.push(Cartesian2::new(ox as f64, corner.y));

                // Use the last good row so we can continue with X
                corner.y -= 1.0;
                range.end_y = corner.y as i32;
            } else if corner.y == end_row as f64 {
                range.end_y = corner.y as i32;
                done_y = true;
            } else {
                corner.y += 1.0;
            }
        }
    }

    (ending_indices, range, value)
}

/// Mirrors `computeAvailability(x, y, width, height, data)`.
fn compute_availability(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    data: &[i64],
) -> Vec<AvailabilityRange> {
    let mut ranges: Vec<AvailabilityRange> = Vec::new();

    let single_value = data.iter().all(|val| *val == data[0]);
    if single_value {
        if data[0] == 1 {
            ranges.push(AvailabilityRange {
                start_x: x,
                start_y: y,
                end_x: x + width - 1,
                end_y: y + height - 1,
            });
        }

        return ranges;
    }

    let mut positions = vec![Cartesian2::new(0.0, 0.0)];
    while let Some(origin) = positions.pop() {
        let (ending_indices, mut range, value) = find_range(&origin, width, height, data);

        if value == 1 {
            // Convert range into the array into global tile coordinates
            range.start_x += x;
            range.end_x += x;
            range.start_y += y;
            range.end_y += y;
            ranges.push(range);
        }

        if !ending_indices.is_empty() {
            positions.extend(ending_indices);
        }
    }

    ranges
}
