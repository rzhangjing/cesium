//! Ported from `packages/engine/Source/DataSources/GpxDataSource.js`.
//!
//! A [`DataSource`] which processes the GPS Exchange Format (GPX).
//!
//! DEVIATION (browser facilities): the CesiumJS implementation relies on
//! `DOMParser`, `FileReader`, `Resource` fetching, `PinBuilder` canvas
//! images and the `Autolinker` link rewriter. This port parses the GPX text
//! with `quick-xml` into a small internal element tree (same approach as the
//! KML port), reads blobs as UTF-8 text, leaves waypoint billboard images
//! unset unless `GpxLoadOptions::waypoint_image` is provided, and embeds the
//! description paragraphs without link rewriting.
//!
//! DEVIATION (namespace check): CesiumJS matches child elements by local
//! name AND namespace URI (`[null, undefined, "http://www.topografix.com/
//! GPX/1/1"]`); `quick-xml` does not resolve namespace URIs, so this port
//! matches by local name only.
//!
//! DEVIATION (simplified value model): time-dynamic track positions are a
//! `SampledPositionProperty` in CesiumJS; the Rust value model stores the
//! first sample as the constant position together with the availability
//! interval, mirroring the established KML/CZML value-model deviation.
//!
//! DEVIATION (clock derivation): CesiumJS clamps an open availability side
//! to local midnight; this port keeps the availability boundary values
//! as-is (same deviation as the KML port, timezone dependent).
//!
//! DEVIATION (double change pass): the JS pipeline derives the clock twice
//! (`loadGpx` and the `load` promise continuation) which can raise
//! `changedEvent` twice per load; this port consolidates the two passes.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::color::Color;
use cesium_core::event::Event;
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_core::time_interval::TimeInterval;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::horizontal_origin::HorizontalOrigin;
use cesium_scene::label_style::LabelStyle;
use cesium_scene::vertical_origin::VerticalOrigin;

use crate::billboard_graphics::BillboardGraphics;
use crate::data_source::DataSource;
use crate::data_source_clock::DataSourceClock;
use crate::entity_collection::EntityCollection;
use crate::entity_cluster::EntityCluster;
use crate::label_graphics::LabelGraphics;
use crate::polyline_graphics::PolylineGraphics;

const BILLBOARD_SIZE: f64 = 32.0;
const BILLBOARD_NEAR_DISTANCE: f64 = 2414016.0;
const BILLBOARD_NEAR_RATIO: f64 = 1.0;
const BILLBOARD_FAR_DISTANCE: f64 = 1.6093e7;
const BILLBOARD_FAR_RATIO: f64 = 0.1;

/// Options for loading GPX data (mirror of the `options` argument of
/// `GpxDataSource.load`).
#[derive(Clone, Default)]
pub struct GpxLoadOptions {
    /// true if the symbols should be rendered at the same height as the
    /// terrain.
    pub clamp_to_ground: bool,
    /// Image to use for waypoint billboards.
    pub waypoint_image: Option<String>,
    /// Image to use for track billboards.
    ///
    /// DEVIATION: CesiumJS documents this option but never uses it (the
    /// track billboard also uses `waypointImage`); the field is kept for
    /// API parity.
    pub track_image: Option<String>,
    /// Color to use for track lines.
    pub track_color: Option<Color>,
    /// Color to use for route lines.
    pub route_color: Option<Color>,
}

// ============================================================================
// Internal XML element tree (replaces the browser DOM)
// ============================================================================

/// A namespace-agnostic XML element (mirror of the subset of DOM `Element`
/// used by GpxDataSource.js: `localName`, attributes, children, text).
#[derive(Debug, Clone)]
struct XmlElement {
    /// The local name with any namespace prefix stripped.
    local_name: String,
    /// Attribute (name, value) pairs; prefixed names are kept verbatim.
    attributes: Vec<(String, String)>,
    /// Direct child elements.
    children: Vec<XmlElement>,
    /// The direct text of this element.
    text: String,
}

impl XmlElement {
    /// Mirrors DOM `textContent`: the concatenated text of this element and
    /// all of its descendants.
    fn text_content(&self) -> String {
        let mut result = self.text.clone();
        for child in &self.children {
            result.push_str(&child.text_content());
        }
        result
    }
}

fn local_name(name: &str) -> String {
    match name.rfind(':') {
        Some(index) => name[index + 1..].to_string(),
        None => name.to_string(),
    }
}

/// Parses GPX text into an element tree (replaces `DOMParser`).
fn parse_xml(text: &str) -> Result<XmlElement, String> {
    use quick_xml::events::Event as XmlEvent;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text_start = true;
    reader.config_mut().trim_text_end = true;

    let mut stack: Vec<XmlElement> = Vec::new();
    let mut root: Option<XmlElement> = None;
    let mut buf: Vec<u8> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(start)) => {
                let element = element_from_start(&start);
                stack.push(element);
            }
            Ok(XmlEvent::Empty(start)) => {
                let element = element_from_start(&start);
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                }
            }
            Ok(XmlEvent::End(_)) => {
                let element = stack
                    .pop()
                    .ok_or_else(|| "GPX document has unbalanced tags".to_string())?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                }
            }
            Ok(XmlEvent::Text(text)) => {
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&text.unescape().unwrap_or_default());
                }
            }
            Ok(XmlEvent::CData(cdata)) => {
                if let Some(parent) = stack.last_mut() {
                    parent
                        .text
                        .push_str(&String::from_utf8_lossy(cdata.as_ref()));
                }
            }
            Ok(XmlEvent::Eof) => break,
            Err(error) => return Err(format!("GPX parse error: {}", error)),
            _ => {}
        }
        buf.clear();
    }

    root.ok_or_else(|| "GPX document is empty".to_string())
}

fn element_from_start(start: &quick_xml::events::BytesStart<'_>) -> XmlElement {
    let name = String::from_utf8_lossy(start.name().as_ref()).to_string();
    let mut attributes = Vec::new();
    for attribute in start.attributes().flatten() {
        let key = String::from_utf8_lossy(attribute.key.as_ref()).to_string();
        let value = String::from_utf8_lossy(&attribute.value).to_string();
        attributes.push((key, value));
    }
    XmlElement {
        local_name: local_name(&name),
        attributes,
        children: Vec::new(),
        text: String::new(),
    }
}

// ============================================================================
// Query helpers (mirrors of the queryXxx family in GpxDataSource.js)
// ============================================================================

/// Mirror of `readBlobAsText`: decodes the blob bytes as UTF-8 text.
fn read_blob_as_text(blob: &[u8]) -> Result<String, String> {
    String::from_utf8(blob.to_vec()).map_err(|error| error.to_string())
}

/// Mirror of `createGuid` (deterministic counter-based variant, same
/// approach as the KML port).
fn create_guid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{:016x}-{:016x}", nanos, count)
}

/// Mirror of `getOrCreateEntity`: the entity id comes from the `id`
/// attribute when present, otherwise a new GUID is generated.
fn entity_id_from_node(node: &XmlElement) -> String {
    match query_string_attribute(node, "id") {
        Some(id) => id,
        None => create_guid(),
    }
}

/// Mirror of `readCoordinateFromNode`.
fn read_coordinate_from_node(node: &XmlElement) -> Result<Cartesian3, String> {
    let longitude = query_numeric_attribute(node, "lon");
    let latitude = query_numeric_attribute(node, "lat");
    let elevation = query_numeric_value(node, "ele");
    let (Some(longitude), Some(latitude)) = (longitude, latitude) else {
        // Mirrors the CesiumJS DeveloperError raised by
        // `Cartesian3.fromDegrees` for missing/invalid coordinates.
        return Err("GPX - waypoint is missing longitude or latitude".to_string());
    };
    Ok(Cartesian3::from_degrees_new(
        longitude,
        latitude,
        Some(elevation.unwrap_or(0.0)),
        None,
    ))
}

/// Mirror of `queryNumericAttribute` (`parseFloat` semantics: `NaN` maps to
/// `None`).
fn query_numeric_attribute(node: &XmlElement, attribute_name: &str) -> Option<f64> {
    let value = query_string_attribute(node, attribute_name)?;
    parse_float(&value)
}

/// Mirror of `queryStringAttribute`.
fn query_string_attribute(node: &XmlElement, attribute_name: &str) -> Option<String> {
    node.attributes
        .iter()
        .find(|(key, _)| key == attribute_name)
        .map(|(_, value)| value.clone())
}

/// Mirror of `queryFirstNode`.
///
/// DEVIATION: the namespace URI filter is approximated with a local-name
/// match (see module-level note).
fn query_first_node<'a>(node: &'a XmlElement, tag_name: &str) -> Option<&'a XmlElement> {
    node.children
        .iter()
        .find(|child| child.local_name == tag_name)
}

/// Mirror of `queryNodes` (`getElementsByTagName` semantics: all
/// descendants).
fn query_nodes_recursive<'a>(node: &'a XmlElement, tag_name: &str) -> Vec<&'a XmlElement> {
    let mut result = Vec::new();
    for child in &node.children {
        if child.local_name == tag_name {
            result.push(child);
        }
        result.extend(query_nodes_recursive(child, tag_name));
    }
    result
}

/// Mirror of `queryNumericValue`.
fn query_numeric_value(node: &XmlElement, tag_name: &str) -> Option<f64> {
    let result_node = query_first_node(node, tag_name)?;
    parse_float(&result_node.text_content())
}

/// Mirror of `queryStringValue` (returns the trimmed `textContent`).
fn query_string_value(node: &XmlElement, tag_name: &str) -> Option<String> {
    let result = query_first_node(node, tag_name)?;
    Some(result.text_content().trim().to_string())
}

/// Mirror of JS `parseFloat`: parses the longest numeric prefix of the
/// trimmed input; `NaN` results map to `None` (the callers use the
/// `!isNaN(result)` guard).
fn parse_float(value: &str) -> Option<f64> {
    let trimmed = value.trim_start();
    let mut end = trimmed.len();
    while end > 0 {
        if let Ok(result) = trimmed[..end].parse::<f64>() {
            return if result.is_nan() { None } else { Some(result) };
        }
        // Step back one character boundary (UTF-8 safe).
        end = trimmed[..end]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0);
    }
    None
}

// ============================================================================
// Default graphics factories
// ============================================================================

/// Mirror of `createDefaultBillboard`.
///
/// DEVIATION: the `image` argument is a URL/string in this port; the
/// CesiumJS `PinBuilder` canvas image is produced by the caller.
fn create_default_billboard(image: Option<String>) -> BillboardGraphics {
    let mut billboard = BillboardGraphics::new();
    billboard.width = Some(BILLBOARD_SIZE);
    billboard.height = Some(BILLBOARD_SIZE);
    billboard.scale_by_distance = Some(NearFarScalar::new(
        BILLBOARD_NEAR_DISTANCE,
        BILLBOARD_NEAR_RATIO,
        BILLBOARD_FAR_DISTANCE,
        BILLBOARD_FAR_RATIO,
    ));
    billboard.pixel_offset_scale_by_distance = Some(NearFarScalar::new(
        BILLBOARD_NEAR_DISTANCE,
        BILLBOARD_NEAR_RATIO,
        BILLBOARD_FAR_DISTANCE,
        BILLBOARD_FAR_RATIO,
    ));
    billboard.vertical_origin = VerticalOrigin::Bottom as i32;
    billboard.image = image;
    billboard
}

/// Mirror of `createDefaultLabel`.
fn create_default_label() -> LabelGraphics {
    let mut label = LabelGraphics::new();
    label.translucency_by_distance = Some(NearFarScalar::new(3000000.0, 1.0, 5000000.0, 0.0));
    label.pixel_offset = Some((17.0, 0.0));
    label.horizontal_origin = HorizontalOrigin::Left as i32;
    label.font = Some("16px sans-serif".to_string());
    label.style = LabelStyle::FillAndOutline as i32;
    label
}

/// Mirror of `createDefaultPolyline`.
///
/// DEVIATION: CesiumJS uses a `PolylineOutlineMaterialProperty` with
/// `outlineWidth` 2 and `outlineColor` BLACK; the Rust value model stores
/// only the material color, so the outline parameters are dropped.
fn create_default_polyline(color: Option<Color>) -> PolylineGraphics {
    let mut polyline = PolylineGraphics::new();
    polyline.width = 4.0;
    polyline.material_color = color.unwrap_or(Color::RED);
    polyline
}

// ============================================================================
// Description processing
// ============================================================================

/// This is a list of the Optional Description Information:
///   `<cmt>` GPS comment of the waypoint
///   `<desc>` Descriptive description of the waypoint
///   `<src>` Source of the waypoint data
///   `<type>` Type (category) of waypoint
///
/// Mirror of `descriptiveInfoTypes` (insertion order preserved).
const DESCRIPTIVE_INFO_TYPES: &[(&str, &str)] = &[
    ("Time", "time"),
    ("Comment", "cmt"),
    ("Description", "desc"),
    ("Source", "src"),
    ("GPS track/route number", "number"),
    ("Type", "type"),
];

/// Mirror of `processDescription`.
///
/// DEVIATION: the Autolinker link rewriting and the DOM `target="_blank"`
/// pass are skipped; the paragraphs are embedded verbatim into the same
/// wrapper div structure.
fn process_description(node: &XmlElement) -> Option<String> {
    let mut text = String::new();
    for (info_text, info_tag) in DESCRIPTIVE_INFO_TYPES {
        let value = query_string_value(node, info_tag).unwrap_or_default();
        if !value.is_empty() {
            text.push_str(&format!("<p>{}: {}</p>", info_text, value));
        }
    }

    if text.is_empty() {
        // No description
        return None;
    }

    let background = Color::WHITE;
    let foreground = Color::BLACK;
    let mut result = String::from("<div class=\"cesium-infoBox-description-lighter\" style=\"");
    result.push_str("overflow:auto;");
    result.push_str("word-wrap:break-word;");
    result.push_str(&format!(
        "background-color:{};",
        to_css_color_string(&background)
    ));
    result.push_str(&format!("color:{};", to_css_color_string(&foreground)));
    result.push_str("\">");
    result.push_str(&text);
    result.push_str("</div>");
    Some(result)
}

/// Mirror of `Color.toCssColorString` (opaque RGB form used here).
fn to_css_color_string(color: &Color) -> String {
    format!(
        "rgb({}, {}, {})",
        (color.red * 255.0).round() as i32,
        (color.green * 255.0).round() as i32,
        (color.blue * 255.0).round() as i32
    )
}

// ============================================================================
// Geometry processing
// ============================================================================

/// Mirror of `processWpt`.
fn process_wpt(
    entity_collection: &mut EntityCollection,
    geometry_node: &XmlElement,
    options: &GpxLoadOptions,
) -> Result<(), String> {
    let position = read_coordinate_from_node(geometry_node)?;

    let id = entity_id_from_node(geometry_node);
    let entity = entity_collection.get_or_create_entity(&id);
    entity.position = Some(position);

    // Get billboard image
    // DEVIATION: no PinBuilder; the default marker image is omitted unless
    // `waypointImage` is provided.
    let image = options.waypoint_image.clone();
    entity.billboard = Some(create_default_billboard(image));

    let name = query_string_value(geometry_node, "name");
    entity.name = name.clone();
    let mut label = create_default_label();
    label.text = name;
    entity.label = Some(label);
    entity.description = process_description(geometry_node);

    if options.clamp_to_ground {
        entity
            .billboard
            .as_mut()
            .expect("billboard created above")
            .height_reference = HeightReference::ClampToGround as i32;
        // DEVIATION: CesiumJS also sets `label.heightReference`; the Rust
        // LabelGraphics value model has no height reference field.
    }
    Ok(())
}

/// Mirror of `processRte`: rte represents a route - an ordered list of
/// waypoints representing a series of turn points leading to a destination.
fn process_rte(
    entity_collection: &mut EntityCollection,
    geometry_node: &XmlElement,
    options: &GpxLoadOptions,
) -> Result<(), String> {
    let id = entity_id_from_node(geometry_node);
    {
        let entity = entity_collection.get_or_create_entity(&id);
        entity.description = process_description(geometry_node);
    }

    // A list of waypoints
    let route_points = query_nodes_recursive(geometry_node, "rtept");
    let mut coordinate_tuples = Vec::with_capacity(route_points.len());
    for route_point in route_points {
        process_wpt(entity_collection, route_point, options)?;
        coordinate_tuples.push(read_coordinate_from_node(route_point)?);
    }

    let entity = entity_collection
        .get_by_id_mut(&id)
        .expect("entity created above");
    let mut polyline = create_default_polyline(options.route_color);
    if options.clamp_to_ground {
        polyline.clamp_to_ground = true;
    }
    polyline.positions = coordinate_tuples;
    entity.polyline = Some(polyline);
    Ok(())
}

/// Mirror of `processTrk`: trk represents a track - an ordered list of
/// points describing a path.
///
/// DEVIATION (value model): the time-dynamic branch assigns a
/// `SampledPositionProperty` in CesiumJS; this port stores the first sample
/// as the constant position plus the availability interval.
fn process_trk(
    entity_collection: &mut EntityCollection,
    geometry_node: &XmlElement,
    options: &GpxLoadOptions,
) -> Result<(), String> {
    let id = entity_id_from_node(geometry_node);
    {
        let entity = entity_collection.get_or_create_entity(&id);
        entity.description = process_description(geometry_node);
    }

    let track_segs = query_nodes_recursive(geometry_node, "trkseg");
    let mut positions: Vec<Cartesian3> = Vec::new();
    let mut times: Vec<JulianDate> = Vec::new();
    let mut is_time_dynamic = true;
    for track_seg in track_segs {
        let track_seg_info = process_trk_seg(track_seg)?;
        positions.extend(track_seg_info.0);
        if !track_seg_info.1.is_empty() {
            times.extend(track_seg_info.1);
            // If one track segment is non-dynamic the whole track must
            // also be (mirror of the JS `isTimeDynamic && true` branch).
        } else {
            is_time_dynamic = false;
        }
    }
    // The JS branch also fires with zero segments (creating an invalid
    // interval); the `!times.is_empty()` guard keeps the port total.
    if is_time_dynamic && !times.is_empty() {
        let entity = entity_collection
            .get_by_id_mut(&id)
            .expect("entity created above");

        // Assign billboard image
        let image = options.waypoint_image.clone();
        entity.billboard = Some(create_default_billboard(image));
        // DEVIATION: `entity.position = property` (SampledPositionProperty)
        // becomes the first sample in the constant value model.
        entity.position = positions.first().copied();
        if options.clamp_to_ground {
            entity
                .billboard
                .as_mut()
                .expect("billboard created above")
                .height_reference = HeightReference::ClampToGround as i32;
        }
        entity.availability = vec![TimeInterval::new(
            Some(times[0].clone()),
            Some(times[times.len() - 1].clone()),
            None,
            None,
        )];
    }

    let entity = entity_collection
        .get_by_id_mut(&id)
        .expect("entity created above");
    let mut polyline = create_default_polyline(options.track_color);
    polyline.positions = positions;
    if options.clamp_to_ground {
        polyline.clamp_to_ground = true;
    }
    entity.polyline = Some(polyline);
    Ok(())
}

/// Mirror of `processTrkSeg`.
fn process_trk_seg(node: &XmlElement) -> Result<(Vec<Cartesian3>, Vec<JulianDate>), String> {
    let mut positions = Vec::new();
    let mut times = Vec::new();
    let track_points = query_nodes_recursive(node, "trkpt");
    for track_point in track_points {
        let position = read_coordinate_from_node(track_point)?;
        positions.push(position);

        if let Some(time) = query_string_value(track_point, "time") {
            // Mirrors `JulianDate.fromIso8601` throwing on invalid input.
            let date = JulianDate::from_iso8601(&time)
                .ok_or_else(|| format!("GPX - invalid time value: {}", time))?;
            times.push(date);
        }
    }
    Ok((positions, times))
}

// ============================================================================
// Metadata processing (GPX schema types)
// ============================================================================

/// A GPX `linkType` object.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxLink {
    /// The `href` attribute of the link.
    pub href: Option<String>,
    /// The text of the link.
    pub text: Option<String>,
    /// The MIME type of the linked content (`type` child).
    pub mime_type: Option<String>,
}

/// A GPX `personType` object.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxPerson {
    /// The person or organization name.
    pub name: Option<String>,
    /// The email address (`id@domain`).
    pub email: Option<String>,
    /// The external link.
    pub link: Option<GpxLink>,
}

/// A GPX `copyrightType` object.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxCopyright {
    /// The copyright holder (`author` attribute).
    pub author: Option<String>,
    /// The year of copyright.
    pub year: Option<String>,
    /// The license of the GPX data.
    pub license: Option<String>,
}

/// A GPX `boundsType` object.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxBounds {
    /// The minimum latitude.
    pub min_lat: Option<f64>,
    /// The maximum latitude.
    pub max_lat: Option<f64>,
    /// The minimum longitude.
    pub min_lon: Option<f64>,
    /// The maximum longitude.
    pub max_lon: Option<f64>,
}

/// A GPX `metadataType` object.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxMetadata {
    /// The name of the GPX data.
    pub name: Option<String>,
    /// A description of the GPX data.
    pub desc: Option<String>,
    /// The author of the GPX data.
    pub author: Option<GpxPerson>,
    /// The copyright information.
    pub copyright: Option<GpxCopyright>,
    /// The external link.
    pub link: Option<GpxLink>,
    /// The creation time of the GPX data.
    pub time: Option<String>,
    /// The keywords associated with the GPX data.
    pub keywords: Option<String>,
    /// The bounds of the GPX data.
    pub bounds: Option<GpxBounds>,
}

/// Processes a metadataType node and returns a metadata object
/// (GPX schema `metadataType`).
fn process_metadata(node: &XmlElement) -> Option<GpxMetadata> {
    let metadata_node = query_first_node(node, "metadata")?;
    let metadata = GpxMetadata {
        name: query_string_value(metadata_node, "name"),
        desc: query_string_value(metadata_node, "desc"),
        author: get_person(metadata_node),
        copyright: get_copyright(metadata_node),
        link: get_link(metadata_node),
        time: query_string_value(metadata_node, "time"),
        keywords: query_string_value(metadata_node, "keywords"),
        bounds: get_bounds(metadata_node),
    };
    if metadata.name.is_some()
        || metadata.desc.is_some()
        || metadata.author.is_some()
        || metadata.copyright.is_some()
        || metadata.link.is_some()
        || metadata.time.is_some()
        || metadata.keywords.is_some()
        || metadata.bounds.is_some()
    {
        Some(metadata)
    } else {
        None
    }
}

/// Receives a XML node and returns a personType object (GPX schema
/// `personType`).
fn get_person(node: &XmlElement) -> Option<GpxPerson> {
    let person_node = query_first_node(node, "author")?;
    let person = GpxPerson {
        name: query_string_value(person_node, "name"),
        email: get_email(person_node),
        link: get_link(person_node),
    };
    if person.name.is_some() || person.email.is_some() || person.link.is_some() {
        Some(person)
    } else {
        None
    }
}

/// Receives a XML node and returns an email address (from emailType, GPX
/// schema `emailType`).
fn get_email(node: &XmlElement) -> Option<String> {
    let email_node = query_first_node(node, "email")?;
    let id = query_string_value(email_node, "id").unwrap_or_default();
    let domain = query_string_value(email_node, "domain").unwrap_or_default();
    Some(format!("{}@{}", id, domain))
}

/// Receives a XML node and returns a linkType object (GPX schema
/// `linkType`).
fn get_link(node: &XmlElement) -> Option<GpxLink> {
    let link_node = query_first_node(node, "link")?;
    let link = GpxLink {
        href: query_string_attribute(link_node, "href"),
        text: query_string_value(link_node, "text"),
        mime_type: query_string_value(link_node, "type"),
    };
    if link.href.is_some() || link.text.is_some() || link.mime_type.is_some() {
        Some(link)
    } else {
        None
    }
}

/// Receives a XML node and returns a copyrightType object (GPX schema
/// `copyrightType`).
fn get_copyright(node: &XmlElement) -> Option<GpxCopyright> {
    let copyright_node = query_first_node(node, "copyright")?;
    let copyright = GpxCopyright {
        author: query_string_attribute(copyright_node, "author"),
        year: query_string_value(copyright_node, "year"),
        license: query_string_value(copyright_node, "license"),
    };
    if copyright.author.is_some() || copyright.year.is_some() || copyright.license.is_some() {
        Some(copyright)
    } else {
        None
    }
}

/// Receives a XML node and returns a boundsType object (GPX schema
/// `boundsType`).
fn get_bounds(node: &XmlElement) -> Option<GpxBounds> {
    let bounds_node = query_first_node(node, "bounds")?;
    let bounds = GpxBounds {
        min_lat: query_numeric_value(bounds_node, "minlat"),
        max_lat: query_numeric_value(bounds_node, "maxlat"),
        min_lon: query_numeric_value(bounds_node, "minlon"),
        max_lon: query_numeric_value(bounds_node, "maxlon"),
    };
    if bounds.min_lat.is_some()
        || bounds.max_lat.is_some()
        || bounds.min_lon.is_some()
        || bounds.max_lon.is_some()
    {
        Some(bounds)
    } else {
        None
    }
}

// ============================================================================
// Document processing
// ============================================================================

/// The order of the complex type dispatch (mirror of the `complexTypes`
/// object key order: wpt, rte, trk).
const COMPLEX_TYPES: &[&str] = &["wpt", "rte", "trk"];

/// Mirror of `processGpx`: processes each top-level `wpt`/`rte`/`trk`
/// child (each type in document order).
fn process_gpx(
    entity_collection: &mut EntityCollection,
    node: &XmlElement,
    options: &GpxLoadOptions,
) -> Result<(), String> {
    for type_name in COMPLEX_TYPES {
        for child in &node.children {
            if child.local_name == *type_name {
                match *type_name {
                    "wpt" => process_wpt(entity_collection, child, options)?,
                    "rte" => process_rte(entity_collection, child, options)?,
                    _ => process_trk(entity_collection, child, options)?,
                }
            }
        }
    }
    Ok(())
}

/// Mirror of `metadataChanged`, including the CesiumJS typos:
/// `old.dec !== current.desc` (so any defined `desc` always counts as a
/// change) and `old.src !== current.src` (both sides are always undefined
/// and never contribute).
fn metadata_changed(old: Option<&GpxMetadata>, current: Option<&GpxMetadata>) -> bool {
    let (Some(old), Some(current)) = (old, current) else {
        // One side undefined (or both): changed unless both undefined.
        return !(old.is_none() && current.is_none());
    };
    // JS reference comparisons on the freshly built sub-objects: a
    // sub-object counts as changed unless both sides are undefined.
    if old.name != current.name
        || current.desc.is_some() // mirror of the `old.dec` typo
        || !(old.author.is_none() && current.author.is_none())
        || !(old.copyright.is_none() && current.copyright.is_none())
        || !(old.link.is_none() && current.link.is_none())
        || old.time != current.time
        || !(old.bounds.is_none() && current.bounds.is_none())
    {
        return true;
    }
    false
}

/// Mirror of the clock derivation shared by `loadGpx` and the `load`
/// continuation.
///
/// DEVIATION: CesiumJS clamps an open side to the local midnight; this
/// port keeps the availability boundary values as-is.
fn compute_clock_from_availability(
    entity_collection: &EntityCollection,
) -> Option<DataSourceClock> {
    let availability = entity_collection.compute_availability();
    let is_min_start = JulianDate::equals(&availability.start, Iso8601::minimum_value());
    let is_max_stop = JulianDate::equals(&availability.stop, Iso8601::maximum_value());
    if is_min_start && is_max_stop {
        return None;
    }

    let start = availability.start.clone();
    let stop = availability.stop.clone();

    let mut clock = DataSourceClock::new();
    clock.start_time = start.clone();
    clock.stop_time = stop.clone();
    clock.current_time = start;
    clock.clock_range = ClockRange::LoopStop;
    clock.clock_step = ClockStep::SystemClockMultiplier;
    clock.multiplier = (JulianDate::seconds_difference(&clock.stop_time, &clock.start_time) / 60.0)
        .clamp(1.0, 3.15569e7)
        .round();
    Some(clock)
}

/// Mirror of `loadGpx`.
fn load_gpx(
    data_source: &mut GpxDataSource,
    root: &XmlElement,
    options: &GpxLoadOptions,
) -> Result<(), String> {
    data_source.entity_collection.remove_all();

    let version = query_string_attribute(root, "version");
    let creator = query_string_attribute(root, "creator");

    let metadata = process_metadata(root);
    let name = metadata.as_ref().and_then(|metadata| metadata.name.clone());

    if root.local_name == "gpx" {
        process_gpx(&mut data_source.entity_collection, root, options)?;
    } else {
        // Mirror of `console.log("GPX - Unsupported node: ...")`.
        eprintln!("GPX - Unsupported node: {}", root.local_name);
    }

    let clock = compute_clock_from_availability(&data_source.entity_collection);

    let mut changed = false;
    if data_source.name != name {
        data_source.name = name;
        changed = true;
    }

    if data_source.creator != creator {
        data_source.creator = creator;
        changed = true;
    }

    if metadata_changed(data_source.metadata.as_ref(), metadata.as_ref()) {
        data_source.metadata = metadata;
        changed = true;
    }

    if data_source.version != version {
        data_source.version = version;
        changed = true;
    }

    // Mirror of `clock !== dataSource._clock`: a freshly derived clock is a
    // new object, so it always differs; only `undefined` === `undefined`.
    if clock.is_some() || data_source.clock.is_some() {
        changed = true;
        data_source.clock = clock;
    }

    if changed {
        data_source.changed_event.raise_event(&());
    }

    data_source.set_loading(false);
    Ok(())
}

// ============================================================================
// GpxDataSource
// ============================================================================

/// A [`DataSource`] which processes the GPS Exchange Format (GPX).
///
/// See the Topografix GPX Standard / GPX 1.1 documentation.
pub struct GpxDataSource {
    name: Option<String>,
    version: Option<String>,
    creator: Option<String>,
    metadata: Option<GpxMetadata>,
    clock: Option<DataSourceClock>,
    entity_collection: EntityCollection,
    entity_cluster: EntityCluster,
    is_loading: bool,
    is_destroyed: bool,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
}

impl GpxDataSource {
    /// Creates a new GPX data source (mirror of the `GpxDataSource`
    /// constructor).
    pub fn new() -> Self {
        Self {
            name: None,
            version: None,
            creator: None,
            metadata: None,
            clock: None,
            entity_collection: EntityCollection::new(),
            entity_cluster: EntityCluster::new(),
            is_loading: false,
            is_destroyed: false,
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
        }
    }

    /// Creates a new instance loaded with the provided GPX data (mirror of
    /// the static `GpxDataSource.load`).
    pub fn load(data: &str, options: Option<&GpxLoadOptions>) -> Result<GpxDataSource, String> {
        let mut data_source = GpxDataSource::new();
        data_source.load_value(data, options)?;
        Ok(data_source)
    }

    /// Loads GPX from an XML text string (or a file path, see
    /// [`GpxDataSource::load_file`]), replacing any existing data.
    ///
    /// Mirror of `GpxDataSource.prototype.load` for the string/Document
    /// data forms; the Rust port is synchronous.
    pub fn load_value(
        &mut self,
        xml: &str,
        options: Option<&GpxLoadOptions>,
    ) -> Result<(), String> {
        let options_owned = options.cloned().unwrap_or_default();
        self.set_loading(true);
        let result = parse_xml(xml).and_then(|root| load_gpx(self, &root, &options_owned));
        if let Err(ref error) = result {
            self.set_loading(false);
            self.error_event.raise_event(&());
            let _ = error; // Mirrors the JS `console.log(error)`.
            return Err(result.unwrap_err());
        }
        Ok(())
    }

    /// Loads GPX from binary blob data (mirror of the `Blob` form of
    /// `GpxDataSource.prototype.load`, `readBlobAsText` included).
    pub fn load_blob(&mut self, blob: &[u8], options: Option<&GpxLoadOptions>) -> Result<(), String> {
        let text = read_blob_as_text(blob)?;
        self.load_value(&text, options)
    }

    /// Loads GPX from a file path (the Rust stand-in for the JS URL form
    /// backed by `Resource.fetchBlob`).
    pub fn load_file(&mut self, path: &str, options: Option<&GpxLoadOptions>) -> Result<(), String> {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("GPX - failed to read {}: {}", path, error))?;
        self.load_blob(&bytes, options)
    }

    /// Gets the version of the GPX Schema in use.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Gets the creator of the GPX document.
    pub fn creator(&self) -> Option<&str> {
        self.creator.as_deref()
    }

    /// Gets an object containing metadata about the GPX file.
    pub fn metadata(&self) -> Option<&GpxMetadata> {
        self.metadata.as_ref()
    }

    /// Gets the clock settings defined by the loaded GPX. This represents
    /// the total availability interval for all time-dynamic data. If the
    /// GPX does not contain time-dynamic data, this value is `None`.
    pub fn clock(&self) -> Option<&DataSourceClock> {
        self.clock.as_ref()
    }

    /// Gets the clustering options for this data source.
    pub fn clustering(&self) -> &EntityCluster {
        &self.entity_cluster
    }

    /// Sets the clustering options for this data source (the debug check
    /// mirrors the `includeStart('debug')` guard).
    pub fn set_clustering(&mut self, value: EntityCluster) {
        self.entity_cluster = value;
    }

    /// Updates the data source to the provided time (mirror of
    /// `GpxDataSource.prototype.update`: always ready).
    pub fn update(&self, _time: &JulianDate) -> bool {
        true
    }

    fn set_loading(&mut self, loading: bool) {
        if self.is_loading != loading {
            self.is_loading = loading;
            self.loading_event.raise_event(&());
        }
    }
}

impl Default for GpxDataSource {
    fn default() -> Self { Self::new() }
}

impl DataSource for GpxDataSource {
    fn name(&self) -> &str { self.name.as_deref().unwrap_or("") }
    fn entities(&self) -> &EntityCollection { &self.entity_collection }
    fn is_loading(&self) -> bool { self.is_loading }
    fn is_destroyed(&self) -> bool { self.is_destroyed }
    fn changed_event(&self) -> &Event { &self.changed_event }
    fn error_event(&self) -> &Event { &self.error_event }
    fn loading_event(&self) -> &Event { &self.loading_event }
    fn show(&self) -> bool { self.entity_collection.show }
    fn set_show(&mut self, show: bool) { self.entity_collection.show = show; }
    fn destroy(&mut self) {
        self.entity_collection.destroy();
        self.is_destroyed = true;
    }
}
