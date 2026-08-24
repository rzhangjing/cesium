//! Ported from `packages/engine/Source/DataSources/exportKml.js`.
//!
//! Exports an [`EntityCollection`] as a KML document. Only Point, Billboard,
//! Model, Polygon and Polyline geometries are exported (the simplified entity
//! model carries no rectangle/path fields).
//!
//! DEVIATION: CesiumJS builds a live DOM document and serializes it with
//! `XMLSerializer`; this port uses a private [`KmlElement`] tree with an
//! XMLSerializer-compatible serializer (no XML declaration, no indentation,
//! empty elements serialized as `<name/>`, text escaped).
//!
//! DEVIATION: the crate's simplified value model stores constant values
//! instead of time-dynamic properties, so `SampledPositionProperty`,
//! `CallbackProperty` and `CompositePositionProperty` sampling cannot occur.
//! [`create_tracks`] is retained for API parity but is never reached by
//! [`create_point`] / [`create_model`].
//!
//! DEVIATION: CesiumJS graphics properties are lazily created, so an
//! "unset" property is `undefined` while the documented default only applies
//! conceptually. The value model always stores the default, so the default
//! value doubles as a sentinel meaning "unset" (billboard scale 1.0, point
//! pixel size 5.0, label fill color WHITE / scale 1.0, polyline width 1.0,
//! material color WHITE meaning "no material", model scale 1.0). A user who
//! explicitly assigns the very same default value gets the "unset" output.
//!
//! DEVIATION: billboard `heading` is gated on `alignedAxis == UNIT_Z` only;
//! upstream additionally requires `rotation` to be defined, so an unset
//! rotation with a UNIT_Z aligned axis emits `heading 360` here.
//!
//! DEVIATION: `PolyStyle/fill` is never emitted: upstream emits it only when
//! `fill` was explicitly assigned `true`, which the value model cannot
//! distinguish from its `true` default (an explicit `false` emits nothing
//! upstream either, so nothing else is lost).
//!
//! DEVIATION: the entity model has no `rectangle`, `path`, `zIndex` and no
//! polygon/model `heightReference`, so `GroundOverlay`/rectangle boundaries
//! (`getRectangleBoundaries`, `createGroundOverlay`) and `gx:drawOrder` are
//! skipped and polygon/model altitude modes are always `absolute`.
//!
//! DEVIATION: `options.kmz` is rejected with an error because the workspace
//! has no zip dependency (`createKmz`/`addExternalFilesToZip` have no
//! counterpart). `options.ellipsoid` is ignored (WGS84 only). External file
//! blobs are `Vec<u8>` instead of browser `Blob`s, and images are strings
//! (plain URLs or data URIs), so canvas images cannot occur.

use std::collections::HashMap;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::color::Color;
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::math::CesiumMath;
use cesium_core::time_interval::TimeInterval;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::horizontal_origin::HorizontalOrigin;
use cesium_scene::vertical_origin::VerticalOrigin;

use crate::billboard_graphics::BillboardGraphics;
use crate::entity::Entity;
use crate::entity_collection::EntityCollection;
use crate::model_graphics::ModelGraphics;
use crate::point_graphics::PointGraphics;
use crate::polygon_graphics::PolygonGraphics;
use crate::polyline_graphics::PolylineGraphics;

/// Mirror of the upstream `BILLBOARD_SIZE` constant.
const BILLBOARD_SIZE: f64 = 32.0;

/// Mirror of the upstream `kmlNamespace` constant.
const KML_NAMESPACE: &str = "http://www.opengis.net/kml/2.2";

/// Mirror of the upstream `gxNamespace` constant.
const GX_NAMESPACE: &str = "http://www.google.com/kml/ext/2.2";

//
// Minimal XML element tree standing in for the browser DOM document.
//

/// DEVIATION: replaces the browser DOM element used by the upstream
/// `kmlDoc.createElement[NS]` calls; serialized below in an
/// XMLSerializer-compatible way.
#[derive(Debug, Clone)]
struct KmlElement {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<KmlElement>,
    text: Option<String>,
}

impl KmlElement {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            attributes: Vec::new(),
            children: Vec::new(),
            text: None,
        }
    }

    /// Mirror of `setAttribute` (replaces an existing attribute).
    fn set_attribute(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        for attribute in &mut self.attributes {
            if attribute.0 == name {
                attribute.1 = value;
                return;
            }
        }
        self.attributes.push((name.to_string(), value));
    }

    /// Mirror of `innerHTML` used as the [`StyleCache`] key: the serialized
    /// children without the element's own tag.
    fn inner_html(&self) -> String {
        let mut result = String::new();
        for child in &self.children {
            child.serialize(&mut result);
        }
        result
    }

    /// XMLSerializer-compatible serialization: no XML declaration, no
    /// indentation, empty elements as `<name/>`, attribute/text escaping.
    fn serialize(&self, out: &mut String) {
        out.push('<');
        out.push_str(&self.name);
        for (name, value) in &self.attributes {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            escape_text(value, true, out);
            out.push('"');
        }

        let has_text = matches!(&self.text, Some(text) if !text.is_empty());
        if !has_text && self.children.is_empty() {
            out.push_str("/>");
            return;
        }

        out.push('>');
        if let Some(text) = &self.text {
            escape_text(text, false, out);
        }
        for child in &self.children {
            child.serialize(out);
        }
        out.push_str("</");
        out.push_str(&self.name);
        out.push('>');
    }
}

fn escape_text(value: &str, in_attribute: bool, out: &mut String) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if in_attribute => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Mirror of `createBasicElementWithText`.
///
/// DEVIATION (faithful bug mirror): the upstream CDATA branch compares
/// `elementValue === "string"` (the literal type name), which is never true,
/// so values always become escaped text nodes.
fn create_basic_element_with_text(
    element_name: &str,
    element_value: Option<&str>,
    gx_namespace: bool,
) -> KmlElement {
    let value = element_value.unwrap_or("");
    let name = if gx_namespace {
        format!("gx:{}", element_name)
    } else {
        element_name.to_string()
    };
    let mut element = KmlElement::new(&name);
    element.text = Some(value.to_string());
    element
}

//
// Handles files external to the KML (eg. textures and models)
//

/// Mirror of `ExternalFileHandler`. DEVIATION: blobs are `Vec<u8>` and all
/// fetches are synchronous, so there is no promise bookkeeping.
struct ExternalFileHandler<'a> {
    files: HashMap<String, Vec<u8>>,
    count: u32,
    model_callback: Option<ModelCallback<'a>>,
}

impl<'a> ExternalFileHandler<'a> {
    fn new(model_callback: Option<ModelCallback<'a>>) -> Self {
        Self {
            files: HashMap::new(),
            count: 0,
            model_callback,
        }
    }

    /// Mirror of `ExternalFileHandler.prototype.texture`. DEVIATION: images
    /// are strings, so only the URL/data-URI branch exists and the data URI
    /// payload is base64-decoded in place of a blob fetch.
    fn texture(&mut self, texture: &str) -> Result<String, String> {
        if !texture.starts_with("data:") {
            return Ok(texture.to_string());
        }

        // If its a data URI try and get the correct extension and then fetch the blob
        self.count += 1;
        let mut filename = format!("texture_{}", self.count);
        if let Some(image_type) = data_uri_image_type(texture) {
            filename.push('.');
            filename.push_str(&image_type);
        }

        let payload = texture.split(',').nth(1).unwrap_or("");
        let bytes = decode_base64(payload)
            .ok_or_else(|| format!("Failed to decode data URI for {}", filename))?;
        self.files.insert(filename.clone(), bytes);

        Ok(filename)
    }

    /// Mirror of `ExternalFileHandler.prototype.model`. DEVIATION: the
    /// callback is synchronous, so the returned files are merged directly.
    fn model(&mut self, model: &ModelGraphics, time: &JulianDate) -> Result<String, String> {
        let model_callback = match &self.model_callback {
            Some(model_callback) => model_callback,
            None => {
                // Mirror of the upstream RuntimeError.
                return Err(String::from(
                    "Encountered a model entity while exporting to KML, but no model callback was supplied.",
                ));
            }
        };

        let mut external_files: HashMap<String, Vec<u8>> = HashMap::new();
        let url = model_callback(model, time, &mut external_files);

        // Iterate through external files and add them to our list
        for (filename, bytes) in external_files {
            self.files.insert(filename, bytes);
        }

        Ok(url)
    }
}

/// Mirror of the upstream `imageTypeRegex` (`^data:image/([^,;]+)`).
fn data_uri_image_type(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("data:image/")?;
    let end = rest
        .find(|c| c == ',' || c == ';')
        .unwrap_or(rest.len());
    let image_type = &rest[..end];
    if image_type.is_empty() {
        None
    } else {
        Some(image_type.to_string())
    }
}

/// DEVIATION: the workspace has no base64 dependency, so a minimal decoder
/// stands in for the browser's data URI blob fetch.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for c in input.chars() {
        if c.is_whitespace() {
            continue;
        }
        if c == '=' {
            break;
        }
        let value = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

//
// Handles getting values from properties taking the desired time and default values into account
//

/// Mirror of `ValueGetter`. DEVIATION: the value model has no time-dynamic
/// properties, so all getters degenerate to identity/default unwrapping;
/// `_time` is kept for parity.
struct ValueGetter {
    _time: JulianDate,
}

impl ValueGetter {
    fn new(time: JulianDate) -> Self {
        Self { _time: time }
    }

    /// Mirror of `ValueGetter.prototype.get`.
    fn get_or<T>(&self, value: Option<T>, default_val: T) -> T {
        value.unwrap_or(default_val)
    }

    /// Mirror of `ValueGetter.prototype.getColor`.
    fn get_color(&self, color: Option<&Color>) -> Option<String> {
        color.map(|color| color_to_string(color))
    }

    /// Mirror of `ValueGetter.prototype.getMaterialType`. DEVIATION: only
    /// color materials exist in the value model.
    #[allow(dead_code)]
    fn get_material_type(&self, has_material: bool) -> Option<&'static str> {
        has_material.then_some("Color")
    }
}

//
// Caches styles so we don't generate a ton of duplicate styles
//

/// Mirror of `StyleCache`.
struct StyleCache {
    ids: HashMap<String, String>,
    styles: Vec<KmlElement>,
    count: u32,
}

impl StyleCache {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            styles: Vec::new(),
            count: 0,
        }
    }

    /// Mirror of `StyleCache.prototype.get`; the element's serialized
    /// children play the role of the DOM `innerHTML` key.
    fn get(&mut self, mut element: KmlElement) -> String {
        let key = element.inner_html();
        if let Some(style_id) = self.ids.get(&key) {
            return style_id.clone();
        }

        self.count += 1;
        let style_id = format!("style-{}", self.count);
        element.set_attribute("id", style_id.clone());

        // Store with #
        let style_url = format!("#{}", style_id);
        self.ids.insert(key, style_url.clone());
        self.styles.push(element);

        style_url
    }

    /// Mirror of `StyleCache.prototype.save` (inserts the cached styles
    /// before the parent's first child).
    fn save(&mut self, parent_element: &mut KmlElement) {
        let styles = std::mem::take(&mut self.styles);
        parent_element.children.splice(0..0, styles);
    }
}

//
// Manages the generation of IDs because an entity may have geometry and a Folder for children
//

/// Mirror of `IdManager`.
struct IdManager {
    ids: HashMap<String, u32>,
    guid_count: u32,
}

impl IdManager {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            guid_count: 0,
        }
    }

    /// Mirror of `IdManager.prototype.get`. DEVIATION: generated GUIDs use a
    /// local counter instead of `createGuid`.
    fn get(&mut self, id: Option<&str>) -> String {
        let id = match id {
            Some(id) => id.to_string(),
            None => {
                self.guid_count += 1;
                format!("export-kml-guid-{}", self.guid_count)
            }
        };

        match self.ids.get_mut(&id) {
            None => {
                self.ids.insert(id.clone(), 0);
                id
            }
            Some(count) => {
                *count += 1;
                format!("{}-{}", id, count)
            }
        }
    }
}

/// Mirror of `exportKmlModelCallback`. DEVIATION: external files map to
/// `Vec<u8>` instead of blobs or promises.
pub type ModelCallback<'a> = Box<
    dyn Fn(&ModelGraphics, &JulianDate, &mut HashMap<String, Vec<u8>>) -> String + 'a,
>;

/// Mirror of the `exportKml` options object. DEVIATION: `ellipsoid` is not
/// available (WGS84 only).
pub struct ExportKmlOptions<'a> {
    /// A callback that will be called with a [`ModelGraphics`] instance and
    /// should return the URI to use in the KML. Required if a model exists
    /// in the entity collection.
    pub model_callback: Option<ModelCallback<'a>>,
    /// The time value to use to get properties that are not time varying in
    /// KML. Defaults to `entities.computeAvailability().start`.
    pub time: Option<JulianDate>,
    /// The interval that will be sampled if an entity doesn't have an
    /// availability. Defaults to `entities.computeAvailability()`.
    pub default_availability: Option<TimeInterval>,
    /// The number of seconds to sample properties that are varying in KML.
    pub sample_duration: f64,
    /// If true the export errors out: KMZ packaging is not supported by this
    /// port (DEVIATION).
    pub kmz: bool,
}

impl Default for ExportKmlOptions<'_> {
    fn default() -> Self {
        Self {
            model_callback: None,
            time: None,
            default_availability: None,
            sample_duration: 60.0,
            kmz: false,
        }
    }
}

/// Mirror of `exportKmlResultKml`. DEVIATION: external files are byte
/// buffers instead of blobs.
#[derive(Debug)]
pub struct ExportKmlResult {
    /// The generated KML.
    pub kml: String,
    /// An object dictionary of external files.
    pub external_files: HashMap<String, Vec<u8>>,
}

/// Mirror of the recursion state assembled by `exportKml._createState`.
struct ExportKmlState<'a> {
    id_manager: IdManager,
    style_cache: StyleCache,
    external_file_handler: ExternalFileHandler<'a>,
    time: JulianDate,
    value_getter: ValueGetter,
    sample_duration: f64,
    default_availability: TimeInterval,
}

/// Mirror of `exportKml._createState`.
fn create_state<'a>(entities: &EntityCollection, options: &mut ExportKmlOptions<'a>) -> ExportKmlState<'a> {
    let style_cache = StyleCache::new();

    // Use the start time as the default because just in case they define
    //  properties with an interval even if they don't change.
    let entity_availability = entities.compute_availability();
    let time = options
        .time
        .clone()
        .unwrap_or_else(|| entity_availability.start.clone());

    // Figure out how we will sample dynamic position properties
    let mut default_availability = options
        .default_availability
        .clone()
        .unwrap_or(entity_availability);
    let sample_duration = options.sample_duration;

    // Make sure we don't have infinite availability if we need to sample
    if JulianDate::equals(&default_availability.start, Iso8601::minimum_value()) {
        if JulianDate::equals(&default_availability.stop, Iso8601::maximum_value()) {
            // Infinite, so just use the default
            default_availability = TimeInterval::new(None, None, None, None);
        } else {
            // No start time, so just sample 10 times before the stop
            default_availability.start = JulianDate::add_seconds_new(
                &default_availability.stop,
                -10.0 * sample_duration,
            );
        }
    } else if JulianDate::equals(&default_availability.stop, Iso8601::maximum_value()) {
        // No stop time, so just sample 10 times after the start
        default_availability.stop = JulianDate::add_seconds_new(
            &default_availability.start,
            10.0 * sample_duration,
        );
    }

    let external_file_handler = ExternalFileHandler::new(options.model_callback.take());

    ExportKmlState {
        id_manager: IdManager::new(),
        style_cache,
        external_file_handler,
        value_getter: ValueGetter::new(time.clone()),
        time,
        sample_duration,
        // Wrap it in a collection because that is what entity.availability is
        default_availability,
    }
}

/// Exports an [`EntityCollection`] as a KML document.
///
/// Mirror of `exportKml`. DEVIATION: synchronous and returns
/// `Result<ExportKmlResult, String>`; `kmz` is unsupported (see module
/// docs).
pub fn export_kml(
    entities: &EntityCollection,
    mut options: ExportKmlOptions<'_>,
) -> Result<ExportKmlResult, String> {
    let kmz = options.kmz;

    // DEVIATION: `entities` is a required argument in Rust, so the upstream
    // DeveloperError guard has no counterpart.

    // Get the state that is passed around during the recursion
    // This is separated out for testing.
    let mut state = create_state(entities, &mut options);

    // Filter EntityCollection so we only have top level entities and build
    // the parent->children map used for the KML hierarchy.
    let values = entities.values();
    let mut root_entities: Vec<&Entity> = Vec::new();
    let mut children_map: HashMap<String, Vec<&Entity>> = HashMap::new();
    for entity in &values {
        match &entity.parent_id {
            Some(parent_id) if entities.contains_entity(parent_id) => {
                children_map
                    .entry(parent_id.clone())
                    .or_default()
                    .push(entity);
            }
            _ => root_entities.push(entity),
        }
    }

    // Add the <Document>
    let mut kml_root = KmlElement::new("kml");
    kml_root.set_attribute("xmlns", KML_NAMESPACE);
    kml_root.set_attribute("xmlns:gx", GX_NAMESPACE);
    let mut kml_document_element = KmlElement::new("Document");

    // Create the KML Hierarchy
    recurse_entities(&mut state, &mut kml_document_element, &root_entities, &children_map)?;

    // Write out the <Style> elements
    state.style_cache.save(&mut kml_document_element);

    kml_root.children.push(kml_document_element);

    let mut kml_string = String::new();
    kml_root.serialize(&mut kml_string);
    if kmz {
        // DEVIATION: mirror of `createKmz`/`addExternalFilesToZip` requires
        // zip support, which the workspace does not provide.
        return Err(String::from(
            "kmz export is not supported by this port; set kmz to false.",
        ));
    }

    Ok(ExportKmlResult {
        kml: kml_string,
        external_files: state.external_file_handler.files,
    })
}

/// Mirror of `recurseEntities`.
fn recurse_entities(
    state: &mut ExportKmlState,
    parent_node: &mut KmlElement,
    entities: &[&Entity],
    children_map: &HashMap<String, Vec<&Entity>>,
) -> Result<(), String> {
    for entity in entities {
        let mut overlays: Vec<KmlElement> = Vec::new();
        let mut geometries: Vec<KmlElement> = Vec::new();
        let mut styles: Vec<KmlElement> = Vec::new();

        create_point(state, entity, &mut geometries, &mut styles)?;
        if let Some(polyline) = &entity.polyline {
            create_line_string(polyline, &mut geometries, &mut styles);
        }
        // DEVIATION: the entity model has no rectangle field, so the upstream
        // `createPolygon(state, entity.rectangle, ...)` call is skipped.
        if let Some(polygon) = &entity.polygon {
            create_polygon(state, polygon, &mut geometries, &mut styles, &mut overlays);
        }
        if let Some(model) = &entity.model {
            create_model(state, entity, model, &mut geometries, &mut styles)?;
        }

        let mut time_span: Option<KmlElement> = None;
        if !entity.availability.is_empty() {
            let mut span = KmlElement::new("TimeSpan");

            // Aggregate the interval collection into start/stop as upstream's
            // TimeIntervalCollection exposes them.
            let mut start = &entity.availability[0].start;
            let mut stop = &entity.availability[0].stop;
            for interval in &entity.availability {
                if JulianDate::compare(&interval.start, start) < 0 {
                    start = &interval.start;
                }
                if JulianDate::compare(&interval.stop, stop) > 0 {
                    stop = &interval.stop;
                }
            }

            if !JulianDate::equals(start, Iso8601::minimum_value()) {
                span.children.push(create_basic_element_with_text(
                    "begin",
                    Some(&start.to_iso8601(None)),
                    false,
                ));
            }

            if !JulianDate::equals(stop, Iso8601::maximum_value()) {
                span.children.push(create_basic_element_with_text(
                    "end",
                    Some(&stop.to_iso8601(None)),
                    false,
                ));
            }

            time_span = Some(span);
        }

        // DEVIATION: overlays stay empty (no rectangle/GroundOverlay), but
        // the loop mirrors upstream for parity.
        for mut overlay in overlays {
            overlay.set_attribute("id", state.id_manager.get(Some(&entity.id)));
            overlay
                .children
                .push(create_basic_element_with_text("name", entity.name.as_deref(), false));
            overlay.children.push(create_basic_element_with_text(
                "visibility",
                Some(if entity.show { "1" } else { "0" }),
                false,
            ));
            overlay.children.push(create_basic_element_with_text(
                "description",
                entity.description.as_deref(),
                false,
            ));

            if let Some(time_span) = time_span.take() {
                overlay.children.push(time_span);
            }

            parent_node.children.push(overlay);
        }

        if !geometries.is_empty() {
            let mut placemark = KmlElement::new("Placemark");
            placemark.set_attribute("id", state.id_manager.get(Some(&entity.id)));

            let mut name = entity.name.clone();
            if let Some(label_graphics) = &entity.label {
                let mut label_style = KmlElement::new("LabelStyle");

                // KML only shows the name as a label, so just change the name if we need to show a label
                let text = label_graphics.text.as_deref();
                if let Some(text) = text {
                    if !text.is_empty() {
                        name = Some(text.to_string());
                    }
                }

                // DEVIATION (sentinel): the default WHITE fill color means
                // "unset" upstream (lazy property creation).
                if label_graphics.fill_color != Color::WHITE {
                    let color = color_to_string(&label_graphics.fill_color);
                    label_style
                        .children
                        .push(create_basic_element_with_text("color", Some(&color), false));
                    label_style.children.push(create_basic_element_with_text(
                        "colorMode",
                        Some("normal"),
                        false,
                    ));
                }

                // DEVIATION (sentinel): the default scale 1.0 means "unset".
                if label_graphics.scale != 1.0 {
                    label_style.children.push(create_basic_element_with_text(
                        "scale",
                        Some(&format!("{}", label_graphics.scale)),
                        false,
                    ));
                }

                styles.push(label_style);
            }

            placemark
                .children
                .push(create_basic_element_with_text("name", name.as_deref(), false));
            placemark.children.push(create_basic_element_with_text(
                "visibility",
                Some(if entity.show { "1" } else { "0" }),
                false,
            ));
            placemark.children.push(create_basic_element_with_text(
                "description",
                entity.description.as_deref(),
                false,
            ));

            if let Some(time_span) = time_span.take() {
                placemark.children.push(time_span);
            }

            if !styles.is_empty() {
                let mut style = KmlElement::new("Style");
                style.children = styles;
                placemark.children.push(create_basic_element_with_text(
                    "styleUrl",
                    Some(&state.style_cache.get(style)),
                    false,
                ));
            }

            if geometries.len() == 1 {
                placemark.children.push(geometries.pop().unwrap());
            } else {
                let mut multigeometry = KmlElement::new("MultiGeometry");
                multigeometry.children = geometries;
                placemark.children.push(multigeometry);
            }

            parent_node.children.push(placemark);
        }

        let empty: Vec<&Entity> = Vec::new();
        let entity_children = children_map.get(&entity.id).unwrap_or(&empty);
        if !entity_children.is_empty() {
            let mut folder_node = KmlElement::new("Folder");
            folder_node.set_attribute("id", state.id_manager.get(Some(&entity.id)));
            folder_node
                .children
                .push(create_basic_element_with_text("name", entity.name.as_deref(), false));
            folder_node.children.push(create_basic_element_with_text(
                "visibility",
                Some(if entity.show { "1" } else { "0" }),
                false,
            ));
            folder_node.children.push(create_basic_element_with_text(
                "description",
                entity.description.as_deref(),
                false,
            ));

            parent_node.children.push(folder_node);

            let folder_node = parent_node.children.last_mut().unwrap();
            recurse_entities(state, folder_node, entity_children, children_map)?;
        }
    }

    Ok(())
}

/// Mirror of `createPoint`.
fn create_point(
    state: &mut ExportKmlState,
    entity: &Entity,
    geometries: &mut Vec<KmlElement>,
    styles: &mut Vec<KmlElement>,
) -> Result<(), String> {
    let billboard_graphics = entity.billboard.as_ref();
    let point_graphics = entity.point.as_ref();
    // DEVIATION: the entity model has no path field.
    if billboard_graphics.is_none() && point_graphics.is_none() {
        return Ok(());
    }

    // If the point isn't constant then create gx:Track or gx:MultiTrack
    let entity_position = match &entity.position {
        // DEVIATION: positions are constant in the value model, so the track
        // branch is unreachable; a missing position is skipped instead of
        // throwing (upstream dereferences the position property).
        Some(position) => position,
        None => return Ok(()),
    };

    let coordinates = create_basic_element_with_text(
        "coordinates",
        Some(&get_coordinates(std::slice::from_ref(entity_position))),
        false,
    );

    let mut point_geometry = KmlElement::new("Point");

    // Set altitude mode
    let height_reference = billboard_graphics
        .map(|billboard| billboard.height_reference)
        .or_else(|| point_graphics.map(|point| point.height_reference))
        .unwrap_or(HeightReference::None as i32);
    let mut altitude_mode = KmlElement::new("altitudeMode");
    altitude_mode.text = Some(get_altitude_mode(height_reference));
    point_geometry.children.push(altitude_mode);

    point_geometry.children.push(coordinates);
    geometries.push(point_geometry);

    // Create style
    if let Some(billboard_graphics) = billboard_graphics {
        styles.push(create_icon_style_from_billboard(state, billboard_graphics)?);
    } else if let Some(point_graphics) = point_graphics {
        styles.push(create_icon_style_from_point(state, point_graphics));
    }

    Ok(())
}

/// Mirror of `createTracks`.
///
/// DEVIATION: the value model holds constant positions only, so
/// [`create_point`]/[`create_model`] never route here; the function is
/// retained for parity, sampling the constant position at the availability
/// interval boundaries (`CompositePositionProperty` intervals and path line
/// styles have no counterpart).
#[allow(dead_code)]
fn create_tracks(
    state: &mut ExportKmlState,
    entity: &Entity,
    height_reference: i32,
    is_model: bool,
    geometries: &mut Vec<KmlElement>,
    styles: &mut Vec<KmlElement>,
) -> Result<(), String> {
    let intervals: Vec<TimeInterval> = if entity.availability.is_empty() {
        vec![state.default_availability.clone()]
    } else {
        entity.availability.clone()
    };

    let position = match &entity.position {
        Some(position) => position,
        None => return Ok(()),
    };

    let mut tracks: Vec<KmlElement> = Vec::new();
    for interval in &intervals {
        let mut track_altitude_mode = KmlElement::new("altitudeMode");
        track_altitude_mode.text = Some(get_altitude_mode(height_reference));

        // The interval position is constant in the value model, so add a
        // track with the same position at start and stop.
        let const_coordinates = get_coordinates(std::slice::from_ref(position));
        let position_times = [
            interval.start.to_iso8601(None),
            interval.stop.to_iso8601(None),
        ];
        let position_values = [const_coordinates.clone(), const_coordinates];

        let mut track_geometry = KmlElement::new("gx:Track");
        track_geometry.children.push(track_altitude_mode);

        for k in 0..position_times.len() {
            let when = create_basic_element_with_text("when", Some(&position_times[k]), false);
            let coord = create_basic_element_with_text("coord", Some(&position_values[k]), true);

            track_geometry.children.push(when);
            track_geometry.children.push(coord);
        }

        if is_model {
            if let Some(model) = &entity.model {
                track_geometry
                    .children
                    .push(create_model_geometry(state, model)?);
            }
        }

        tracks.push(track_geometry);
    }

    // If one track, then use it otherwise combine into a multitrack
    if tracks.len() == 1 {
        geometries.push(tracks.pop().unwrap());
    } else if tracks.len() > 1 {
        let mut multi_track_geometry = KmlElement::new("gx:MultiTrack");
        multi_track_geometry.children = tracks;
        geometries.push(multi_track_geometry);
    }

    // Create style
    if !is_model {
        if let Some(billboard) = &entity.billboard {
            styles.push(create_icon_style_from_billboard(state, billboard)?);
        } else if let Some(point) = &entity.point {
            styles.push(create_icon_style_from_point(state, point));
        }
    }

    // DEVIATION: the entity model has no path field, so the upstream path
    // LineStyle block has no counterpart.

    Ok(())
}

/// Mirror of `createIconStyleFromPoint`.
fn create_icon_style_from_point(state: &ExportKmlState, point_graphics: &PointGraphics) -> KmlElement {
    let mut icon_style = KmlElement::new("IconStyle");

    // DEVIATION (sentinel): the default WHITE color means "unset" upstream.
    if point_graphics.color != Color::WHITE {
        let color = state.value_getter.get_color(Some(&point_graphics.color));
        if let Some(color) = color {
            icon_style
                .children
                .push(create_basic_element_with_text("color", Some(&color), false));
            icon_style.children.push(create_basic_element_with_text(
                "colorMode",
                Some("normal"),
                false,
            ));
        }
    }

    // DEVIATION (sentinel): the default pixel size 5.0 means "unset".
    if point_graphics.pixel_size != 5.0 {
        icon_style.children.push(create_basic_element_with_text(
            "scale",
            Some(&format!("{}", point_graphics.pixel_size / BILLBOARD_SIZE)),
            false,
        ));
    }

    icon_style
}

/// Mirror of `createIconStyleFromBillboard`.
fn create_icon_style_from_billboard(
    state: &mut ExportKmlState,
    billboard_graphics: &BillboardGraphics,
) -> Result<KmlElement, String> {
    let mut icon_style = KmlElement::new("IconStyle");

    if let Some(image) = &billboard_graphics.image {
        let image = state.external_file_handler.texture(image)?;

        let mut icon = KmlElement::new("Icon");
        icon.children
            .push(create_basic_element_with_text("href", Some(&image), false));

        if let Some((x, y, width, height)) = billboard_graphics.image_sub_region {
            icon.children.push(create_basic_element_with_text(
                "x",
                Some(&format!("{}", x)),
                true,
            ));
            icon.children.push(create_basic_element_with_text(
                "y",
                Some(&format!("{}", y)),
                true,
            ));
            icon.children.push(create_basic_element_with_text(
                "w",
                Some(&format!("{}", width)),
                true,
            ));
            icon.children.push(create_basic_element_with_text(
                "h",
                Some(&format!("{}", height)),
                true,
            ));
        }

        icon_style.children.push(icon);
    }

    if let Some(color) = state.value_getter.get_color(billboard_graphics.color.as_ref()) {
        icon_style
            .children
            .push(create_basic_element_with_text("color", Some(&color), false));
        icon_style.children.push(create_basic_element_with_text(
            "colorMode",
            Some("normal"),
            false,
        ));
    }

    // DEVIATION (sentinel): the default scale 1.0 means "unset".
    let mut scale: Option<f64> = None;
    if billboard_graphics.scale != 1.0 {
        scale = Some(billboard_graphics.scale);
        icon_style.children.push(create_basic_element_with_text(
            "scale",
            Some(&format!("{}", billboard_graphics.scale)),
            false,
        ));
    }

    if let Some((offset_x, offset_y)) = billboard_graphics.pixel_offset {
        let scale = scale.unwrap_or(1.0);

        let mut pixel_offset_x = offset_x / scale;
        let mut pixel_offset_y = offset_y / scale;

        let width = state
            .value_getter
            .get_or(billboard_graphics.width, BILLBOARD_SIZE);
        let height = state
            .value_getter
            .get_or(billboard_graphics.height, BILLBOARD_SIZE);

        // KML Hotspots are from the bottom left, but we work from the top left

        // Move to left
        let horizontal_origin = billboard_graphics.horizontal_origin;
        if horizontal_origin == HorizontalOrigin::Center as i32 {
            pixel_offset_x -= width * 0.5;
        } else if horizontal_origin == HorizontalOrigin::Right as i32 {
            pixel_offset_x -= width;
        }

        // Move to bottom
        let vertical_origin = billboard_graphics.vertical_origin;
        if vertical_origin == VerticalOrigin::Top as i32 {
            pixel_offset_y += height;
        } else if vertical_origin == VerticalOrigin::Center as i32 {
            pixel_offset_y += height * 0.5;
        }

        let mut hot_spot = KmlElement::new("hotSpot");
        hot_spot.set_attribute("x", format!("{}", -pixel_offset_x));
        hot_spot.set_attribute("y", format!("{}", pixel_offset_y));
        hot_spot.set_attribute("xunits", "pixels");
        hot_spot.set_attribute("yunits", "pixels");

        icon_style.children.push(hot_spot);
    }

    // We can only specify heading so if axis isn't Z, then we skip the rotation
    // GE treats a heading of zero as no heading but can still point north using a 360 degree angle
    // DEVIATION (sentinel): rotation "definedness" is approximated by the
    // aligned axis check (see module docs).
    if let Some(aligned_axis) = &billboard_graphics.aligned_axis {
        if Cartesian3::equals(Some(&Cartesian3::UNIT_Z), Some(aligned_axis)) {
            let mut rotation = CesiumMath::to_degrees(-billboard_graphics.rotation);
            if rotation == 0.0 {
                rotation = 360.0;
            }

            icon_style.children.push(create_basic_element_with_text(
                "heading",
                Some(&format!("{}", rotation)),
                false,
            ));
        }
    }

    Ok(icon_style)
}

/// Mirror of `createLineString`.
fn create_line_string(
    polyline_graphics: &PolylineGraphics,
    geometries: &mut Vec<KmlElement>,
    styles: &mut Vec<KmlElement>,
) {
    let mut line_string_geometry = KmlElement::new("LineString");

    // Set altitude mode
    let mut altitude_mode = KmlElement::new("altitudeMode");
    let clamp_to_ground = polyline_graphics.clamp_to_ground;
    if clamp_to_ground {
        line_string_geometry
            .children
            .push(create_basic_element_with_text("tessellate", Some("1"), false));
        altitude_mode.text = Some(String::from("clampToGround"));
    } else {
        altitude_mode.text = Some(String::from("absolute"));
    }
    line_string_geometry.children.push(altitude_mode);

    // Set coordinates
    let coordinates = create_basic_element_with_text(
        "coordinates",
        Some(&get_coordinates(&polyline_graphics.positions)),
        false,
    );
    line_string_geometry.children.push(coordinates);

    // DEVIATION: the value model's PolylineGraphics has no zIndex, so the
    // upstream gx:drawOrder element is skipped.

    geometries.push(line_string_geometry);

    // Create style
    let mut line_style = KmlElement::new("LineStyle");

    // DEVIATION (sentinel): the default width 1.0 means "unset".
    if polyline_graphics.width != 1.0 {
        line_style.children.push(create_basic_element_with_text(
            "width",
            Some(&format!("{}", polyline_graphics.width)),
            false,
        ));
    }

    // DEVIATION (sentinel): the default WHITE material color means "no
    // material" upstream.
    if polyline_graphics.material_color != Color::WHITE {
        process_material(Some(&polyline_graphics.material_color), &mut line_style);
    }

    styles.push(line_style);
}

/// Mirror of `getLinearRing`.
fn get_linear_ring(
    positions: &[Cartesian3],
    height: f64,
    per_position_height: bool,
) -> KmlElement {
    let mut coordinate_strings = Vec::with_capacity(positions.len());
    for position in positions {
        if let Some(cartographic) = Cartographic::from_cartesian_new(position, None) {
            coordinate_strings.push(format!(
                "{},{},{}",
                CesiumMath::to_degrees(cartographic.longitude),
                CesiumMath::to_degrees(cartographic.latitude),
                if per_position_height {
                    cartographic.height
                } else {
                    height
                }
            ));
        }
    }

    let mut linear_ring = KmlElement::new("LinearRing");
    linear_ring.children.push(create_basic_element_with_text(
        "coordinates",
        Some(&coordinate_strings.join(" ")),
        false,
    ));

    linear_ring
}

/// Mirror of `getPolygonBoundaries`.
fn get_polygon_boundaries(
    state: &ExportKmlState,
    polygon_graphics: &PolygonGraphics,
    extruded_height: f64,
) -> Vec<KmlElement> {
    let mut height = state.value_getter.get_or(polygon_graphics.height, 0.0);
    let per_position_height = state
        .value_getter
        .get_or(polygon_graphics.per_position_height, false);

    if !per_position_height && extruded_height > 0.0 {
        // We extrude up and KML extrudes down, so if we extrude, set the polygon height to
        // the extruded height so KML will look similar to Cesium
        height = extruded_height;
    }

    let mut boundaries = Vec::new();

    // Polygon boundaries
    let mut outer_boundary_is = KmlElement::new("outerBoundaryIs");
    outer_boundary_is
        .children
        .push(get_linear_ring(&polygon_graphics.hierarchy, height, per_position_height));
    boundaries.push(outer_boundary_is);

    // Hole boundaries
    for hole in &polygon_graphics.holes {
        let mut inner_boundary_is = KmlElement::new("innerBoundaryIs");
        inner_boundary_is
            .children
            .push(get_linear_ring(hole, height, per_position_height));
        boundaries.push(inner_boundary_is);
    }

    boundaries
}

/// Mirror of `createPolygon`. DEVIATION: only the polygon branch exists
/// (rectangles have no counterpart in the entity model).
fn create_polygon(
    state: &ExportKmlState,
    polygon_graphics: &PolygonGraphics,
    geometries: &mut Vec<KmlElement>,
    styles: &mut Vec<KmlElement>,
    _overlays: &mut Vec<KmlElement>,
) {
    let mut polygon_geometry = KmlElement::new("Polygon");

    let extruded_height = state
        .value_getter
        .get_or(polygon_graphics.extruded_height, 0.0);
    if extruded_height > 0.0 {
        polygon_geometry
            .children
            .push(create_basic_element_with_text("extrude", Some("1"), false));
    }

    // Set boundaries
    for boundary in get_polygon_boundaries(state, polygon_graphics, extruded_height) {
        polygon_geometry.children.push(boundary);
    }

    // Set altitude mode
    let mut altitude_mode = KmlElement::new("altitudeMode");
    // DEVIATION: the value model's PolygonGraphics has no heightReference,
    // so the mode is always "absolute".
    altitude_mode.text = Some(get_altitude_mode(HeightReference::None as i32));
    polygon_geometry.children.push(altitude_mode);

    geometries.push(polygon_geometry);

    // Create style
    let mut poly_style = KmlElement::new("PolyStyle");

    // DEVIATION (sentinel): upstream emits <fill> only when fill was
    // explicitly assigned true; the value model cannot distinguish that from
    // the true default, so no fill element is ever emitted (see module docs).

    // DEVIATION (sentinel): the default WHITE material color means "no
    // material" upstream.
    if polygon_graphics.material_color != Color::WHITE {
        process_material(Some(&polygon_graphics.material_color), &mut poly_style);
    }

    let outline = polygon_graphics.outline;
    if outline {
        poly_style
            .children
            .push(create_basic_element_with_text("outline", Some("1"), false));

        // Outline uses LineStyle
        let mut line_style = KmlElement::new("LineStyle");

        let outline_width = polygon_graphics.outline_width;
        line_style.children.push(create_basic_element_with_text(
            "width",
            Some(&format!("{}", outline_width)),
            false,
        ));

        let outline_color = color_to_string(&polygon_graphics.outline_color);
        line_style
            .children
            .push(create_basic_element_with_text("color", Some(&outline_color), false));
        line_style.children.push(create_basic_element_with_text(
            "colorMode",
            Some("normal"),
            false,
        ));

        styles.push(line_style);
    }

    styles.push(poly_style);
}

/// Mirror of `createModelGeometry`.
fn create_model_geometry(
    state: &mut ExportKmlState,
    model_graphics: &ModelGraphics,
) -> Result<KmlElement, String> {
    let mut model_geometry = KmlElement::new("Model");

    // DEVIATION (sentinel): the default scale 1.0 means "unset".
    if model_graphics.scale != 1.0 {
        let scale = model_graphics.scale;
        let mut scale_element = KmlElement::new("scale");
        for axis in ["x", "y", "z"] {
            scale_element.children.push(create_basic_element_with_text(
                axis,
                Some(&format!("{}", scale)),
                false,
            ));
        }
        model_geometry.children.push(scale_element);
    }

    let time = state.time.clone();
    let uri = state.external_file_handler.model(model_graphics, &time)?;

    let mut link = KmlElement::new("Link");
    link.children
        .push(create_basic_element_with_text("href", Some(&uri), false));
    model_geometry.children.push(link);

    Ok(model_geometry)
}

/// Mirror of `createModel`.
fn create_model(
    state: &mut ExportKmlState,
    entity: &Entity,
    model_graphics: &ModelGraphics,
    geometries: &mut Vec<KmlElement>,
    _styles: &mut Vec<KmlElement>,
) -> Result<(), String> {
    // If the point isn't constant then create gx:Track or gx:MultiTrack
    // DEVIATION: positions are constant in the value model, so the track
    // branch is unreachable; a missing position is skipped instead of
    // throwing.
    let entity_position = match &entity.position {
        Some(position) => position,
        None => return Ok(()),
    };

    let mut model_geometry = create_model_geometry(state, model_graphics)?;

    // Set altitude mode
    let mut altitude_mode = KmlElement::new("altitudeMode");
    // DEVIATION: the value model's ModelGraphics has no heightReference, so
    // the mode is always "absolute".
    altitude_mode.text = Some(get_altitude_mode(HeightReference::None as i32));
    model_geometry.children.push(altitude_mode);

    if let Some(cartographic) = Cartographic::from_cartesian_new(entity_position, None) {
        let mut location = KmlElement::new("Location");
        location.children.push(create_basic_element_with_text(
            "longitude",
            Some(&format!("{}", CesiumMath::to_degrees(cartographic.longitude))),
            false,
        ));
        location.children.push(create_basic_element_with_text(
            "latitude",
            Some(&format!("{}", CesiumMath::to_degrees(cartographic.latitude))),
            false,
        ));
        location.children.push(create_basic_element_with_text(
            "altitude",
            Some(&format!("{}", cartographic.height)),
            false,
        ));
        model_geometry.children.push(location);
    }

    geometries.push(model_geometry);

    Ok(())
}

/// Mirror of `processMaterial`.
///
/// DEVIATION: the value model only carries a plain material color, so only
/// the "Color" branch of the upstream switch is reachable (Image/Grid/Glow/
/// Arrow/Dash/Outline/Stripe materials have no counterpart).
fn process_material(color: Option<&Color>, style: &mut KmlElement) {
    let color = match color {
        Some(color) => color,
        None => return,
    };

    let color = color_to_string(color);
    style
        .children
        .push(create_basic_element_with_text("color", Some(&color), false));
    style
        .children
        .push(create_basic_element_with_text("colorMode", Some("normal"), false));
}

/// Mirror of `getAltitudeMode` (returns the text node content).
fn get_altitude_mode(height_reference: i32) -> String {
    if height_reference == HeightReference::ClampToGround as i32 {
        String::from("clampToGround")
    } else if height_reference == HeightReference::RelativeToGround as i32 {
        String::from("relativeToGround")
    } else {
        String::from("absolute")
    }
}

/// Mirror of `getCoordinates`.
fn get_coordinates(coordinates: &[Cartesian3]) -> String {
    let mut coordinate_strings = Vec::with_capacity(coordinates.len());
    for coordinate in coordinates {
        if let Some(cartographic) = Cartographic::from_cartesian_new(coordinate, None) {
            coordinate_strings.push(format!(
                "{},{},{}",
                CesiumMath::to_degrees(cartographic.longitude),
                CesiumMath::to_degrees(cartographic.latitude),
                cartographic.height
            ));
        }
    }

    coordinate_strings.join(" ")
}

/// Mirror of `colorToString`.
fn color_to_string(color: &Color) -> String {
    let mut result = String::with_capacity(8);
    let bytes = color.to_bytes();
    for i in (0..4).rev() {
        result.push_str(&format!("{:02x}", bytes[i]));
    }
    result
}
