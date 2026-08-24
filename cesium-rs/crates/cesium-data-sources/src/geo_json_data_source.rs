//! Ported from `packages/engine/Source/DataSources/GeoJsonDataSource.js`.
//!
//! A data source that loads GeoJSON (and TopoJSON) data into entities,
//! honouring the simplestyle-spec properties.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use cesium_core::arc_type::ArcType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::credit::Credit;
use cesium_core::event::Event;
use cesium_core::get_filename_from_uri::get_filename_from_uri;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::vertical_origin::VerticalOrigin;
use serde_json::Value;

use crate::billboard_graphics::BillboardGraphics;
use crate::data_source::DataSource;
use crate::entity::Entity;
use crate::entity_collection::EntityCollection;
use crate::polygon_graphics::PolygonGraphics;
use crate::polyline_graphics::PolylineGraphics;
use crate::property::PropertyResult;
use crate::property_bag::PropertyBag;

// ============================================================================
// Static defaults (mirror of the module-level `default*` variables and the
// static `GeoJsonDataSource.markerSize/...` property accessors).
// ============================================================================

/// The global default styling values used when no load options are given.
///
/// Mirrors the module-level `defaultMarkerSize`, `defaultMarkerSymbol`,
/// `defaultMarkerColor`, `defaultStroke`, `defaultStrokeWidth`, `defaultFill`
/// and `defaultClampToGround` variables of GeoJsonDataSource.js.
#[derive(Debug, Clone)]
pub struct GeoJsonDefaults {
    /// The default size of the map pin created for each point, in pixels.
    pub marker_size: f64,
    /// The default symbol of the map pin created for each point.
    pub marker_symbol: Option<String>,
    /// The default color of the map pin created for each point.
    pub marker_color: Color,
    /// The default color of polylines and polygon outlines.
    pub stroke: Color,
    /// The default width of polylines and polygon outlines.
    pub stroke_width: f64,
    /// The default color for polygon interiors.
    pub fill: Color,
    /// The default of whether to clamp to the ground.
    pub clamp_to_ground: bool,
}

impl Default for GeoJsonDefaults {
    fn default() -> Self {
        Self {
            marker_size: 48.0,
            marker_symbol: None,
            marker_color: Color::ROYALBLUE,
            stroke: Color::YELLOW,
            stroke_width: 2.0,
            // Color.fromBytes(255, 255, 0, 100)
            fill: Color::from_bytes(255, 255, 0, 100),
            clamp_to_ground: false,
        }
    }
}

static DEFAULTS: LazyLock<Mutex<GeoJsonDefaults>> =
    LazyLock::new(|| Mutex::new(GeoJsonDefaults::default()));

/// Returns a snapshot of the current global default styling values.
pub fn defaults() -> GeoJsonDefaults {
    DEFAULTS.lock().unwrap().clone()
}

/// Updates the global default styling values (mirrors the static setters
/// `GeoJsonDataSource.markerSize = ...` etc.).
pub fn set_defaults(update: impl FnOnce(&mut GeoJsonDefaults)) {
    let mut guard = DEFAULTS.lock().unwrap();
    update(&mut guard);
}

/// Resets the global defaults to the original CesiumJS values.
pub fn reset_defaults() {
    *DEFAULTS.lock().unwrap() = GeoJsonDefaults::default();
}

// ============================================================================
// CRS handling (mirror of `crsNames`, `crsLinkHrefs`, `crsLinkTypes`).
// ============================================================================

/// A coordinate transformation function: takes a GeoJSON coordinate
/// (`[lon, lat]` or `[lon, lat, height]`) and returns a WGS84 Earth-fixed
/// [`Cartesian3`].
pub type CrsFunction = Arc<dyn Fn(&[f64]) -> Cartesian3 + Send + Sync>;

/// A resolver registered for a crs link `href` or `type`: takes the crs
/// `properties` object and returns the [`CrsFunction`] to use.
pub type CrsLinkResolver = Arc<dyn Fn(&Value) -> Result<CrsFunction, String> + Send + Sync>;

fn default_crs_function() -> CrsFunction {
    Arc::new(|coordinates: &[f64]| {
        Cartesian3::from_degrees_new(
            coordinates.first().copied().unwrap_or(f64::NAN),
            coordinates.get(1).copied().unwrap_or(f64::NAN),
            coordinates.get(2).copied(),
            None,
        )
    })
}

/// The set of crs names resolved to the default (WGS84 lon/lat) function,
/// mirroring the module-level `crsNames` object.
const CRS_NAMES: &[&str] = &[
    "urn:ogc:def:crs:OGC:1.3:CRS84",
    "EPSG:4326",
    "urn:ogc:def:crs:EPSG::4326",
];

static CRS_LINK_HREFS: LazyLock<Mutex<HashMap<String, CrsLinkResolver>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CRS_LINK_TYPES: LazyLock<Mutex<HashMap<String, CrsLinkResolver>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Registers a crs link resolver by `href` (mirrors adding an entry to
/// `GeoJsonDataSource.crsLinkHrefs`).
pub fn register_crs_link_href(href: &str, resolver: CrsLinkResolver) {
    CRS_LINK_HREFS
        .lock()
        .unwrap()
        .insert(href.to_string(), resolver);
}

/// Removes a crs link resolver registered with [`register_crs_link_href`].
pub fn unregister_crs_link_href(href: &str) {
    CRS_LINK_HREFS.lock().unwrap().remove(href);
}

/// Registers a crs link resolver by link `type` (mirrors adding an entry to
/// `GeoJsonDataSource.crsLinkTypes`).
pub fn register_crs_link_type(link_type: &str, resolver: CrsLinkResolver) {
    CRS_LINK_TYPES
        .lock()
        .unwrap()
        .insert(link_type.to_string(), resolver);
}

/// Removes a crs link resolver registered with [`register_crs_link_type`].
pub fn unregister_crs_link_type(link_type: &str) {
    CRS_LINK_TYPES.lock().unwrap().remove(link_type);
}

// ============================================================================
// Pin builder stand-in.
//
// DEVIATION: CesiumJS renders the pin into an HTMLCanvasElement via
// PinBuilder (canvas 2D drawing). There is no canvas in this port and
// cesium_core::pin_builder is still a stub, so the billboard `image` stores
// a deterministic descriptor string of the pin instead. The descriptor
// encodes the exact same inputs (symbol/text/maki id, color, size) as the
// JS PinBuilder calls, so specs can compare pins for equality.
// ============================================================================

/// Descriptor of a pin created from a plain color
/// (mirror of `PinBuilder.fromColor`).
#[must_use]
pub fn pin_from_color(color: Color, size: f64) -> String {
    format!(
        "pin:color:{},{},{},{};{}",
        color.red, color.green, color.blue, color.alpha, size
    )
}

/// Descriptor of a pin stamped with text (mirror of `PinBuilder.fromText`).
#[must_use]
pub fn pin_from_text(text: &str, color: Color, size: f64) -> String {
    format!(
        "pin:text:{};{},{},{},{};{}",
        text, color.red, color.green, color.blue, color.alpha, size
    )
}

/// Descriptor of a pin using a Maki icon (mirror of
/// `PinBuilder.fromMakiIconId`). Returns `None` when the icon id is not a
/// known Maki icon — in CesiumJS the image load fails and the caller falls
/// back to [`pin_from_color`].
#[must_use]
pub fn pin_from_maki_icon_id(id: &str, color: Color, size: f64) -> Option<String> {
    if !MAKI_ICON_IDS.contains(&id) {
        return None;
    }
    Some(format!(
        "pin:maki:{};{},{},{},{};{}",
        id, color.red, color.green, color.blue, color.alpha, size
    ))
}

/// The valid Maki icon ids — the basenames of
/// `packages/engine/Source/Assets/Textures/maki/*.png`.
const MAKI_ICON_IDS: &[&str] = &[
    "airfield",
    "airport",
    "alcohol-shop",
    "america-football",
    "art-gallery",
    "bakery",
    "bank",
    "bar",
    "baseball",
    "basketball",
    "beer",
    "bicycle",
    "building",
    "bus",
    "cafe",
    "camera",
    "campsite",
    "car",
    "cemetery",
    "cesium",
    "chemist",
    "cinema",
    "circle",
    "circle-stroked",
    "city",
    "clothing-store",
    "college",
    "commercial",
    "cricket",
    "cross",
    "dam",
    "danger",
    "disability",
    "dog-park",
    "embassy",
    "emergency-telephone",
    "entrance",
    "farm",
    "fast-food",
    "ferry",
    "fire-station",
    "fuel",
    "garden",
    "gift",
    "golf",
    "grocery",
    "hairdresser",
    "harbor",
    "heart",
    "heliport",
    "hospital",
    "ice-cream",
    "industrial",
    "land-use",
    "laundry",
    "library",
    "lighthouse",
    "lodging",
    "logging",
    "london-underground",
    "marker",
    "marker-stroked",
    "minefield",
    "mobilephone",
    "monument",
    "museum",
    "music",
    "oil-well",
    "park",
    "park2",
    "parking",
    "parking-garage",
    "pharmacy",
    "pitch",
    "place-of-worship",
    "playground",
    "police",
    "polling-place",
    "post",
    "prison",
    "rail",
    "rail-above",
    "rail-light",
    "rail-metro",
    "rail-underground",
    "religious-christian",
    "religious-jewish",
    "religious-muslim",
    "restaurant",
    "roadblock",
    "rocket",
    "school",
    "scooter",
    "shop",
    "skiing",
    "slaughterhouse",
    "soccer",
    "square",
    "square-stroked",
    "star",
    "star-stroked",
    "suitcase",
    "swimming",
    "telephone",
    "tennis",
    "theatre",
    "toilets",
    "town",
    "town-hall",
    "triangle",
    "triangle-stroked",
    "village",
    "warehouse",
    "waste-basket",
    "water",
    "wetland",
    "zoo",
];

// ============================================================================
// Load options and describe callback.
// ============================================================================

/// A function that generates a description string from the feature
/// properties (mirror of the `GeoJsonDataSource.describe` callback:
/// `(properties, nameProperty) => string`).
pub type DescribeFn = Arc<dyn Fn(&Value, Option<&str>) -> String + Send + Sync>;

/// Initialization options for the load/process methods.
///
/// Mirrors `GeoJsonDataSource.LoadOptions`; `None` fields fall back to the
/// global defaults, exactly like `options.x ?? defaultX` in CesiumJS.
#[derive(Default, Clone)]
pub struct GeoJsonLoadOptions {
    /// Overrides the url to use for resolving relative links (and for
    /// deriving the data source name).
    pub source_uri: Option<String>,
    /// The describe callback used to generate entity descriptions.
    pub describe: Option<DescribeFn>,
    /// The default size of the map pin created for each point, in pixels.
    pub marker_size: Option<f64>,
    /// The default symbol of the map pin created for each point.
    pub marker_symbol: Option<String>,
    /// The default color of the map pin created for each point.
    pub marker_color: Option<Color>,
    /// The default color of polylines and polygon outlines.
    pub stroke: Option<Color>,
    /// The default width of polylines and polygon outlines.
    pub stroke_width: Option<f64>,
    /// The default color for polygon interiors.
    pub fill: Option<Color>,
    /// Whether the geometry features are clamped to the ground.
    pub clamp_to_ground: Option<bool>,
    /// A credit for the data source (HTML string).
    pub credit: Option<String>,
}

/// The options resolved against the global defaults (mirror of the options
/// object built inside `preload`).
#[derive(Clone)]
struct ResolvedOptions {
    describe: DescribeFn,
    marker_size: f64,
    marker_symbol: Option<String>,
    marker_color: Color,
    stroke_width: f64,
    stroke: Color,
    fill: Color,
    clamp_to_ground: bool,
}

/// The default describe function (mirror of `defaultDescribe`): renders the
/// properties as an HTML table, skipping the name property and the
/// simplestyle identifiers.
#[must_use]
pub fn default_describe(properties: &Value, name_property: Option<&str>) -> String {
    let mut html = String::new();
    if let Some(object) = properties.as_object() {
        for (key, value) in object {
            if name_property == Some(key.as_str())
                || SIMPLE_STYLE_IDENTIFIERS.contains(&key.as_str())
            {
                continue;
            }
            // `defined(value)` — null and undefined are skipped.
            if value.is_null() {
                continue;
            }
            if value.is_object() {
                html.push_str(&format!(
                    "<tr><th>{}</th><td>{}</td></tr>",
                    key,
                    default_describe(value, None)
                ));
            } else {
                html.push_str(&format!(
                    "<tr><th>{}</th><td>{}</td></tr>",
                    key,
                    json_to_display_string(value)
                ));
            }
        }
    }

    if !html.is_empty() {
        html = format!(
            "<table class=\"cesium-infoBox-defaultTable\"><tbody>{}</tbody></table>",
            html
        );
    }

    html
}

fn default_describe_property(properties: &Value, name_property: Option<&str>) -> String {
    default_describe(properties, name_property)
}

/// The simplestyle-spec property names excluded from the generated
/// description (mirror of `simpleStyleIdentifiers`).
const SIMPLE_STYLE_IDENTIFIERS: &[&str] = &[
    "title",
    "description",
    "marker-size",
    "marker-symbol",
    "marker-color",
    "stroke",
    "stroke-opacity",
    "stroke-width",
    "fill",
    "fill-opacity",
];

/// The pixel sizes of the `marker-size` simplestyle values
/// (mirror of `sizes`).
fn marker_size_from_simplestyle(value: &str) -> Option<f64> {
    match value {
        "small" => Some(24.0),
        "medium" => Some(48.0),
        "large" => Some(64.0),
        _ => None,
    }
}

// ============================================================================
// GeoJsonDataSource
// ============================================================================

/// A [`DataSource`] which processes both GeoJSON and TopoJSON data.
/// simplestyle-spec properties are used when present.
///
/// Port of `GeoJsonDataSource`.
pub struct GeoJsonDataSource {
    name: Option<String>,
    entity_collection: EntityCollection,
    is_loading: bool,
    is_destroyed: bool,
    credit: Option<Credit>,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
}

impl GeoJsonDataSource {
    /// Creates a new GeoJSON data source.
    pub fn new() -> Self {
        Self {
            name: None,
            entity_collection: EntityCollection::new(),
            is_loading: false,
            is_destroyed: false,
            credit: None,
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
        }
    }

    /// Returns the entity collection of this data source.
    pub fn entities(&self) -> &EntityCollection {
        &self.entity_collection
    }

    /// Returns the credit of this data source, if any.
    pub fn credit(&self) -> Option<&Credit> {
        self.credit.as_ref()
    }

    /// Returns the human-readable name, or `None` when not set yet
    /// (mirrors the JS `name` property which is `undefined` initially).
    pub fn display_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets the human-readable name; raises the changed event when the
    /// value actually changes (mirror of the `name` property setter).
    pub fn set_name(&mut self, value: &str) {
        if self.name.as_deref() != Some(value) {
            self.name = Some(value.to_string());
            self.changed_event.raise_event(&());
        }
    }

    /// Returns whether this data source should be displayed. Delegates to
    /// the underlying entity collection (mirror of the `show` property).
    pub fn show(&self) -> bool {
        self.entity_collection.show
    }

    /// Sets whether this data source should be displayed.
    pub fn set_show(&mut self, show: bool) {
        self.entity_collection.show = show;
    }

    /// Returns the changed event.
    pub fn changed_event(&self) -> &Event {
        &self.changed_event
    }

    /// Returns the error event.
    pub fn error_event(&self) -> &Event {
        &self.error_event
    }

    /// Returns the loading event.
    pub fn loading_event(&self) -> &Event {
        &self.loading_event
    }

    /// Returns whether the data source is currently loading data.
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Updates the data source to the provided time. This data source is
    /// static, so it is always ready (mirror of `update`).
    pub fn update(&self, _time: f64) -> bool {
        true
    }

    /// Loads GeoJSON from a JSON string, replacing any existing data
    /// (mirror of `load` with a resolved promise/object).
    pub fn load_json(
        &mut self,
        json: &str,
        options: &GeoJsonLoadOptions,
    ) -> Result<(), String> {
        let value: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.load_value(&value, options)
    }

    /// Processes GeoJSON from a JSON string without replacing existing data
    /// (mirror of `process`).
    pub fn process_json(
        &mut self,
        json: &str,
        options: &GeoJsonLoadOptions,
    ) -> Result<(), String> {
        let value: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.process_value(&value, options)
    }

    /// Loads an already-parsed GeoJSON/TopoJSON value, replacing any
    /// existing data (mirror of `load`).
    pub fn load_value(
        &mut self,
        geo_json: &Value,
        options: &GeoJsonLoadOptions,
    ) -> Result<(), String> {
        self.preload(geo_json, options, true)
    }

    /// Processes an already-parsed GeoJSON/TopoJSON value without replacing
    /// existing data (mirror of `process`).
    pub fn process_value(
        &mut self,
        geo_json: &Value,
        options: &GeoJsonLoadOptions,
    ) -> Result<(), String> {
        self.preload(geo_json, options, false)
    }

    /// Loads GeoJSON from a file on disk (the filesystem analogue of
    /// `load(url)`); the data source name is derived from the file name.
    pub fn load_file(
        &mut self,
        path: &str,
        options: &GeoJsonLoadOptions,
    ) -> Result<(), String> {
        let mut options = options.clone();
        if options.source_uri.is_none() {
            // Normalize the separators so `get_filename_from_uri` (which
            // splits on '/') also works for Windows paths.
            options.source_uri = Some(path.replace('\\', "/"));
        }
        // Mirror the JS `load(url)` flow: fetching/parsing failures are
        // also routed through the loading state and the error event.
        self.set_loading(true);
        let result = (|| -> Result<(), String> {
            let contents =
                std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
            let value: Value = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            self.preload_inner(&value, &options, true)
        })();
        self.set_loading(false);
        if result.is_err() {
            self.error_event.raise_event(&());
        }
        result
    }

    /// Sets the loading state, raising the loading event on change
    /// (mirror of `DataSource.setLoading`).
    fn set_loading(&mut self, value: bool) {
        if self.is_loading != value {
            self.is_loading = value;
            self.loading_event.raise_event(&());
        }
    }

    /// Mirror of `preload`: resolves the options against the defaults,
    /// toggles the loading state and routes errors to the error event.
    fn preload(
        &mut self,
        geo_json: &Value,
        options: &GeoJsonLoadOptions,
        clear: bool,
    ) -> Result<(), String> {
        self.set_loading(true);
        let result = self.preload_inner(geo_json, options, clear);
        self.set_loading(false);
        if result.is_err() {
            self.error_event.raise_event(&());
        }
        result
    }

    /// The option-resolving core of [`GeoJsonDataSource::preload`], shared
    /// with [`GeoJsonDataSource::load_file`] which manages the loading
    /// state around the fetch + parse step itself.
    fn preload_inner(
        &mut self,
        geo_json: &Value,
        options: &GeoJsonLoadOptions,
        clear: bool,
    ) -> Result<(), String> {
        // User specified credit.
        self.credit = options
            .credit
            .as_ref()
            .map(|html| Credit::new(html, false));

        let defaults = defaults();
        let resolved = ResolvedOptions {
            describe: options.describe.clone().unwrap_or_else(|| {
                Arc::new(|properties: &Value, name_property: Option<&str>| {
                    default_describe_property(properties, name_property)
                })
            }),
            marker_size: options.marker_size.unwrap_or(defaults.marker_size),
            marker_symbol: options
                .marker_symbol
                .clone()
                .or_else(|| defaults.marker_symbol.clone()),
            marker_color: options.marker_color.unwrap_or(defaults.marker_color),
            stroke_width: options.stroke_width.unwrap_or(defaults.stroke_width),
            stroke: options.stroke.unwrap_or(defaults.stroke),
            fill: options.fill.unwrap_or(defaults.fill),
            clamp_to_ground: options
                .clamp_to_ground
                .unwrap_or(defaults.clamp_to_ground),
        };

        let result = self.load_inner(geo_json, &resolved, options.source_uri.as_deref(), clear);
        result
    }

    /// Mirror of the module-level `load` function.
    fn load_inner(
        &mut self,
        geo_json: &Value,
        options: &ResolvedOptions,
        source_uri: Option<&str>,
        clear: bool,
    ) -> Result<(), String> {
        if let Some(source_uri) = source_uri {
            let name = get_filename_from_uri(Some(source_uri));
            if !name.is_empty() && self.name.as_deref() != Some(name.as_str()) {
                self.name = Some(name);
                self.changed_event.raise_event(&());
            }
        }

        let type_handler = geo_json
            .get("type")
            .map(json_to_display_string)
            .unwrap_or_else(|| "undefined".to_string());
        if !is_supported_object_type(&type_handler) {
            return Err(format!(
                "Unsupported GeoJSON object type: {}",
                type_handler
            ));
        }

        // Check for a Coordinate Reference System.
        let crs_function = resolve_crs(geo_json.get("crs"))?;

        if clear {
            self.entity_collection.remove_all();
        }

        // null is a valid value for the crs, but means the entire load
        // process becomes a no-op because we can't assume anything about
        // the coordinates.
        if let Some(crs_function) = crs_function {
            self.process_object(geo_json, geo_json, &crs_function, options)?;
        }

        Ok(())
    }

    /// Dispatches a top-level GeoJSON object to its handler
    /// (mirror of `geoJsonObjectTypes`).
    fn process_object(
        &mut self,
        geo_json: &Value,
        object: &Value,
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) -> Result<(), String> {
        match object.get("type").and_then(|v| v.as_str()) {
            Some("Feature") => self.process_feature(object, crs_function, options),
            Some("FeatureCollection") => {
                self.process_feature_collection(object, crs_function, options)
            }
            Some("Topology") => self.process_topology(object, crs_function, options),
            _ => self.process_geometry(geo_json, object, crs_function, options),
        }
    }

    /// Mirror of `processFeature`.
    fn process_feature(
        &mut self,
        feature: &Value,
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) -> Result<(), String> {
        match feature.get("geometry") {
            // Null geometry is allowed, so just create an empty entity
            // instance for it.
            Some(Value::Null) => {
                let entity = self.create_object(feature, options);
                self.entity_collection.add(entity);
                Ok(())
            }
            None => Err("feature.geometry is required.".to_string()),
            Some(geometry) => {
                self.process_geometry(feature, geometry, crs_function, options)
            }
        }
    }

    /// Mirror of `processFeatureCollection`.
    fn process_feature_collection(
        &mut self,
        feature_collection: &Value,
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) -> Result<(), String> {
        let features = feature_collection
            .get("features")
            .and_then(|v| v.as_array());
        if let Some(features) = features {
            for feature in features {
                self.process_feature(feature, crs_function, options)?;
            }
        }
        Ok(())
    }

    /// Mirror of `processGeometryCollection`.
    fn process_geometry_collection(
        &mut self,
        geo_json: &Value,
        geometry_collection: &Value,
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) -> Result<(), String> {
        let geometries = geometry_collection
            .get("geometries")
            .and_then(|v| v.as_array());
        if let Some(geometries) = geometries {
            for geometry in geometries {
                self.process_geometry(geo_json, geometry, crs_function, options)?;
            }
        }
        Ok(())
    }

    /// Dispatches a geometry object to its handler
    /// (mirror of `geometryTypes`).
    fn process_geometry(
        &mut self,
        geo_json: &Value,
        geometry: &Value,
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) -> Result<(), String> {
        let geometry_type = geometry
            .get("type")
            .map(json_to_display_string)
            .unwrap_or_else(|| "undefined".to_string());
        match geometry_type.as_str() {
            "Point" => {
                self.create_point(geo_json, &coordinate_array(geometry), crs_function, options);
                Ok(())
            }
            "MultiPoint" => {
                for coordinates in nested_coordinate_arrays(geometry) {
                    self.create_point(geo_json, &coordinates, crs_function, options);
                }
                Ok(())
            }
            "LineString" => {
                self.create_line_string(
                    geo_json,
                    &coordinate_array(geometry),
                    crs_function,
                    options,
                );
                Ok(())
            }
            "MultiLineString" => {
                for line in nested_coordinate_arrays(geometry) {
                    self.create_line_string(geo_json, &line, crs_function, options);
                }
                Ok(())
            }
            "Polygon" => {
                self.create_polygon(
                    geo_json,
                    &polygon_ring_arrays(geometry),
                    crs_function,
                    options,
                );
                Ok(())
            }
            "MultiPolygon" => {
                for polygon in multipolygon_ring_arrays(geometry) {
                    self.create_polygon(geo_json, &polygon, crs_function, options);
                }
                Ok(())
            }
            "GeometryCollection" => {
                self.process_geometry_collection(geo_json, geometry, crs_function, options)
            }
            "Topology" => self.process_topology(geometry, crs_function, options),
            _ => Err(format!("Unknown geometry type: {}", geometry_type)),
        }
    }

    /// Mirror of `createPoint`.
    fn create_point(
        &mut self,
        geo_json: &Value,
        coordinates: &[Value],
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) {
        let mut symbol = options.marker_symbol.clone();
        let mut color = options.marker_color;
        let mut size = options.marker_size;

        if let Some(properties) = geo_json.get("properties").and_then(|v| v.as_object()) {
            if let Some(css_color) = properties.get("marker-color").and_then(|v| v.as_str()) {
                if let Some(parsed) = Color::from_css_color_string(css_color) {
                    color = parsed;
                }
            }

            if let Some(marker_size) = properties.get("marker-size").and_then(|v| v.as_str()) {
                if let Some(value) = marker_size_from_simplestyle(marker_size) {
                    size = value;
                }
            }
            if let Some(marker_symbol) = properties.get("marker-symbol") {
                if !marker_symbol.is_null() {
                    if let Some(value) = marker_symbol.as_str() {
                        symbol = Some(value.to_string());
                    }
                }
            }
        }

        // DEVIATION: the JS PinBuilder renders a canvas image (possibly
        // asynchronously); here a deterministic descriptor string is used.
        // An unknown Maki id fails to "load" and falls back to the plain
        // color pin, mirroring the promise `.catch` fallback.
        let image = match &symbol {
            Some(symbol) if symbol.chars().count() == 1 => {
                pin_from_text(&symbol.to_uppercase(), color, size)
            }
            Some(symbol) => pin_from_maki_icon_id(symbol, color, size)
                .unwrap_or_else(|| pin_from_color(color, size)),
            None => pin_from_color(color, size),
        };

        let mut billboard = BillboardGraphics::new();
        billboard.vertical_origin = VerticalOrigin::Bottom as i32;

        // Clamp to ground if there isn't a height specified.
        if coordinates.len() == 2 && options.clamp_to_ground {
            billboard.height_reference = HeightReference::ClampToGround as i32;
        }
        billboard.image = Some(image);

        let mut entity = self.create_object(geo_json, options);
        entity.billboard = Some(billboard);
        // A Point's `coordinates` is the position itself: a flat array of
        // numeric components (unlike LineString & friends whose entries
        // are positions).
        let position: Vec<f64> = coordinates.iter().filter_map(|c| c.as_f64()).collect();
        entity.position = Some(crs_function(&position));

        self.entity_collection.add(entity);
    }

    /// Mirror of `createLineString`.
    fn create_line_string(
        &mut self,
        geo_json: &Value,
        coordinates: &[Value],
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) {
        let mut material_color = options.stroke;
        let mut width = options.stroke_width;

        if let Some(properties) = geo_json.get("properties").and_then(|v| v.as_object()) {
            if let Some(width_value) = properties.get("stroke-width").and_then(|v| v.as_f64()) {
                width = width_value;
            }

            let mut color: Option<Color> = None;
            if let Some(stroke) = properties.get("stroke").and_then(|v| v.as_str()) {
                color = Color::from_css_color_string(stroke);
            }
            if let Some(opacity) = properties.get("stroke-opacity").and_then(|v| v.as_f64()) {
                if opacity != 1.0 {
                    let mut resolved = color.unwrap_or(material_color);
                    resolved.alpha = opacity;
                    color = Some(resolved);
                }
            }
            if let Some(resolved) = color {
                material_color = resolved;
            }
        }

        let mut entity = self.create_object(geo_json, options);
        let mut polyline = PolylineGraphics::new();
        polyline.clamp_to_ground = options.clamp_to_ground;
        polyline.material_color = material_color;
        polyline.width = width;
        polyline.positions = coordinates_array_to_cartesian_array(coordinates, crs_function);
        polyline.arc_type = ArcType::Rhumb;
        entity.polyline = Some(polyline);

        self.entity_collection.add(entity);
    }

    /// Mirror of `createPolygon`. `rings` holds the outer ring followed by
    /// the hole rings.
    fn create_polygon(
        &mut self,
        geo_json: &Value,
        rings: &[Vec<Value>],
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) {
        if rings.is_empty() || rings[0].is_empty() {
            return;
        }

        let mut outline_color = options.stroke;
        let mut material_color = options.fill;
        let mut width = options.stroke_width;

        if let Some(properties) = geo_json.get("properties").and_then(|v| v.as_object()) {
            if let Some(width_value) = properties.get("stroke-width").and_then(|v| v.as_f64()) {
                width = width_value;
            }

            let mut color: Option<Color> = None;
            if let Some(stroke) = properties.get("stroke").and_then(|v| v.as_str()) {
                color = Color::from_css_color_string(stroke);
            }
            if let Some(opacity) = properties.get("stroke-opacity").and_then(|v| v.as_f64()) {
                if opacity != 1.0 {
                    let mut resolved = color.unwrap_or(outline_color);
                    resolved.alpha = opacity;
                    color = Some(resolved);
                }
            }
            if let Some(resolved) = color {
                outline_color = resolved;
            }

            let material_alpha = material_color.alpha;
            let mut fill_color: Option<Color> = None;
            if let Some(fill) = properties.get("fill").and_then(|v| v.as_str()) {
                if let Some(parsed) = Color::from_css_color_string(fill) {
                    let mut parsed = parsed;
                    parsed.alpha = material_alpha;
                    fill_color = Some(parsed);
                }
            }
            if let Some(opacity) = properties.get("fill-opacity").and_then(|v| v.as_f64()) {
                if opacity != material_alpha {
                    let mut resolved = fill_color.unwrap_or(material_color);
                    resolved.alpha = opacity;
                    fill_color = Some(resolved);
                }
            }
            if let Some(resolved) = fill_color {
                material_color = resolved;
            }
        }

        let mut polygon = PolygonGraphics::new();
        polygon.outline = true;
        polygon.outline_color = outline_color;
        polygon.outline_width = width;
        polygon.material_color = material_color;
        polygon.arc_type = ArcType::Rhumb;

        for ring in rings.iter().skip(1) {
            polygon
                .holes
                .push(coordinates_array_to_cartesian_array(ring, crs_function));
        }

        let positions = &rings[0];
        polygon.hierarchy = coordinates_array_to_cartesian_array(positions, crs_function);
        let first_position_len = positions
            .first()
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if first_position_len > 2 {
            polygon.per_position_height = Some(true);
        } else if !options.clamp_to_ground {
            polygon.height = Some(0.0);
        }

        let mut entity = self.create_object(geo_json, options);
        entity.polygon = Some(polygon);

        self.entity_collection.add(entity);
    }

    /// Mirror of `processTopology` (TopoJSON support). Each object of the
    /// topology is converted to a GeoJSON feature (the analogue of
    /// `topojson.feature`) and processed through the regular handlers.
    fn process_topology(
        &mut self,
        topology: &Value,
        crs_function: &CrsFunction,
        options: &ResolvedOptions,
    ) -> Result<(), String> {
        if let Some(objects) = topology.get("objects").and_then(|v| v.as_object()) {
            for object in objects.values() {
                let feature = topojson_feature(topology, object)?;
                self.process_object(&feature, &feature, crs_function, options)?;
            }
        }
        Ok(())
    }

    /// Mirror of `createObject`: assigns the id (deduplicating repeated
    /// feature ids with `_2`, `_3` suffixes, or a new GUID), copies the
    /// properties and derives the name and description.
    fn create_object(&mut self, geo_json: &Value, options: &ResolvedOptions) -> Entity {
        // GeoJSON specifies only the Feature object has a usable id
        // property. But since "multi" geometries create multiple entities,
        // we can't use it for them either.
        let mut feature_id: Option<String> = None;
        if geo_json.get("type").and_then(|v| v.as_str()) == Some("Feature") {
            match geo_json.get("id") {
                Some(Value::String(s)) => feature_id = Some(s.clone()),
                Some(Value::Number(n)) => feature_id = Some(n.to_string()),
                _ => {}
            }
        }

        let id = match feature_id {
            Some(id) => {
                let mut final_id = id.clone();
                let mut i = 2;
                while self.entity_collection.contains_entity(&final_id) {
                    final_id = format!("{}_{}", id, i);
                    i += 1;
                }
                final_id
            }
            None => uuid::Uuid::new_v4().to_string(),
        };

        let mut entity = Entity::new(&id);
        let properties = geo_json.get("properties");
        if let Some(object) = properties.and_then(|v| v.as_object()) {
            let mut bag = PropertyBag::new();
            for (key, value) in object {
                bag.set(key, json_to_property_result(value));
            }
            entity.properties = bag;

            let mut name_property: Option<String> = None;

            // Check for the simplestyle specified name first.
            let name = object.get("title");
            if let Some(name) = name {
                if !name.is_null() {
                    entity.name = Some(json_to_display_string(name));
                    name_property = Some("title".to_string());
                }
            }

            if entity.name.is_none() {
                // Else, find the name by selecting an appropriate property.
                // The name will be obtained based on this order:
                // 1) The first case-insensitive property with the name 'title',
                // 2) The first case-insensitive property with the name 'name',
                // 3) The first property containing the word 'title'.
                // 4) The first property containing the word 'name',
                let mut name_property_precedence = i32::MAX;
                for (key, value) in object {
                    if is_truthy(value) {
                        let lower_key = key.to_lowercase();

                        if name_property_precedence > 1 && lower_key == "title" {
                            name_property = Some(key.clone());
                            break;
                        } else if name_property_precedence > 2 && lower_key == "name" {
                            name_property_precedence = 2;
                            name_property = Some(key.clone());
                        } else if name_property_precedence > 3
                            && lower_key.contains("title")
                        {
                            name_property_precedence = 3;
                            name_property = Some(key.clone());
                        } else if name_property_precedence > 4 && lower_key.contains("name")
                        {
                            name_property_precedence = 4;
                            name_property = Some(key.clone());
                        }
                    }
                }
                if let Some(name_property) = &name_property {
                    entity.name = object
                        .get(name_property)
                        .map(json_to_display_string);
                }
            }

            let description = object.get("description");
            match description {
                // `description !== null` — null means no description at all.
                Some(Value::Null) => {}
                None => {
                    entity.description = Some((options.describe)(
                        properties.unwrap(),
                        name_property.as_deref(),
                    ));
                }
                Some(description) => {
                    entity.description = Some(json_to_display_string(description));
                }
            }
        }
        entity
    }
}

impl Default for GeoJsonDataSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for GeoJsonDataSource {
    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
    fn entities(&self) -> &EntityCollection {
        &self.entity_collection
    }
    fn is_loading(&self) -> bool {
        self.is_loading
    }
    fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }
    fn changed_event(&self) -> &Event {
        &self.changed_event
    }
    fn error_event(&self) -> &Event {
        &self.error_event
    }
    fn loading_event(&self) -> &Event {
        &self.loading_event
    }
    fn show(&self) -> bool {
        self.entity_collection.show
    }
    fn set_show(&mut self, show: bool) {
        self.entity_collection.show = show;
    }
    fn destroy(&mut self) {
        self.entity_collection.destroy();
        self.is_destroyed = true;
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn is_supported_object_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "Feature"
            | "FeatureCollection"
            | "GeometryCollection"
            | "LineString"
            | "MultiLineString"
            | "MultiPoint"
            | "MultiPolygon"
            | "Point"
            | "Polygon"
            | "Topology"
    )
}

/// Mirror of the crs resolution block of the module-level `load` function.
/// Returns `Ok(None)` when `crs` is `null` (a valid value meaning the load
/// becomes a no-op).
fn resolve_crs(crs: Option<&Value>) -> Result<Option<CrsFunction>, String> {
    // `crs !== null ? defaultCrsFunction : null`
    let mut crs_function: Option<CrsFunction> = match crs {
        Some(Value::Null) => None,
        _ => Some(default_crs_function()),
    };

    if let Some(crs) = crs {
        if crs.is_null() {
            return Ok(None);
        }
        let properties = crs.get("properties");
        // `!defined(crs.properties)` — both missing and null.
        if properties.map_or(true, |p| p.is_null()) {
            return Err("crs.properties is undefined.".to_string());
        }
        let properties = properties.unwrap();

        match crs.get("type").and_then(|v| v.as_str()) {
            Some("name") => {
                let name = properties
                    .get("name")
                    .map(json_to_display_string)
                    .unwrap_or_else(|| "undefined".to_string());
                if CRS_NAMES.contains(&name.as_str()) {
                    crs_function = Some(default_crs_function());
                } else {
                    return Err(format!("Unknown crs name: {}", name));
                }
            }
            Some("link") => {
                let href = properties
                    .get("href")
                    .map(json_to_display_string)
                    .unwrap_or_default();
                let link_type = properties
                    .get("type")
                    .map(json_to_display_string)
                    .unwrap_or_default();

                let handler = CRS_LINK_HREFS
                    .lock()
                    .unwrap()
                    .get(&href)
                    .cloned()
                    .or_else(|| CRS_LINK_TYPES.lock().unwrap().get(&link_type).cloned());

                let handler = match handler {
                    Some(handler) => handler,
                    None => {
                        return Err(format!(
                            "Unable to resolve crs link: {}",
                            serde_json::to_string(properties).unwrap_or_default()
                        ))
                    }
                };

                crs_function = Some(handler(properties)?);
            }
            Some("EPSG") => {
                let code = properties
                    .get("code")
                    .map(json_to_display_string)
                    .unwrap_or_else(|| "undefined".to_string());
                let name = format!("EPSG:{}", code);
                if CRS_NAMES.contains(&name.as_str()) {
                    crs_function = Some(default_crs_function());
                } else {
                    return Err(format!("Unknown crs EPSG code: {}", code));
                }
            }
            Some(other) => {
                return Err(format!("Unknown crs type: {}", other));
            }
            None => {
                return Err("Unknown crs type: undefined".to_string());
            }
        }
    }

    Ok(crs_function)
}

/// Mirror of `coordinatesArrayToCartesianArray`.
fn coordinates_array_to_cartesian_array(
    coordinates: &[Value],
    crs_function: &CrsFunction,
) -> Vec<Cartesian3> {
    let values = coordinate_values(coordinates);
    let mut positions = Vec::with_capacity(coordinates.len());
    let mut index = 0;
    for coordinate in coordinates {
        let arity = coordinate.as_array().map(|a| a.len()).unwrap_or(0);
        positions.push(crs_function(&values[index..index + arity]));
        index += arity;
    }
    positions
}

/// Flattens an array of coordinate arrays into one f64 buffer.
fn coordinate_values(coordinates: &[Value]) -> Vec<f64> {
    let mut values = Vec::new();
    for coordinate in coordinates {
        if let Some(components) = coordinate.as_array() {
            for component in components {
                values.push(component.as_f64().unwrap_or(f64::NAN));
            }
        }
    }
    values
}

/// The `coordinates` member of a geometry as a slice of position values.
fn coordinate_array(geometry: &Value) -> Vec<Value> {
    geometry
        .get("coordinates")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// The nested coordinate arrays of MultiPoint/MultiLineString geometries.
fn nested_coordinate_arrays(geometry: &Value) -> Vec<Vec<Value>> {
    geometry
        .get("coordinates")
        .and_then(|v| v.as_array())
        .map(|outer| {
            outer
                .iter()
                .map(|inner| inner.as_array().cloned().unwrap_or_default())
                .collect()
        })
        .unwrap_or_default()
}

/// The rings of a Polygon geometry (outer ring + holes).
fn polygon_ring_arrays(geometry: &Value) -> Vec<Vec<Value>> {
    nested_coordinate_arrays(geometry)
}

/// The per-polygon ring arrays of a MultiPolygon geometry.
fn multipolygon_ring_arrays(geometry: &Value) -> Vec<Vec<Vec<Value>>> {
    geometry
        .get("coordinates")
        .and_then(|v| v.as_array())
        .map(|polygons| {
            polygons
                .iter()
                .map(|polygon| {
                    polygon
                        .as_array()
                        .map(|rings| {
                            rings
                                .iter()
                                .map(|ring| ring.as_array().cloned().unwrap_or_default())
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Converts a JSON value to the string that JS template interpolation
/// (`${value}`) would produce.
fn json_to_display_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

/// Converts a JSON property value into a [`PropertyResult`].
///
/// DEVIATION: JSON objects/arrays have no [`PropertyResult`] counterpart in
/// this port and are stored as [`PropertyResult::None`].
fn json_to_property_result(value: &Value) -> PropertyResult {
    match value {
        Value::Bool(b) => PropertyResult::Boolean(*b),
        Value::Number(n) => PropertyResult::Number(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => PropertyResult::String(s.clone()),
        _ => PropertyResult::None,
    }
}

/// JS truthiness for property values used in the name selection loop.
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map_or(true, |v| v != 0.0),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

// ============================================================================
// TopoJSON conversion (the analogue of `topojson.feature`).
// ============================================================================

/// Converts a TopoJSON object into a GeoJSON `Feature` value, mirroring
/// `topojson.feature(topology, object)` for the geometry types supported by
/// the CesiumJS test suite (Point, LineString, MultiLineString, Polygon,
/// MultiPolygon).
fn topojson_feature(topology: &Value, object: &Value) -> Result<Value, String> {
    let arcs = decode_arcs(topology);
    let geometry = topojson_geometry(object, &arcs)?;
    let mut feature = serde_json::Map::new();
    feature.insert("type".to_string(), Value::String("Feature".to_string()));
    feature.insert(
        "properties".to_string(),
        object
            .get("properties")
            .cloned()
            .unwrap_or(Value::Null),
    );
    feature.insert("geometry".to_string(), geometry);
    Ok(Value::Object(feature))
}

/// Decodes the delta-encoded arcs of a topology into absolute lon/lat
/// positions with the topology transform applied.
fn decode_arcs(topology: &Value) -> Vec<Vec<[f64; 2]>> {
    let (scale_x, scale_y, translate_x, translate_y) = match topology
        .get("transform")
    {
        Some(transform) => {
            let scale = transform.get("scale").and_then(|v| v.as_array());
            let translate = transform.get("translate").and_then(|v| v.as_array());
            (
                scale.and_then(|s| s.first()).and_then(|v| v.as_f64()).unwrap_or(1.0),
                scale.and_then(|s| s.get(1)).and_then(|v| v.as_f64()).unwrap_or(1.0),
                translate
                    .and_then(|s| s.first())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                translate
                    .and_then(|s| s.get(1))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
            )
        }
        None => (1.0, 1.0, 0.0, 0.0),
    };

    let mut decoded = Vec::new();
    if let Some(arcs) = topology.get("arcs").and_then(|v| v.as_array()) {
        for arc in arcs {
            let mut x = 0.0_f64;
            let mut y = 0.0_f64;
            let mut positions = Vec::new();
            if let Some(points) = arc.as_array() {
                for point in points {
                    if let Some(components) = point.as_array() {
                        x += components.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                        y += components.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        positions.push([
                            x * scale_x + translate_x,
                            y * scale_y + translate_y,
                        ]);
                    }
                }
            }
            decoded.push(positions);
        }
    }
    decoded
}

/// Concatenates the arcs referenced by `indices` into one position list,
/// mirroring the arc stitching of topojson-client (negative indices are
/// reversed via `~i`; the duplicated join point is dropped).
fn stitch_arcs(indices: &[Value], arcs: &[Vec<[f64; 2]>]) -> Vec<[f64; 2]> {
    let mut positions: Vec<[f64; 2]> = Vec::new();
    for index in indices {
        if let Some(index) = index.as_i64() {
            let (arc_index, reversed) = if index < 0 {
                ((!index) as usize, true)
            } else {
                (index as usize, false)
            };
            if let Some(arc) = arcs.get(arc_index) {
                let mut points = arc.clone();
                if reversed {
                    points.reverse();
                }
                if !positions.is_empty() {
                    // The first point duplicates the previous last point.
                    points.remove(0);
                }
                positions.extend(points);
            }
        }
    }
    positions
}

fn positions_to_value(positions: &[[f64; 2]]) -> Value {
    Value::Array(
        positions
            .iter()
            .map(|[lon, lat]| {
                Value::Array(vec![
                    serde_json::Number::from_f64(*lon)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                    serde_json::Number::from_f64(*lat)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                ])
            })
            .collect(),
    )
}

fn geometry_value(geometry_type: &str, coordinates: Value) -> Value {
    let mut geometry = serde_json::Map::new();
    geometry.insert("type".to_string(), Value::String(geometry_type.to_string()));
    geometry.insert("coordinates".to_string(), coordinates);
    Value::Object(geometry)
}

/// Converts a TopoJSON geometry object into a GeoJSON geometry value.
fn topojson_geometry(object: &Value, arcs: &[Vec<[f64; 2]>]) -> Result<Value, String> {
    let object_type = object
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("undefined");
    match object_type {
        "Point" => {
            let coordinates = object
                .get("coordinates")
                .cloned()
                .unwrap_or(Value::Null);
            Ok(geometry_value("Point", coordinates))
        }
        "LineString" => {
            let indices = object.get("arcs").and_then(|v| v.as_array());
            let positions =
                stitch_arcs(indices.map(|a| a.as_slice()).unwrap_or(&[]), arcs);
            Ok(geometry_value("LineString", positions_to_value(&positions)))
        }
        "MultiLineString" => {
            let lines = object.get("arcs").and_then(|v| v.as_array());
            let coordinates: Vec<Value> = lines
                .map(|lines| {
                    lines
                        .iter()
                        .filter_map(|line| line.as_array())
                        .map(|indices| positions_to_value(&stitch_arcs(indices, arcs)))
                        .collect()
                })
                .unwrap_or_default();
            Ok(geometry_value("MultiLineString", Value::Array(coordinates)))
        }
        "Polygon" => {
            let rings = object.get("arcs").and_then(|v| v.as_array());
            let coordinates: Vec<Value> = rings
                .map(|rings| {
                    rings
                        .iter()
                        .filter_map(|ring| ring.as_array())
                        .map(|indices| positions_to_value(&stitch_arcs(indices, arcs)))
                        .collect()
                })
                .unwrap_or_default();
            Ok(geometry_value("Polygon", Value::Array(coordinates)))
        }
        "MultiPolygon" => {
            let polygons = object.get("arcs").and_then(|v| v.as_array());
            let coordinates: Vec<Value> = polygons
                .map(|polygons| {
                    polygons
                        .iter()
                        .filter_map(|polygon| polygon.as_array())
                        .map(|rings| {
                            Value::Array(
                                rings
                                    .iter()
                                    .filter_map(|ring| ring.as_array())
                                    .map(|indices| {
                                        positions_to_value(&stitch_arcs(indices, arcs))
                                    })
                                    .collect(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(geometry_value("MultiPolygon", Value::Array(coordinates)))
        }
        _ => Err(format!("Unsupported GeoJSON object type: {}", object_type)),
    }
}
