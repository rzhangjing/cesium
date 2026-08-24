//! Ported from `packages/engine/Source/Core/VRTheWorldTerrainProvider.js`.
//!
//! A [`TerrainProvider`] that produces terrain geometry by tessellating height
//! maps retrieved from a VT MÄK VR-TheWorld server.
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `DataRectangle` | [`DataRectangle`] | identical |
//! | `TerrainProviderBuilder` | [`TerrainProviderBuilder`] | identical field set |
//! | `TerrainProviderBuilder.prototype.build` | [`TerrainProviderBuilder::build`] | identical |
//! | `metadataSuccess` | [`metadata_success`] | XML queried with a minimal parser instead of `DOMParser` (DEVIATION 1) |
//! | `metadataFailure` | [`metadata_failure`] | identical message composition |
//! | `requestMetadata` | inlined in [`VRTheWorldTerrainProvider::from_resource`] | `fetchXML` → `fetch_text` (DEVIATION 1) |
//! | `VRTheWorldTerrainProvider` constructor | [`VRTheWorldTerrainProvider::with_options`] | identical option defaults |
//! | `fromUrl` | [`VRTheWorldTerrainProvider::from_url`] / [`VRTheWorldTerrainProvider::from_resource`] | DEVIATION 2 |
//! | `requestTileGeometry` | [`VRTheWorldTerrainProvider::request_tile_geometry`] | DEVIATION 3 |
//! | `getChildMask` | [`get_child_mask`] | identical |
//! | `isTileInRectangle` | [`is_tile_in_rectangle`] | identical |
//! | properties / `getLevelMaximumGeometricError` / `getTileDataAvailable` / `loadTileDataAvailability` | accessor methods | identical values |
//!
//! # DEVIATIONS
//!
//! 1. HTTP access goes through the injected [`ResourceBackend`] and the XML
//!    metadata is fetched as text (`fetchXML` is browser-only, see the
//!    `resource` module DEVIATION table) and parsed with a minimal parser
//!    covering the `TileMap` elements used here (`SRS`, `TileFormat`,
//!    `DataExtent`) instead of `DOMParser`.
//! 2. `fromUrl` accepts a `Resource` or `&str` (JS also accepts promises,
//!    which Rust callers resolve before the call).
//! 3. JS requests the tile with `fetchImage({preferImageBitmap: true})` and
//!    converts it with `getImagePixels` (an RGBA pixel array); the Rust port
//!    fetches the tile body as bytes through the backend and uses them
//!    directly as the RGBA pixel buffer. `RequestScheduler` throttling is
//!    not modeled.

use std::collections::HashMap;

use crate::check;
use crate::credit::Credit;
use crate::ellipsoid::Ellipsoid;
use crate::event::Event;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::heightmap_terrain_data::{
    HeightmapBuffer, HeightmapStructureOptions, HeightmapTerrainData,
    HeightmapTerrainDataOptions,
};
use crate::math::CesiumMath;
use crate::rectangle::Rectangle;
use crate::resource::{DerivedResourceOptions, Resource, ResourceBackend};
use crate::runtime_error::RuntimeError;
use crate::terrain_provider::{
    get_estimated_level_zero_geometric_error_for_a_heightmap, TerrainProvider,
};
use crate::tile_provider_error::TileProviderError;
use crate::tiling_scheme::TilingScheme;

/// A data-extent rectangle and its maximum level, mirroring the private
/// `DataRectangle` constructor.
#[derive(Debug, Clone)]
pub struct DataRectangle {
    /// The extent of the data.
    pub rectangle: Rectangle,
    /// The maximum level of the data inside the rectangle.
    pub max_level: i32,
}

/// Initialization options for [`VRTheWorldTerrainProvider`].
///
/// Mirrors `VRTheWorldTerrainProvider.ConstructorOptions`.
#[derive(Default, Clone)]
pub struct VRTheWorldTerrainProviderOptions {
    /// The ellipsoid (defaults to WGS84).
    pub ellipsoid: Option<Ellipsoid>,
    /// A credit for the data source.
    pub credit: Option<String>,
}

/// Used to track creation details while fetching initial metadata.
///
/// Mirrors the private `TerrainProviderBuilder`.
struct TerrainProviderBuilder {
    ellipsoid: Ellipsoid,
    tiling_scheme: Option<GeographicTilingScheme>,
    heightmap_width: usize,
    heightmap_height: usize,
    level_zero_maximum_geometric_error: f64,
    rectangles: Vec<DataRectangle>,
}

impl TerrainProviderBuilder {
    fn new(options: &VRTheWorldTerrainProviderOptions) -> Self {
        Self {
            ellipsoid: options.ellipsoid.unwrap_or(Ellipsoid::WGS84),
            tiling_scheme: None,
            heightmap_width: 0,
            heightmap_height: 0,
            level_zero_maximum_geometric_error: 0.0,
            rectangles: Vec::new(),
        }
    }

    fn build(self, provider: &mut VRTheWorldTerrainProvider) {
        provider.tiling_scheme = self
            .tiling_scheme
            .map(|scheme| Box::new(scheme) as Box<dyn TilingScheme>);
        provider.heightmap_width = self.heightmap_width;
        provider.heightmap_height = self.heightmap_height;
        provider.level_zero_maximum_geometric_error = self.level_zero_maximum_geometric_error;
        provider.rectangles = self.rectangles;
    }
}

/// Mirrors `metadataSuccess(terrainProviderBuilder, xml)`.
///
/// DEVIATION: the `xml` document is represented as the raw XML text and the
/// needed elements/attributes are extracted with a minimal parser (JS uses
/// `DOMParser` + `getElementsByTagName`).
fn metadata_success(builder: &mut TerrainProviderBuilder, xml: &str) -> Result<(), RuntimeError> {
    let srs = element_text(xml, "SRS").ok_or_else(|| {
        RuntimeError::new(Some("The TileMap XML does not contain an SRS element."))
    })?;
    if srs == "EPSG:4326" {
        builder.tiling_scheme = Some(GeographicTilingScheme::new(
            Some(builder.ellipsoid),
            None,
            None,
            None,
        ));
    } else {
        return Err(RuntimeError::new(Some(&format!("SRS {srs} is not supported"))));
    }

    let tile_format = element_tag(xml, "TileFormat").ok_or_else(|| {
        RuntimeError::new(Some("The TileMap XML does not contain a TileFormat element."))
    })?;
    builder.heightmap_width = element_attribute(&tile_format, "width")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| {
            RuntimeError::new(Some("The TileFormat element has no valid width attribute."))
        })?;
    builder.heightmap_height = element_attribute(&tile_format, "height")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| {
            RuntimeError::new(Some("The TileFormat element has no valid height attribute."))
        })?;
    builder.level_zero_maximum_geometric_error =
        get_estimated_level_zero_geometric_error_for_a_heightmap(
            &builder.ellipsoid,
            (builder.heightmap_width.min(builder.heightmap_height)) as f64,
            builder
                .tiling_scheme
                .as_ref()
                .unwrap()
                .get_number_of_x_tiles_at_level(0),
        );

    for data_rectangle in element_tags(xml, "DataExtent") {
        let west = CesiumMath::to_radians(
            element_attribute(&data_rectangle, "minx")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0.0),
        );
        let south = CesiumMath::to_radians(
            element_attribute(&data_rectangle, "miny")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0.0),
        );
        let east = CesiumMath::to_radians(
            element_attribute(&data_rectangle, "maxx")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0.0),
        );
        let north = CesiumMath::to_radians(
            element_attribute(&data_rectangle, "maxy")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0.0),
        );
        let max_level = element_attribute(&data_rectangle, "maxlevel")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);

        builder.rectangles.push(DataRectangle {
            rectangle: Rectangle::from_radians(west, south, east, north),
            max_level,
        });
    }

    Ok(())
}

/// Mirrors `metadataFailure(resource, error, provider)`.
fn metadata_failure(resource: &Resource, error: Option<&str>) -> RuntimeError {
    let mut message = format!("An error occurred while accessing {}", resource.url());
    if let Some(error) = error {
        message.push_str(&format!(": {error}"));
    }

    TileProviderError::report_error(None, message.clone(), None, None, None);

    RuntimeError::new(Some(&message))
}

/// A [`TerrainProvider`] that produces terrain geometry by tessellating height
/// maps retrieved from a VT MÄK VR-TheWorld server.
pub struct VRTheWorldTerrainProvider {
    error_event: Event,
    terrain_data_structure: HeightmapStructureOptions,
    credit: Option<Credit>,
    tiling_scheme: Option<Box<dyn TilingScheme>>,
    rectangles: Vec<DataRectangle>,
    heightmap_width: usize,
    heightmap_height: usize,
    level_zero_maximum_geometric_error: f64,
    resource: Option<Resource>,
}

impl VRTheWorldTerrainProvider {
    /// Mirrors the private JS constructor (call [`from_url`] instead).
    fn with_options(options: Option<VRTheWorldTerrainProviderOptions>) -> Self {
        let options = options.unwrap_or_default();

        let credit = options.credit.map(|credit| Credit::new(&credit, false));

        Self {
            error_event: Event::new(),
            terrain_data_structure: HeightmapStructureOptions {
                height_scale: Some(1.0 / 1000.0),
                height_offset: Some(-1000.0),
                elements_per_height: Some(3),
                stride: Some(4),
                element_multiplier: Some(256.0),
                is_big_endian: Some(true),
                lowest_encoded_height: Some(0.0),
                highest_encoded_height: Some(256.0 * 256.0 * 256.0 - 1.0),
            },
            credit,
            tiling_scheme: None,
            rectangles: Vec::new(),
            heightmap_width: 0,
            heightmap_height: 0,
            level_zero_maximum_geometric_error: 0.0,
            resource: None,
        }
    }

    /// Creates a `VRTheWorldTerrainProvider` from the URL of the VR-TheWorld
    /// TileMap.
    ///
    /// Mirrors `VRTheWorldTerrainProvider.fromUrl`.
    ///
    /// # Panics
    ///
    /// Panics with a `DeveloperError` when `url` is not provided.
    pub async fn from_url<B: ResourceBackend + ?Sized>(
        url: Option<&str>,
        options: Option<VRTheWorldTerrainProviderOptions>,
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
        options: Option<VRTheWorldTerrainProviderOptions>,
        backend: &B,
    ) -> Result<Self, RuntimeError> {
        let options = options.unwrap_or_default();

        let mut terrain_provider_builder = TerrainProviderBuilder::new(&options);

        match resource.fetch_text(backend).await {
            Ok(Some(xml)) => {
                if let Err(error) = metadata_success(&mut terrain_provider_builder, &xml) {
                    return Err(metadata_failure(&resource, Some(&error.message)));
                }
            }
            Ok(None) => return Err(metadata_failure(&resource, None)),
            Err(error) => {
                return Err(metadata_failure(&resource, Some(&format!("{error}"))));
            }
        }

        let mut provider = Self::with_options(Some(options));
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

    /// Gets the availability object; always `None` for this provider.
    pub fn availability(&self) -> Option<&crate::tile_availability::TileAvailability> {
        None
    }

    /// Requests the geometry for a given tile. The result includes terrain
    /// data and indicates that all child tiles are available.
    ///
    /// Mirrors `requestTileGeometry` (DEVIATION 3: the tile body is fetched
    /// as bytes instead of an image).
    pub async fn request_tile_geometry<B: ResourceBackend + ?Sized>(
        &self,
        x: i32,
        y: i32,
        level: i32,
        backend: &B,
    ) -> Result<Option<HeightmapTerrainData>, RuntimeError> {
        let tiling_scheme = self.tiling_scheme.as_ref().unwrap();
        let y_tiles = tiling_scheme.get_number_of_y_tiles_at_level(level);
        let mut query = HashMap::new();
        query.insert("cesium".to_string(), "true".to_string());
        let mut resource = self
            .resource
            .as_ref()
            .unwrap()
            .clone_resource()
            .get_derived_resource_with_options(DerivedResourceOptions {
                url: Some(&format!("{level}/{x}/{}.tif", y_tiles - y - 1)),
                query_parameters: Some(&query),
                ..Default::default()
            });

        let pixels = match resource.fetch_array_buffer(backend).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(None),
            Err(error) => return Err(RuntimeError::new(Some(&format!("{error}")))),
        };

        Ok(Some(HeightmapTerrainData::new(
            HeightmapTerrainDataOptions {
                buffer: Some(HeightmapBuffer::U8(pixels)),
                width: Some(self.heightmap_width),
                height: Some(self.heightmap_height),
                child_tile_mask: Some(get_child_mask(self, x, y, level)),
                structure: Some(self.terrain_data_structure.clone()),
                ..Default::default()
            },
        )))
    }

    /// Makes sure we load availability data for a tile; always `None` for
    /// this provider (mirrors the JS `undefined` return).
    pub fn load_tile_data_availability(&self, _x: i32, _y: i32, _level: i32) -> Option<()> {
        None
    }
}

impl TerrainProvider for VRTheWorldTerrainProvider {
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

    fn get_tile_data_available(&self, _x: i32, _y: i32, _level: i32) -> Option<bool> {
        None
    }
}

/// Mirrors `getChildMask(provider, x, y, level)`.
fn get_child_mask(provider: &VRTheWorldTerrainProvider, x: i32, y: i32, level: i32) -> i32 {
    let tiling_scheme = provider.tiling_scheme.as_ref().unwrap();
    let rectangles = &provider.rectangles;
    let mut parent_rectangle = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
    tiling_scheme.tile_xy_to_rectangle(x, y, level, &mut parent_rectangle);

    let mut child_mask = 0i32;

    let mut i = 0;
    while i < rectangles.len() && child_mask != 15 {
        let data_rectangle = &rectangles[i];
        i += 1;
        if data_rectangle.max_level <= level {
            continue;
        }

        let test_rectangle = &data_rectangle.rectangle;

        if Rectangle::intersection(test_rectangle, &parent_rectangle).is_some() {
            // Parent tile is inside this rectangle, so at least one child
            // is, too.
            if is_tile_in_rectangle(tiling_scheme.as_ref(), test_rectangle, x * 2, y * 2, level + 1)
            {
                child_mask |= 4; // northwest
            }
            if is_tile_in_rectangle(
                tiling_scheme.as_ref(),
                test_rectangle,
                x * 2 + 1,
                y * 2,
                level + 1,
            ) {
                child_mask |= 8; // northeast
            }
            if is_tile_in_rectangle(
                tiling_scheme.as_ref(),
                test_rectangle,
                x * 2,
                y * 2 + 1,
                level + 1,
            ) {
                child_mask |= 1; // southwest
            }
            if is_tile_in_rectangle(
                tiling_scheme.as_ref(),
                test_rectangle,
                x * 2 + 1,
                y * 2 + 1,
                level + 1,
            ) {
                child_mask |= 2; // southeast
            }
        }
    }

    child_mask
}

/// Mirrors `isTileInRectangle(tilingScheme, rectangle, x, y, level)`.
fn is_tile_in_rectangle(
    tiling_scheme: &dyn TilingScheme,
    rectangle: &Rectangle,
    x: i32,
    y: i32,
    level: i32,
) -> bool {
    let mut tile_rectangle = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
    tiling_scheme.tile_xy_to_rectangle(x, y, level, &mut tile_rectangle);
    Rectangle::intersection(&tile_rectangle, rectangle).is_some()
}

// ── Minimal TileMap XML querying (DEVIATION 1) ─────────────────────────

/// Returns the text content of the first `<name>...</name>` element.
fn element_text(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

/// Returns the first complete tag (including attributes) named `name`,
/// i.e. the content of `<name ...>` up to `>` or `/>`.
fn element_tag(xml: &str, name: &str) -> Option<String> {
    element_tags(xml, name).into_iter().next()
}

/// Returns every tag (including attributes) named `name`.
fn element_tags(xml: &str, name: &str) -> Vec<String> {
    let open = format!("<{name}");
    let mut tags = Vec::new();
    let mut search_from = 0;
    while let Some(pos) = xml[search_from..].find(&open) {
        let tag_start = search_from + pos;
        let after = tag_start + open.len();
        // The character after the element name must delimit it (space,
        // `>` or `/`) so e.g. `<TileFormatExtra>` does not match.
        let delimiter_ok = xml
            .as_bytes()
            .get(after)
            .map(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'/'))
            .unwrap_or(false);
        if !delimiter_ok {
            search_from = after;
            continue;
        }
        let rest = &xml[after..];
        let end = match rest.find('>') {
            Some(end) => end,
            None => break,
        };
        tags.push(rest[..end].to_string());
        search_from = after + end + 1;
    }
    tags
}

/// Returns the value of `attribute="value"` inside a tag string.
fn element_attribute(tag: &str, attribute: &str) -> Option<String> {
    let pattern = format!("{attribute}=\"");
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}
