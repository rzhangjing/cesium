//! Ported from `packages/engine/Source/DataSources/KmlDataSource.js`.
//!
//! A data source that loads KML (Keyhole Markup Language) files.
//!
//! DEVIATION (browser facilities): the CesiumJS implementation relies on
//! `DOMParser`, `Resource` fetching, KMZ (zip) decoding, `PinBuilder` canvas
//! images, NetworkLink timers and ScreenOverlay DOM nodes. This port parses
//! the KML text with `quick-xml` into a small internal element tree and
//! materializes the constant feature/geometry/style subset directly onto the
//! entities. KMZ archives, NetworkLink, Tour, ScreenOverlay, GroundOverlay,
//! gx:Track/MultiTrack and pin images are not produced.
//!
//! DEVIATION (simplified value model): time-dynamic `Property` objects are
//! replaced by the stored constant values, mirroring the CZML port.

use std::collections::HashMap;

use cesium_core::arc_type::ArcType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::color::Color;
use cesium_core::credit::Credit;
use cesium_core::event::Event;
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_core::get_filename_from_uri::get_filename_from_uri;
use cesium_core::time_interval::TimeInterval;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::horizontal_origin::HorizontalOrigin;
use cesium_scene::label_style::LabelStyle;

use crate::billboard_graphics::BillboardGraphics;
use crate::data_source::DataSource;
use crate::data_source_clock::DataSourceClock;
use crate::entity::Entity;
use crate::entity_collection::EntityCollection;
use crate::label_graphics::LabelGraphics;
use crate::polygon_graphics::PolygonGraphics;
use crate::polyline_graphics::PolylineGraphics;
use crate::property::PropertyResult;

const BILLBOARD_SIZE: f64 = 32.0;
const BILLBOARD_NEAR_DISTANCE: f64 = 2414016.0;
const BILLBOARD_NEAR_RATIO: f64 = 1.0;
const BILLBOARD_FAR_DISTANCE: f64 = 1.6093e7;
const BILLBOARD_FAR_RATIO: f64 = 0.1;

/// Options for loading KML data (mirror of `KmlDataSource.LoadOptions`).
#[derive(Clone, Default)]
pub struct KmlLoadOptions {
    /// Overrides the url to use for resolving relative links.
    pub source_uri: Option<String>,
    /// true if geometry features should be clamped to the ground.
    pub clamp_to_ground: bool,
    /// A credit for the data source.
    pub credit: Option<String>,
}

/// Builds a `DataSourceClock` from the entity availability (mirror of the
/// clock derivation in the `KmlDataSource.load` success handler).
///
/// DEVIATION: CesiumJS clamps an open interval side to the local midnight;
/// this port keeps the availability boundary values as-is.
fn compute_clock_from_availability(
    entity_collection: &EntityCollection,
) -> Option<DataSourceClock> {
    let availability = entity_collection.compute_availability();
    let is_min_start = JulianDate::equals(&availability.start, Iso8601::minimum_value());
    let is_max_stop = JulianDate::equals(&availability.stop, Iso8601::maximum_value());
    if is_min_start && is_max_stop {
        return None;
    }

    let mut clock = DataSourceClock::new();
    clock.start_time = availability.start.clone();
    clock.stop_time = availability.stop.clone();
    clock.current_time = availability.start.clone();
    clock.clock_range = ClockRange::LoopStop;
    clock.clock_step = ClockStep::SystemClockMultiplier;
    let seconds_difference = JulianDate::seconds_difference(
        &availability.stop,
        &availability.start,
    );
    clock.multiplier = (seconds_difference / 60.0).round().clamp(1.0, 3.15569e7);
    Some(clock)
}

/// Field-wise comparison of two optional clocks (mirror of the JS reference
/// check `clock !== that._clock` used to decide the changed event).
fn clock_equals(left: &Option<DataSourceClock>, right: &Option<DataSourceClock>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            JulianDate::equals(&left.start_time, &right.start_time)
                && JulianDate::equals(&left.stop_time, &right.stop_time)
                && JulianDate::equals(&left.current_time, &right.current_time)
                && left.clock_range == right.clock_range
                && left.clock_step == right.clock_step
                && left.multiplier == right.multiplier
        }
        _ => false,
    }
}

// ============================================================================
// Internal XML element tree (replaces the browser DOM)
// ============================================================================

/// A namespace-agnostic XML element (mirror of the subset of DOM `Element`
/// used by KmlDataSource.js: `localName`, attributes, children, text).
#[derive(Debug, Clone)]
struct XmlElement {
    /// The local name with any namespace prefix stripped.
    local_name: String,
    /// The full tag name including any namespace prefix (used to tell
    /// `gx:altitudeMode` apart from `altitudeMode`).
    full_name: String,
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

/// Parses KML text into an element tree (replaces `DOMParser`).
fn parse_xml(text: &str) -> Result<XmlElement, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text_start = true;
    reader.config_mut().trim_text_end = true;

    let mut stack: Vec<XmlElement> = Vec::new();
    let mut root: Option<XmlElement> = None;
    let mut buf: Vec<u8> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(start)) => {
                let element = element_from_start(&start);
                stack.push(element);
            }
            Ok(Event::Empty(start)) => {
                let element = element_from_start(&start);
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                }
            }
            Ok(Event::End(_)) => {
                let element = stack
                    .pop()
                    .ok_or_else(|| "KML document has unbalanced tags".to_string())?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&text.unescape().unwrap_or_default());
                }
            }
            Ok(Event::CData(cdata)) => {
                if let Some(parent) = stack.last_mut() {
                    parent
                        .text
                        .push_str(&String::from_utf8_lossy(cdata.as_ref()));
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(format!("KML parse error: {}", error)),
            _ => {}
        }
        buf.clear();
    }

    root.ok_or_else(|| "KML document is empty".to_string())
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
        full_name: name,
        attributes,
        children: Vec::new(),
        text: String::new(),
    }
}

// ============================================================================
// Query helpers (mirrors of queryXxx in KmlDataSource.js)
// ============================================================================

/// Mirror of `queryStringAttribute`.
fn query_string_attribute(node: Option<&XmlElement>, name: &str) -> Option<String> {
    let node = node?;
    node.attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

/// Mirror of `queryFirstNode` (direct children, local name match).
fn query_first_node<'a>(node: Option<&'a XmlElement>, tag_name: &str) -> Option<&'a XmlElement> {
    let node = node?;
    node.children
        .iter()
        .find(|child| child.local_name == tag_name)
}

/// Mirror of `queryChildNodes` (direct children, local name match).
fn query_child_nodes<'a>(node: Option<&'a XmlElement>, tag_name: &str) -> Vec<&'a XmlElement> {
    let Some(node) = node else { return Vec::new() };
    node.children
        .iter()
        .filter(|child| child.local_name == tag_name)
        .collect()
}

/// Mirror of `queryNodes` (`getElementsByTagName` semantics: all descendants).
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

/// Mirror of `queryStringValue`.
fn query_string_value(node: Option<&XmlElement>, tag_name: &str) -> Option<String> {
    let child = query_first_node(node, tag_name)?;
    let text = child.text_content();
    if text.is_empty() { None } else { Some(text) }
}

/// Mirror of `queryNumericValue`.
fn query_numeric_value(node: Option<&XmlElement>, tag_name: &str) -> Option<f64> {
    let value = query_string_value(node, tag_name)?;
    value.trim().parse::<f64>().ok()
}

/// Mirror of `queryBooleanValue` (`checkBooleanValue`: `1`/`true`).
fn query_boolean_value(node: Option<&XmlElement>, tag_name: &str) -> Option<bool> {
    let value = query_string_value(node, tag_name)?;
    let value = value.trim().to_lowercase();
    if value == "1" || value == "true" {
        Some(true)
    } else if value == "0" || value == "false" {
        Some(false)
    } else {
        None
    }
}

/// Mirror of `queryColorValue`.
fn query_color_value(node: Option<&XmlElement>, tag_name: &str) -> Option<Color> {
    let value = query_string_value(node, tag_name)?;
    // DEVIATION: `colorMode="random"` uses the deterministic base color
    // instead of `Color.fromRandom`.
    parse_color_string(&value)
}

// ============================================================================
// Value parsing helpers
// ============================================================================

/// Mirror of `parseColorString`: KML colors are `aabbggrr` hex strings.
fn parse_color_string(value: &str) -> Option<Color> {
    if value.trim().is_empty() {
        return None;
    }
    let value = value.trim().trim_start_matches('#');
    if value.len() < 8 {
        return None;
    }
    let alpha = u8::from_str_radix(&value[0..2], 16).ok()? as f64 / 255.0;
    let blue = u8::from_str_radix(&value[2..4], 16).ok()? as f64 / 255.0;
    let green = u8::from_str_radix(&value[4..6], 16).ok()? as f64 / 255.0;
    let red = u8::from_str_radix(&value[6..8], 16).ok()? as f64 / 255.0;
    Some(Color::new(red, green, blue, alpha))
}

/// Mirror of `readCoordinate`: Google Earth treats empty or missing
/// coordinates as 0.
fn read_coordinate(value: Option<&str>) -> Cartesian3 {
    let Some(value) = value else {
        return Cartesian3::from_degrees_new(0.0, 0.0, Some(0.0), None);
    };
    let digits: Vec<&str> = value
        .split(|c: char| c.is_whitespace() || c == ',' || c == '\n')
        .filter(|token| !token.is_empty())
        .collect();
    if digits.is_empty() {
        return Cartesian3::from_degrees_new(0.0, 0.0, Some(0.0), None);
    }
    let longitude = digits.first().and_then(|d| d.parse::<f64>().ok()).unwrap_or(0.0);
    let latitude = digits.get(1).and_then(|d| d.parse::<f64>().ok()).unwrap_or(0.0);
    let height = digits.get(2).and_then(|d| d.parse::<f64>().ok()).unwrap_or(0.0);
    Cartesian3::from_degrees_new(longitude, latitude, Some(height), None)
}

/// Mirror of `readCoordinates`.
fn read_coordinates(element: Option<&XmlElement>) -> Option<Vec<Cartesian3>> {
    let element = element?;
    let text = element.text_content();
    let tuples: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == '\n')
        .filter(|token| !token.is_empty())
        .collect();
    if tuples.is_empty() {
        return None;
    }
    Some(tuples.iter().map(|tuple| read_coordinate(Some(tuple))).collect())
}

/// Mirror of `isExtrudable`.
fn is_extrudable(altitude_mode: Option<&str>, gx_altitude_mode: Option<&str>) -> bool {
    altitude_mode == Some("absolute")
        || altitude_mode == Some("relativeToGround")
        || gx_altitude_mode == Some("relativeToSeaFloor")
}

/// Mirror of `heightReferenceFromAltitudeMode` (warnings omitted).
fn height_reference_from_altitude_mode(
    altitude_mode: Option<&str>,
    gx_altitude_mode: Option<&str>,
) -> i32 {
    if (altitude_mode.is_none() && gx_altitude_mode.is_none())
        || altitude_mode == Some("clampToGround")
    {
        return HeightReference::ClampToGround as i32;
    }
    if altitude_mode == Some("relativeToGround") {
        return HeightReference::RelativeToGround as i32;
    }
    if altitude_mode == Some("absolute") {
        return HeightReference::None as i32;
    }
    if gx_altitude_mode == Some("clampToSeaFloor") {
        return HeightReference::ClampToGround as i32;
    }
    if gx_altitude_mode == Some("relativeToSeaFloor") {
        return HeightReference::RelativeToGround as i32;
    }
    // Clamp to ground is the default
    HeightReference::ClampToGround as i32
}

/// Reads the KML and gx `altitudeMode` children of a geometry node
/// (distinguished by their namespace prefix, which is stripped from
/// `local_name`).
fn query_altitude_modes(node: &XmlElement) -> (Option<String>, Option<String>) {
    let mut kml_mode = None;
    let mut gx_mode = None;
    for child in &node.children {
        if child.local_name != "altitudeMode" {
            continue;
        }
        let text = child.text_content();
        if text.is_empty() {
            continue;
        }
        if child.full_name.contains(':') {
            gx_mode = Some(text);
        } else {
            kml_mode = Some(text);
        }
    }
    (kml_mode, gx_mode)
}

/// Mirror of `processTimeStamp`.
fn process_time_stamp(feature_node: &XmlElement) -> Option<Vec<TimeInterval>> {
    let node = query_first_node(Some(feature_node), "TimeStamp")?;
    let when_string = query_string_value(Some(node), "when")?;
    if when_string.is_empty() {
        return None;
    }
    let when = JulianDate::from_iso8601(&when_string)?;
    Some(vec![TimeInterval::new(
        Some(when),
        Some(Iso8601::maximum_value().clone()),
        None,
        None,
    )])
}

/// Mirror of `processTimeSpan`.
fn process_time_span(feature_node: &XmlElement) -> Option<Vec<TimeInterval>> {
    let node = query_first_node(Some(feature_node), "TimeSpan")?;

    let begin_date = query_first_node(Some(node), "begin")
        .and_then(|n| JulianDate::from_iso8601(&n.text_content()));
    let end_date = query_first_node(Some(node), "end")
        .and_then(|n| JulianDate::from_iso8601(&n.text_content()));

    match (begin_date, end_date) {
        (Some(mut begin), Some(mut end)) => {
            // The spec flips dates when end is earlier.
            if JulianDate::compare(&end, &begin) < 0 {
                std::mem::swap(&mut begin, &mut end);
            }
            Some(vec![TimeInterval::new(Some(begin), Some(end), None, None)])
        }
        (Some(begin), None) => Some(vec![TimeInterval::new(
            Some(begin),
            Some(Iso8601::maximum_value().clone()),
            None,
            None,
        )]),
        (None, Some(end)) => Some(vec![TimeInterval::new(
            Some(Iso8601::minimum_value().clone()),
            Some(end),
            None,
            None,
        )]),
        (None, None) => None,
    }
}

/// Mirrors the `TimeIntervalCollection.intersect` semantics used by
/// `mergeAvailabilityWithParent`: pairwise intersection of the two interval
/// lists, keeping non-empty results.
fn intersect_availability(left: &[TimeInterval], right: &[TimeInterval]) -> Vec<TimeInterval> {
    let mut result = Vec::new();
    for l in left {
        for r in right {
            let candidate = TimeInterval::intersect(l, r);
            // `intersect` returns an empty interval (start > stop) when the
            // inputs do not overlap.
            if JulianDate::compare(&candidate.stop, &candidate.start) >= 0 {
                result.push(candidate);
            }
        }
    }
    result
}

/// Mirror of the constant paths of `createGuid`.
fn create_guid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("kml-guid-{:x}-{:x}", nanos, count)
}

// ============================================================================
// KmlDataSource
// ============================================================================

/// A [`DataSource`] which processes Keyhole Markup Language 2.2 (KML).
pub struct KmlDataSource {
    name: Option<String>,
    entity_collection: EntityCollection,
    is_loading: bool,
    is_destroyed: bool,
    show: bool,
    credit: Option<Credit>,
    clamp_to_ground: bool,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
    unsupported_node_event: Event,
    clock: Option<DataSourceClock>,
}

impl KmlDataSource {
    /// Creates a new KML data source.
    pub fn new() -> Self {
        Self {
            name: None,
            entity_collection: EntityCollection::new(),
            is_loading: false,
            is_destroyed: false,
            show: true,
            credit: None,
            clamp_to_ground: false,
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
            unsupported_node_event: Event::new(),
            clock: None,
        }
    }

    /// Creates a new instance loaded with the provided KML text (mirror of
    /// the static `KmlDataSource.load`).
    pub fn load(kml: &str, options: Option<&KmlLoadOptions>) -> Result<KmlDataSource, String> {
        let mut data_source = KmlDataSource::new();
        data_source.load_value(kml, options)?;
        Ok(data_source)
    }

    /// Loads KML from a text string, replacing any existing data.
    pub fn load_value(
        &mut self,
        kml: &str,
        options: Option<&KmlLoadOptions>,
    ) -> Result<(), String> {
        let options = options.cloned().unwrap_or_default();
        self.clamp_to_ground = options.clamp_to_ground;
        self.credit = options
            .credit
            .as_deref()
            .map(|html| Credit::new(html, false));

        // Mirrors `KmlDataSource.prototype.load`: the previous name is
        // cleared before processing so the changed event only fires when
        // the newly loaded document actually (re)names the data source.
        let old_name = self.name.take();

        self.set_loading(true);
        let result = self.load_kml(kml, options.source_uri.as_deref());
        if let Err(ref error) = result {
            self.set_loading(false);
            self.error_event.raise_event(&());
            return Err(error.clone());
        }

        // Derive the clock from the entity availability (mirror of the
        // load success handler).
        let clock = compute_clock_from_availability(&self.entity_collection);
        let mut changed = false;
        if !clock_equals(&clock, &self.clock) {
            self.clock = clock;
            changed = true;
        }
        if old_name != self.name {
            changed = true;
        }
        if changed {
            self.changed_event.raise_event(&());
        }
        self.set_loading(false);
        Ok(())
    }

    /// Loads KML from a file path, replacing any existing data. When no
    /// `sourceUri` is provided in the options, the path itself is used.
    pub fn load_file(
        &mut self,
        path: &str,
        options: Option<&KmlLoadOptions>,
    ) -> Result<(), String> {
        let mut options = options.cloned().unwrap_or_default();
        if options.source_uri.is_none() {
            options.source_uri = Some(path.replace('\\', "/"));
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {}", path, e))
            .and_then(|contents| {
                if contents.trim_start().starts_with('<') {
                    Ok(contents)
                } else {
                    Err(format!("{} is not a KML file", path))
                }
            });
        let contents = match contents {
            Ok(contents) => contents,
            Err(error) => {
                self.error_event.raise_event(&());
                return Err(error);
            }
        };
        self.load_value(&contents, Some(&options))
    }

    /// Returns the human-readable name of this instance (may be unset).
    pub fn display_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets the name of this data source (raises the changed event when the
    /// name actually changes, mirroring the JS property setter).
    pub fn set_name(&mut self, name: Option<&str>) {
        let new_name = name.map(|n| n.to_string());
        if self.name != new_name {
            self.name = new_name;
            self.changed_event.raise_event(&());
        }
    }

    /// The credit for this data source (mirrors `_credit`).
    pub fn credit(&self) -> Option<&Credit> {
        self.credit.as_ref()
    }

    /// The clock derived from the entity availability of this data source
    /// (mirrors `_clock`).
    pub fn clock(&self) -> Option<&crate::data_source_clock::DataSourceClock> {
        self.clock.as_ref()
    }

    /// Raised when an unsupported node is encountered
    /// (mirrors `_unsupportedNode`).
    pub fn unsupported_node_event(&self) -> &Event {
        &self.unsupported_node_event
    }

    fn set_loading(&mut self, loading: bool) {
        if self.is_loading != loading {
            self.is_loading = loading;
            self.loading_event.raise_event(&());
        }
    }

    /// Core KML processing (mirror of `loadKml`).
    fn load_kml(&mut self, kml: &str, source_uri: Option<&str>) -> Result<(), String> {
        let text = insert_namespaces(kml);
        let text = remove_duplicate_namespaces(&text);
        let root = parse_xml(&text)?;

        self.entity_collection.remove_all();

        // Only set the name from the root document.
        let document = if root.local_name == "Document" {
            Some(&root)
        } else {
            query_first_node(Some(&root), "Document")
        };
        if self.name.is_none() {
            let name = query_string_value(document, "name").or_else(|| {
                source_uri.map(|uri| get_filename_from_uri(Some(uri)))
            });
            self.name = name;
        }

        let style_collection = process_styles(&root, source_uri);

        // Find the root feature node (mirror of the `featureTypes` lookup).
        let mut feature: &XmlElement = &root;
        if root.local_name == "kml" {
            for child in &root.children {
                if is_feature_type(&child.local_name) {
                    feature = child;
                    break;
                }
            }
        }

        let mut processing = ProcessingData {
            parent_entity: None,
            style_collection: &style_collection,
            source_uri,
            clamp_to_ground: self.clamp_to_ground,
        };
        process_feature_node(self, feature, &mut processing);
        Ok(())
    }
}

impl Default for KmlDataSource {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for KmlDataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KmlDataSource")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl DataSource for KmlDataSource {
    fn name(&self) -> &str { self.name.as_deref().unwrap_or("") }
    fn entities(&self) -> &EntityCollection { &self.entity_collection }
    fn is_loading(&self) -> bool { self.is_loading }
    fn is_destroyed(&self) -> bool { self.is_destroyed }
    fn changed_event(&self) -> &Event { &self.changed_event }
    fn error_event(&self) -> &Event { &self.error_event }
    fn loading_event(&self) -> &Event { &self.loading_event }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) {
        self.show = show;
        self.entity_collection.show = show;
    }
    fn destroy(&mut self) {
        self.entity_collection.destroy();
        self.is_destroyed = true;
    }
}

// ============================================================================
// Namespace fixups (mirrors of insertNamespaces/removeDuplicateNamespaces)
// ============================================================================

/// Mirror of `insertNamespaces` (only the `xsi` declaration in CesiumJS).
fn insert_namespaces(text: &str) -> String {
    let key = "xsi";
    let declaration = format!("xmlns:{}=", key);
    let used = text.contains(&format!(" {}:", key)) || text.contains(&format!("<{}:", key));
    if used && !text.contains(&declaration) {
        if let Some(index) = text.find("<kml") {
            let split = index + 4;
            let mut result = text[..split].to_string();
            result.push_str(&format!(
                " {}\"http://www.w3.org/2001/XMLSchema-instance\"",
                declaration
            ));
            result.push_str(&text[split..]);
            return result;
        }
    }
    text.to_string()
}

/// Mirror of `removeDuplicateNamespaces`: drops repeated `xmlns:`
/// declarations inside the root element opening tag.
fn remove_duplicate_namespaces(text: &str) -> String {
    let mut text = text.to_string();
    let mut index = text.find("xmlns:");
    let end_declaration = index.and_then(|i| text[i..].find('>')).map(|d| d + index.unwrap_or(0));

    while let (Some(idx), Some(end)) = (index, end_declaration) {
        if idx >= end {
            break;
        }
        let quote = match text[idx..].find('"') {
            Some(q) => idx + q,
            None => break,
        };
        let namespace = text[idx..quote].to_string();
        let start_index = idx;
        let dup = text[start_index + 1..]
            .find(&namespace)
            .map(|i| i + start_index + 1);
        if let Some(dup) = dup {
            let first_quote = match text[dup..].find('"') {
                Some(q) => dup + q,
                None => break,
            };
            let end_quote = match text[first_quote + 1..].find('"') {
                Some(q) => first_quote + 1 + q,
                None => break,
            };
            let mut result = text[..dup.saturating_sub(1)].to_string();
            result.push_str(&text[end_quote + 1..]);
            text = result;
            index = text[..start_index.saturating_add(1)].rfind("xmlns:");
        } else {
            index = text[start_index + 1..].find("xmlns:").map(|i| i + start_index + 1);
        }
    }
    text
}

// ============================================================================
// Feature / geometry type tables
// ============================================================================

/// Mirrors the `featureTypes` table keys (NetworkLink/GroundOverlay/
/// PhotoOverlay/ScreenOverlay/Tour are routed to the unsupported path in
/// this port, see the module DEVIATION note).
fn is_feature_type(name: &str) -> bool {
    matches!(
        name,
        "Document"
            | "Folder"
            | "Placemark"
            | "NetworkLink"
            | "GroundOverlay"
            | "PhotoOverlay"
            | "ScreenOverlay"
            | "Tour"
    )
}

/// Mirrors the `geometryTypes` table keys.
fn is_geometry_type(name: &str) -> bool {
    matches!(
        name,
        "Point"
            | "LineString"
            | "LinearRing"
            | "Polygon"
            | "Track"
            | "MultiTrack"
            | "MultiGeometry"
            | "Model"
    )
}

// ============================================================================
// Default graphics (mirrors of createDefaultBillboard/Polygon/Label)
// ============================================================================

/// Mirror of `createDefaultBillboard`.
fn create_default_billboard() -> BillboardGraphics {
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
    billboard
}

/// Mirror of `createDefaultPolygon`.
fn create_default_polygon() -> PolygonGraphics {
    let mut polygon = PolygonGraphics::new();
    polygon.outline = true;
    polygon.outline_color = Color::WHITE;
    polygon
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

// ============================================================================
// Style processing (mirrors of resolveHref/applyStyle/processStyles/etc.)
// ============================================================================

/// Mirror of `resolveHref` (relative resolution only; KMZ uriResolver and
/// network fetching are not available in this port).
fn resolve_href(href: &str, source_uri: Option<&str>) -> String {
    let href = href.replace('\\', "/");
    if href.starts_with("http://") || href.starts_with("https://") {
        return append_trailing_slash(&href);
    }
    if href.starts_with("root://") || href.starts_with("data:") {
        return href;
    }
    if let Some(base) = source_uri {
        let base = base.replace('\\', "/");
        if href.starts_with('/') {
            return href;
        }
        if let Some(index) = base.rfind('/') {
            // A '/' belonging to the scheme ("http://host") is not a
            // directory separator: append instead of splitting there.
            let scheme_end = base.find("://").map(|i| i + 3).unwrap_or(0);
            if index >= scheme_end {
                return format!("{}/{}", &base[..index], href);
            }
            return format!("{}/{}", base, href);
        }
    }
    append_trailing_slash(&href)
}

/// Mirror of the `Resource` url normalization applied to billboard images
/// in CesiumJS: an http(s) url without a path gains a trailing slash.
fn append_trailing_slash(url: &str) -> String {
    let Some(scheme_end) = url.find("://").map(|i| i + 3) else {
        return url.to_string();
    };
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return url.to_string();
    }
    if url[scheme_end..].contains('/') {
        return url.to_string();
    }
    format!("{}/", url)
}

/// Mirror of the constant path of `getIconHref` (palette rewrite + relative
/// resolution; refresh parameters are not supported).
fn get_icon_href(icon_node: Option<&XmlElement>, source_uri: Option<&str>) -> Option<String> {
    let icon_node = icon_node?;
    let mut href = query_string_value(Some(icon_node), "href")?;
    if href.is_empty() {
        return None;
    }
    if href.starts_with("root://icons/palette-") {
        let palette = href.chars().nth(21).unwrap_or('0');
        let x = query_numeric_value(Some(icon_node), "x").unwrap_or(0.0);
        let y = query_numeric_value(Some(icon_node), "y").unwrap_or(0.0);
        let x = (x / 32.0).min(7.0) as i64;
        let y = 7 - ((y / 32.0).min(7.0) as i64);
        let icon_num = 8 * y + x;
        href = format!(
            "https://maps.google.com/mapfiles/kml/pal{}/icon{}.png",
            palette, icon_num
        );
        return Some(href);
    }
    Some(resolve_href(&href, source_uri))
}

/// Mirror of `processBillboardIcon` (warnings omitted).
fn process_billboard_icon(node: &XmlElement, target: &mut Entity, source_uri: Option<&str>) {
    let scale = query_numeric_value(Some(node), "scale");
    let heading = query_numeric_value(Some(node), "heading");
    let color = query_color_value(Some(node), "color");

    let icon_node = query_first_node(Some(node), "Icon");
    let icon = get_icon_href(icon_node, source_uri);

    let x = icon_node.and_then(|n| query_numeric_value(Some(n), "x"));
    let y = icon_node.and_then(|n| query_numeric_value(Some(n), "y"));
    let w = icon_node.and_then(|n| query_numeric_value(Some(n), "w"));
    let h = icon_node.and_then(|n| query_numeric_value(Some(n), "h"));

    let hot_spot_node = query_first_node(Some(node), "hotSpot");
    let hot_spot_x = hot_spot_node.and_then(|n| query_string_attribute(Some(n), "x").and_then(|v| v.parse::<f64>().ok()));
    let hot_spot_y = hot_spot_node.and_then(|n| query_string_attribute(Some(n), "y").and_then(|v| v.parse::<f64>().ok()));
    let hot_spot_x_unit = hot_spot_node.and_then(|n| query_string_attribute(Some(n), "xunits"));
    let hot_spot_y_unit = hot_spot_node.and_then(|n| query_string_attribute(Some(n), "yunits"));

    let billboard = target
        .billboard
        .get_or_insert_with(create_default_billboard);

    // If icon tags are present but blank, we do not want to show an icon
    // (the JS assigns `false`, which later resolves to no image).
    if icon.is_some() {
        billboard.image = icon;
    }
    if let Some(scale) = scale {
        billboard.scale = scale;
    }
    if color.is_some() {
        billboard.color = color;
    }

    if x.is_some() || y.is_some() || w.is_some() || h.is_some() {
        billboard.image_sub_region =
            Some((x.unwrap_or(0.0), y.unwrap_or(0.0), w.unwrap_or(0.0), h.unwrap_or(0.0)));
    }

    // GE treats a heading of zero as no heading; you can still point north
    // using a 360 degree angle (or any multiple of 360).
    if let Some(heading) = heading {
        if heading != 0.0 {
            billboard.rotation = (-heading).to_radians();
            billboard.aligned_axis = Some(Cartesian3::UNIT_Z);
        }
    }

    // HotSpot is the KML equivalent of pixel offset. The hotspot origin is
    // the lower left, but we leave our billboard origin at the center and
    // simply modify the pixel offset to take this into account.
    let scale = scale.unwrap_or(1.0);

    let mut x_offset: Option<f64> = None;
    if let Some(hot_spot_x) = hot_spot_x {
        let mut value = match hot_spot_x_unit.as_deref() {
            Some("pixels") => -hot_spot_x * scale,
            Some("insetPixels") => (hot_spot_x - BILLBOARD_SIZE) * scale,
            Some("fraction") => -hot_spot_x * BILLBOARD_SIZE * scale,
            _ => 0.0,
        };
        value += BILLBOARD_SIZE * 0.5 * scale;
        x_offset = Some(value);
    }

    let mut y_offset: Option<f64> = None;
    if let Some(hot_spot_y) = hot_spot_y {
        let mut value = match hot_spot_y_unit.as_deref() {
            Some("pixels") => hot_spot_y * scale,
            Some("insetPixels") => (-hot_spot_y + BILLBOARD_SIZE) * scale,
            Some("fraction") => hot_spot_y * BILLBOARD_SIZE * scale,
            _ => 0.0,
        };
        value -= BILLBOARD_SIZE * 0.5 * scale;
        y_offset = Some(value);
    }

    if x_offset.is_some() || y_offset.is_some() {
        billboard.pixel_offset = Some((x_offset.unwrap_or(0.0), y_offset.unwrap_or(0.0)));
    }
}

/// Mirror of `applyStyle` (BalloonStyle/ListStyle warnings omitted;
/// gx LineStyle extensions are not supported).
fn apply_style(style_node: &XmlElement, target: &mut Entity, source_uri: Option<&str>) {
    for node in &style_node.children {
        match node.local_name.as_str() {
            "IconStyle" => process_billboard_icon(node, target, source_uri),
            "LabelStyle" => {
                let name = target.name.clone();
                let label = target.label.get_or_insert_with(create_default_label);
                if let Some(scale) = query_numeric_value(Some(node), "scale") {
                    label.scale = scale;
                }
                if let Some(color) = query_color_value(Some(node), "color") {
                    label.fill_color = color;
                }
                label.text = name;
            }
            "LineStyle" => {
                let polyline = target.polyline.get_or_insert_with(PolylineGraphics::new);
                if let Some(width) = query_numeric_value(Some(node), "width") {
                    polyline.width = width;
                }
                if let Some(color) = query_color_value(Some(node), "color") {
                    polyline.material_color = color;
                }
            }
            "PolyStyle" => {
                let polygon = target.polygon.get_or_insert_with(create_default_polygon);
                if let Some(color) = query_color_value(Some(node), "color") {
                    polygon.material_color = color;
                }
                if let Some(fill) = query_boolean_value(Some(node), "fill") {
                    polygon.fill = fill;
                }
                if let Some(outline) = query_boolean_value(Some(node), "outline") {
                    polygon.outline = outline;
                }
            }
            _ => {}
        }
    }
}

/// Mirror of `processStyles` (local styles only; external style files are
/// not fetched in this port). Stores `"#id" -> style entity`.
fn process_styles(root: &XmlElement, source_uri: Option<&str>) -> HashMap<String, Entity> {
    let mut style_collection: HashMap<String, Entity> = HashMap::new();

    for node in query_nodes_recursive(root, "Style") {
        if let Some(id) = query_string_attribute(Some(node), "id") {
            let id = format!("#{}", id);
            if !style_collection.contains_key(&id) {
                let mut style_entity = Entity::new(&id);
                apply_style(node, &mut style_entity, source_uri);
                style_collection.insert(id, style_entity);
            }
        }
    }

    for style_map in query_nodes_recursive(root, "StyleMap") {
        let Some(raw_id) = query_string_attribute(Some(style_map), "id") else {
            continue;
        };
        for pair in query_child_nodes(Some(style_map), "Pair") {
            let key = query_string_value(Some(pair), "key");
            if key.as_deref() != Some("normal") {
                continue;
            }
            let id = format!("#{}", raw_id);
            if style_collection.contains_key(&id) {
                continue;
            }
            let mut style_entity = Entity::new(&id);
            if let Some(mut style_url) = query_string_value(Some(pair), "styleUrl") {
                if !style_url.starts_with('#') {
                    style_url = format!("#{}", style_url);
                }
                if let Some(base) = style_collection.get(&style_url) {
                    merge_style_entity(&mut style_entity, base);
                }
            } else if let Some(style) = query_first_node(Some(pair), "Style") {
                apply_style(style, &mut style_entity, source_uri);
            }
            style_collection.insert(id, style_entity);
        }
    }

    style_collection
}

// ============================================================================
// Style merging (fill-undefined semantics of `Entity.prototype.merge`)
// ============================================================================

/// Fills `dst` with values from `src` for billboard properties that were
/// never set. `Option` properties are fill-None; scalar properties treat
/// their default value as "not provided" (the value model has no
/// undefined scalars).
fn merge_billboard(dst: &mut BillboardGraphics, src: &BillboardGraphics) {
    if dst.image.is_none() {
        dst.image = src.image.clone();
    }
    if dst.scale == 1.0 {
        dst.scale = src.scale;
    }
    if dst.color.is_none() {
        dst.color = src.color;
    }
    if dst.rotation == 0.0 {
        dst.rotation = src.rotation;
    }
    if dst.horizontal_origin == 0 {
        dst.horizontal_origin = src.horizontal_origin;
    }
    if dst.vertical_origin == 0 {
        dst.vertical_origin = src.vertical_origin;
    }
    if dst.pixel_offset.is_none() {
        dst.pixel_offset = src.pixel_offset;
    }
    if dst.eye_offset.is_none() {
        dst.eye_offset = src.eye_offset;
    }
    if dst.aligned_axis.is_none() {
        dst.aligned_axis = src.aligned_axis;
    }
    if dst.size_in_meters.is_none() {
        dst.size_in_meters = src.size_in_meters;
    }
    if dst.width.is_none() {
        dst.width = src.width;
    }
    if dst.height.is_none() {
        dst.height = src.height;
    }
    if dst.scale_by_distance.is_none() {
        dst.scale_by_distance = src.scale_by_distance;
    }
    if dst.translucency_by_distance.is_none() {
        dst.translucency_by_distance = src.translucency_by_distance;
    }
    if dst.pixel_offset_scale_by_distance.is_none() {
        dst.pixel_offset_scale_by_distance = src.pixel_offset_scale_by_distance;
    }
    if dst.image_sub_region.is_none() {
        dst.image_sub_region = src.image_sub_region;
    }
    if dst.height_reference == 0 {
        dst.height_reference = src.height_reference;
    }
}

/// Fill-undefined merge for [`LabelGraphics`] (see [`merge_billboard`]).
fn merge_label(dst: &mut LabelGraphics, src: &LabelGraphics) {
    if dst.text.is_none() {
        dst.text = src.text.clone();
    }
    if dst.font.is_none() {
        dst.font = src.font.clone();
    }
    if dst.fill_color == Color::WHITE {
        dst.fill_color = src.fill_color;
    }
    if dst.outline_color == Color::BLACK {
        dst.outline_color = src.outline_color;
    }
    if dst.outline_width == 1.0 {
        dst.outline_width = src.outline_width;
    }
    if dst.scale == 1.0 {
        dst.scale = src.scale;
    }
    if dst.style == 0 {
        dst.style = src.style;
    }
    if dst.horizontal_origin == 0 {
        dst.horizontal_origin = src.horizontal_origin;
    }
    if dst.vertical_origin == 0 {
        dst.vertical_origin = src.vertical_origin;
    }
    if dst.eye_offset.is_none() {
        dst.eye_offset = src.eye_offset;
    }
    if dst.pixel_offset.is_none() {
        dst.pixel_offset = src.pixel_offset;
    }
    if dst.translucency_by_distance.is_none() {
        dst.translucency_by_distance = src.translucency_by_distance;
    }
    if dst.pixel_offset_scale_by_distance.is_none() {
        dst.pixel_offset_scale_by_distance = src.pixel_offset_scale_by_distance;
    }
}

/// Fill-undefined merge for [`PolylineGraphics`] (see [`merge_billboard`]).
fn merge_polyline(dst: &mut PolylineGraphics, src: &PolylineGraphics) {
    if dst.positions.is_empty() {
        dst.positions = src.positions.clone();
    }
    if dst.width == 1.0 {
        dst.width = src.width;
    }
    if dst.material_color == Color::WHITE {
        dst.material_color = src.material_color;
    }
    if !dst.clamp_to_ground {
        dst.clamp_to_ground = src.clamp_to_ground;
    }
    if !dst.loop_ {
        dst.loop_ = src.loop_;
    }
    if dst.arc_type == ArcType::Geodesic {
        dst.arc_type = src.arc_type;
    }
}

/// Fill-undefined merge for [`PolygonGraphics`] (see [`merge_billboard`]).
fn merge_polygon(dst: &mut PolygonGraphics, src: &PolygonGraphics) {
    if dst.hierarchy.is_empty() {
        dst.hierarchy = src.hierarchy.clone();
    }
    if dst.holes.is_empty() {
        dst.holes = src.holes.clone();
    }
    if dst.height.is_none() {
        dst.height = src.height;
    }
    if dst.extruded_height.is_none() {
        dst.extruded_height = src.extruded_height;
    }
    if dst.material_color == Color::WHITE {
        dst.material_color = src.material_color;
    }
    if !dst.outline {
        dst.outline = src.outline;
    }
    if dst.outline_color == Color::BLACK {
        dst.outline_color = src.outline_color;
    }
    if !dst.extrude {
        dst.extrude = src.extrude;
    }
    if dst.fill {
        dst.fill = src.fill;
    }
    if dst.outline_width == 1.0 {
        dst.outline_width = src.outline_width;
    }
    if dst.per_position_height.is_none() {
        dst.per_position_height = src.per_position_height;
    }
    if dst.arc_type == ArcType::Geodesic {
        dst.arc_type = src.arc_type;
    }
}

/// Style-local mirror of the fill-undefined semantics of
/// `Entity.prototype.merge`: assigns each graphics object of `source` to
/// `target` only when `target` has not already defined one of its own.
/// (`Entity::merge` itself uses replace semantics for the CZML port.)
fn merge_style_entity(target: &mut Entity, source: &Entity) {
    match (&mut target.billboard, &source.billboard) {
        (None, Some(src)) => target.billboard = Some(src.clone()),
        (Some(dst), Some(src)) => merge_billboard(dst, src),
        _ => {}
    }
    match (&mut target.label, &source.label) {
        (None, Some(src)) => target.label = Some(src.clone()),
        (Some(dst), Some(src)) => merge_label(dst, src),
        _ => {}
    }
    match (&mut target.polyline, &source.polyline) {
        (None, Some(src)) => target.polyline = Some(src.clone()),
        (Some(dst), Some(src)) => merge_polyline(dst, src),
        _ => {}
    }
    match (&mut target.polygon, &source.polygon) {
        (None, Some(src)) => target.polygon = Some(src.clone()),
        (Some(dst), Some(src)) => merge_polygon(dst, src),
        _ => {}
    }
}

/// Mirror of `computeFinalStyle`: the last inline Style/StyleMap wins, then
/// the first `styleUrl` reference is merged on top.
fn compute_final_style(
    placemark: &XmlElement,
    style_collection: &HashMap<String, Entity>,
    source_uri: Option<&str>,
) -> Entity {
    let mut result = Entity::new("");

    // Google Earth seems to always use the last inline Style/StyleMap only.
    let mut style_index: Option<usize> = None;
    for (index, child) in placemark.children.iter().enumerate() {
        if child.local_name == "Style" || child.local_name == "StyleMap" {
            style_index = Some(index);
        }
    }

    if let Some(style_index) = style_index {
        let inline_style_node = &placemark.children[style_index];
        if inline_style_node.local_name == "Style" {
            apply_style(inline_style_node, &mut result, source_uri);
        } else {
            // StyleMap
            for pair in query_child_nodes(Some(inline_style_node), "Pair") {
                let key = query_string_value(Some(pair), "key");
                if key.as_deref() != Some("normal") {
                    continue;
                }
                if let Some(style_url) = query_string_value(Some(pair), "styleUrl") {
                    let style_entity = style_collection
                        .get(&style_url)
                        .or_else(|| style_collection.get(&format!("#{}", style_url)));
                    if let Some(style_entity) = style_entity {
                        merge_style_entity(&mut result, style_entity);
                    }
                } else if let Some(node) = query_first_node(Some(pair), "Style") {
                    apply_style(node, &mut result, source_uri);
                }
            }
        }
    }

    // Google Earth seems to always use the first external style only.
    if let Some(external_style) = query_string_value(Some(placemark), "styleUrl") {
        // DEVIATION: external style documents (network fetch) are not
        // resolved; only local `#id` references apply.
        let id = external_style.trim();
        let style_entity = style_collection
            .get(id)
            .or_else(|| style_collection.get(&format!("#{}", id)))
            .or_else(|| {
                id.rsplit_once('#')
                    .and_then(|(_, fragment)| style_collection.get(&format!("#{}", fragment)))
            });
        if let Some(style_entity) = style_entity {
            merge_style_entity(&mut result, style_entity);
        }
    }

    result
}

// ============================================================================
// Feature processing
// ============================================================================

/// Mirrors `processingData` (the entity collection lives on the data
/// source itself in this port).
struct ProcessingData<'a> {
    parent_entity: Option<String>,
    style_collection: &'a HashMap<String, Entity>,
    source_uri: Option<&'a str>,
    clamp_to_ground: bool,
}

/// Mirror of `createEntity` (id attribute or generated guid; duplicate ids
/// get a fresh guid, as Google Earth tolerates them).
fn create_entity_id(
    node: &XmlElement,
    entity_collection: &EntityCollection,
    context: Option<&str>,
) -> String {
    let mut id = match query_string_attribute(Some(node), "id") {
        Some(id) if !id.is_empty() => id,
        _ => create_guid(),
    };
    if let Some(context) = context {
        id = format!("{}{}", context, id);
    }
    if entity_collection.contains_entity(&id) {
        id = create_guid();
        if let Some(context) = context {
            id = format!("{}{}", context, id);
        }
    }
    id
}

/// Mirror of `processExtendedData`: `<Data name="x"><value>v</value>`
/// entries stored under the `kml.extendedData` metadata.
fn process_extended_data(node: &XmlElement, extended_data: &mut serde_json::Map<String, serde_json::Value>) {
    let Some(extended_data_node) = query_first_node(Some(node), "ExtendedData") else {
        return;
    };
    // DEVIATION: SchemaData and xmlns:prefix forms are unsupported.
    for data_node in query_child_nodes(Some(extended_data_node), "Data") {
        if let Some(name) = query_string_attribute(Some(data_node), "name") {
            let mut entry = serde_json::Map::new();
            if let Some(display_name) = query_string_value(Some(data_node), "displayName") {
                entry.insert("displayName".to_string(), serde_json::Value::String(display_name));
            }
            if let Some(value) = query_string_value(Some(data_node), "value") {
                entry.insert("value".to_string(), serde_json::Value::String(value));
            }
            extended_data.insert(name, serde_json::Value::Object(entry));
        }
    }
}

/// Mirror of `processFeature`: creates the entity, computes its final
/// style, and fills in the feature-level metadata. Returns the entity id
/// and the computed style entity.
fn process_feature(
    data_source: &mut KmlDataSource,
    feature_node: &XmlElement,
    processing: &ProcessingData,
) -> (String, Entity) {
    let entity_id = create_entity_id(feature_node, &data_source.entity_collection, None);
    let style_entity = compute_final_style(
        feature_node,
        processing.style_collection,
        processing.source_uri,
    );

    let name = query_string_value(Some(feature_node), "name");

    let mut availability = process_time_span(feature_node);
    if availability.is_none() {
        availability = process_time_stamp(feature_node);
    }

    // Snapshot the parent state before borrowing the collection mutably.
    let parent_info = processing.parent_entity.as_ref().and_then(|parent_id| {
        data_source
            .entity_collection
            .get_by_id(parent_id)
            .map(|parent| (parent.show, parent.availability.clone()))
    });

    data_source.entity_collection.add(Entity::new(&entity_id));
    let entity = data_source
        .entity_collection
        .get_by_id_mut(&entity_id)
        .expect("entity was just added");

    entity.name = name;
    entity.parent_id = processing.parent_entity.clone();
    entity.availability = availability.unwrap_or_default();

    // Mirrors `mergeAvailabilityWithParent`.
    if let Some((_, ref parent_availability)) = parent_info {
        if !parent_availability.is_empty() {
            if entity.availability.is_empty() {
                entity.availability = parent_availability.clone();
            } else {
                entity.availability =
                    intersect_availability(&entity.availability, parent_availability);
            }
        }
    }

    // Per KML spec: "A Feature is visible only if it and all its ancestors
    // are visible." (ancestors are processed first, so the parent's `show`
    // already reflects its own ancestry)
    let ancestry_visible = parent_info.as_ref().map(|(show, _)| *show).unwrap_or(true);
    let visibility = query_boolean_value(Some(feature_node), "visibility");
    entity.show = ancestry_visible && visibility.unwrap_or(true);

    // KML feature metadata (mirrors `KmlFeatureData`, stored as a `kml`
    // property in this port).
    let mut kml_data = serde_json::Map::new();
    if let Some(address) = query_string_value(Some(feature_node), "address") {
        kml_data.insert("address".to_string(), serde_json::Value::String(address));
    }
    if let Some(phone_number) = query_string_value(Some(feature_node), "phoneNumber") {
        kml_data.insert("phoneNumber".to_string(), serde_json::Value::String(phone_number));
    }
    if let Some(snippet) = query_string_value(Some(feature_node), "Snippet") {
        kml_data.insert("snippet".to_string(), serde_json::Value::String(snippet));
    }
    let mut extended_data = serde_json::Map::new();
    process_extended_data(feature_node, &mut extended_data);
    if !extended_data.is_empty() {
        kml_data.insert(
            "extendedData".to_string(),
            serde_json::Value::Object(extended_data),
        );
    }
    if !kml_data.is_empty() {
        entity.properties.set(
            "kml",
            PropertyResult::Json(serde_json::Value::Object(kml_data)),
        );
    }

    // DEVIATION: the description is stored verbatim; the BalloonStyle/HTML
    // link rewriting of the browser implementation is not performed.
    entity.description = query_string_value(Some(feature_node), "description");

    (entity_id, style_entity)
}

// ============================================================================
// Geometry processing
// ============================================================================

/// Mirror of `processPositionGraphics` (pinBuilder image omitted; the
/// label height reference has no counterpart in this value model).
fn process_position_graphics(
    clamp_to_ground: bool,
    entity: &mut Entity,
    style_entity: &Entity,
    height_reference: Option<i32>,
) {
    let name = entity.name.clone();
    let label = entity.label.get_or_insert_with(|| {
        style_entity
            .label
            .clone()
            .unwrap_or_else(create_default_label)
    });
    label.text = name;

    let billboard = entity.billboard.get_or_insert_with(|| {
        style_entity
            .billboard
            .clone()
            .unwrap_or_else(create_default_billboard)
    });

    // DEVIATION: no pinBuilder; billboard.image stays unset unless a style
    // provided one.

    if billboard.scale != 0.0 {
        let scale = billboard.scale;
        let label = entity
            .label
            .as_mut()
            .expect("label created above");
        label.pixel_offset = Some((scale * 16.0 + 1.0, 0.0));
    } else {
        // Minor tweaks to better match Google Earth.
        let label = entity
            .label
            .as_mut()
            .expect("label created above");
        label.pixel_offset = None;
        label.horizontal_origin = 0;
    }

    if let Some(height_reference) = height_reference {
        if clamp_to_ground {
            entity
                .billboard
                .as_mut()
                .expect("billboard created above")
                .height_reference = height_reference;
        }
    }
}

/// Mirrors `createPositionPropertyArrayFromAltitudeMode`: clamped positions
/// are projected onto the geodetic surface.
fn clamp_positions_to_ground(positions: &mut [Cartesian3]) {
    let ellipsoid = cesium_core::ellipsoid::Ellipsoid::WGS84;
    for position in positions.iter_mut() {
        let mut scaled = Cartesian3::default();
        if ellipsoid.scale_to_geodetic_surface(position, &mut scaled) {
            *position = scaled;
        }
    }
}

/// Dispatches a geometry node to its processor (mirrors the
/// `geometryTypes[node.localName]` lookup in `processPlacemark`).
fn process_geometry(
    data_source: &mut KmlDataSource,
    geometry_node: &XmlElement,
    entity: &mut Entity,
    style_entity: &Entity,
    context: Option<&str>,
) -> bool {
    match geometry_node.local_name.as_str() {
        "Point" => process_point(data_source.clamp_to_ground, geometry_node, entity, style_entity),
        "LineString" | "LinearRing" => process_line_string_or_linear_ring(
            data_source.clamp_to_ground,
            geometry_node,
            entity,
            style_entity,
        ),
        "Polygon" => {
            process_polygon(data_source.clamp_to_ground, geometry_node, entity, style_entity)
        }
        "MultiGeometry" => process_multi_geometry(
            data_source,
            geometry_node,
            entity,
            style_entity,
            context,
        ),
        // DEVIATION: Track/MultiTrack/Model are not materialized.
        _ => false,
    }
}

/// Mirror of `processPoint` (the extrude drop line is not produced).
fn process_point(
    clamp_to_ground: bool,
    geometry_node: &XmlElement,
    entity: &mut Entity,
    style_entity: &Entity,
) -> bool {
    let coordinates_string = query_string_value(Some(geometry_node), "coordinates");
    let (altitude_mode, gx_altitude_mode) = query_altitude_modes(geometry_node);
    let _extrude = query_boolean_value(Some(geometry_node), "extrude");

    let position = read_coordinate(coordinates_string.as_deref());
    entity.position = Some(position);
    process_position_graphics(
        clamp_to_ground,
        entity,
        style_entity,
        Some(height_reference_from_altitude_mode(
            altitude_mode.as_deref(),
            gx_altitude_mode.as_deref(),
        )),
    );
    // DEVIATION: `createDropLine` (extruded point drop line) is skipped.
    true
}

/// Mirror of `processLineStringOrLinearRing` (wall graphics for extruded
/// lines fall back to a polyline; gx:drawOrder unsupported).
fn process_line_string_or_linear_ring(
    clamp_to_ground: bool,
    geometry_node: &XmlElement,
    entity: &mut Entity,
    style_entity: &Entity,
) -> bool {
    let coordinates_node = query_first_node(Some(geometry_node), "coordinates");
    let (altitude_mode, gx_altitude_mode) = query_altitude_modes(geometry_node);
    let extrude = query_boolean_value(Some(geometry_node), "extrude");
    let tessellate = query_boolean_value(Some(geometry_node), "tessellate");
    let can_extrude = is_extrudable(altitude_mode.as_deref(), gx_altitude_mode.as_deref());

    let mut coordinates = read_coordinates(coordinates_node).unwrap_or_default();
    let style_polyline = style_entity.polyline.clone();

    if can_extrude && extrude.unwrap_or(false) {
        // DEVIATION: WallGraphics are not materialized; the wall is
        // approximated with a styled polyline.
        let mut polyline = PolylineGraphics::new();
        polyline.positions = coordinates;
        if let Some(ref style) = style_polyline {
            polyline.material_color = style.material_color;
            polyline.width = style.width;
        }
        entity.polyline = Some(polyline);
    } else if clamp_to_ground && !can_extrude && tessellate.unwrap_or(false) {
        let mut polyline = PolylineGraphics::new();
        polyline.clamp_to_ground = true;
        polyline.positions = coordinates;
        if let Some(ref style) = style_polyline {
            polyline.material_color = style.material_color;
            polyline.width = style.width;
        }
        entity.polyline = Some(polyline);
    } else {
        let mut polyline = style_polyline.unwrap_or_else(PolylineGraphics::new);
        if altitude_mode.is_none() && gx_altitude_mode.is_none() {
            // Clamp to ground is the default: scale positions to the
            // geodetic surface.
            clamp_positions_to_ground(&mut coordinates);
        }
        polyline.positions = coordinates;
        if !tessellate.unwrap_or(false) || can_extrude {
            polyline.arc_type = ArcType::None;
        }
        entity.polyline = Some(polyline);
    }

    true
}

/// Mirror of `processPolygon`.
fn process_polygon(
    clamp_to_ground: bool,
    geometry_node: &XmlElement,
    entity: &mut Entity,
    style_entity: &Entity,
) -> bool {
    let outer_boundary_is_node = query_first_node(Some(geometry_node), "outerBoundaryIs");
    let linear_ring_node = query_first_node(outer_boundary_is_node, "LinearRing");
    let coordinates_node = query_first_node(linear_ring_node, "coordinates");
    let coordinates = read_coordinates(coordinates_node);

    let extrude = query_boolean_value(Some(geometry_node), "extrude");
    let (altitude_mode, gx_altitude_mode) = query_altitude_modes(geometry_node);
    let can_extrude = is_extrudable(altitude_mode.as_deref(), gx_altitude_mode.as_deref());

    let mut polygon = style_entity
        .polygon
        .clone()
        .unwrap_or_else(create_default_polygon);

    if let Some(ref polyline) = style_entity.polyline {
        polygon.outline_color = polyline.material_color;
        polygon.outline_width = polyline.width;
    }

    if can_extrude {
        polygon.per_position_height = Some(true);
        polygon.extruded_height = if extrude.unwrap_or(false) { Some(0.0) } else { None };
    } else if !clamp_to_ground {
        polygon.height = Some(0.0);
    }

    if let Some(coordinates) = coordinates {
        polygon.hierarchy = coordinates;
        for inner_boundary_is_node in query_child_nodes(Some(geometry_node), "innerBoundaryIs") {
            for linear_ring_node in query_child_nodes(Some(inner_boundary_is_node), "LinearRing") {
                let coordinates_node = query_first_node(Some(linear_ring_node), "coordinates");
                if let Some(ring) = read_coordinates(coordinates_node) {
                    polygon.holes.push(ring);
                }
            }
        }
    }

    entity.polygon = Some(polygon);
    true
}

/// Mirror of `processMultiGeometry`: each child geometry gets its own
/// entity parented to the placemark entity.
fn process_multi_geometry(
    data_source: &mut KmlDataSource,
    geometry_node: &XmlElement,
    entity: &mut Entity,
    style_entity: &Entity,
    context: Option<&str>,
) -> bool {
    let mut has_geometry = false;
    for child_node in &geometry_node.children {
        if !is_geometry_type(&child_node.local_name) {
            continue;
        }
        let child_id = create_entity_id(child_node, &data_source.entity_collection, context);
        let mut child_entity = Entity::new(&child_id);
        child_entity.parent_id = Some(entity.id.clone());
        child_entity.name = entity.name.clone();
        child_entity.availability = entity.availability.clone();
        child_entity.description = entity.description.clone();
        if let Some(kml) = entity.properties.get("kml") {
            child_entity.properties.set("kml", kml.clone());
        }
        if process_geometry(
            data_source,
            child_node,
            &mut child_entity,
            style_entity,
            context,
        ) {
            has_geometry = true;
        }
        data_source.entity_collection.add(child_entity);
    }
    has_geometry
}

// ============================================================================
// Feature node dispatch
// ============================================================================

/// Mirror of `processDocument`: processes each child feature node.
fn process_document(
    data_source: &mut KmlDataSource,
    node: &XmlElement,
    processing: &ProcessingData,
) {
    let children: Vec<&XmlElement> = node.children.iter().collect();
    for child in children {
        process_feature_node(data_source, child, processing);
    }
}

/// Mirror of `processFolder`: creates the folder entity then recurses with
/// it as the parent.
fn process_folder(
    data_source: &mut KmlDataSource,
    node: &XmlElement,
    processing: &ProcessingData,
) {
    let (entity_id, _style_entity) = process_feature(data_source, node, processing);
    let new_processing = ProcessingData {
        parent_entity: Some(entity_id),
        style_collection: processing.style_collection,
        source_uri: processing.source_uri,
        clamp_to_ground: processing.clamp_to_ground,
    };
    process_document(data_source, node, &new_processing);
}

/// Mirror of `processPlacemark`.
fn process_placemark(
    data_source: &mut KmlDataSource,
    placemark: &XmlElement,
    processing: &ProcessingData,
) {
    let (entity_id, style_entity) = process_feature(data_source, placemark, processing);

    let geometry_node = placemark
        .children
        .iter()
        .find(|child| is_geometry_type(&child.local_name));

    match geometry_node {
        Some(geometry_node) => {
            // Take the entity out of the collection so the geometry
            // processors (which may add child entities) can borrow it.
            let mut entity = data_source
                .entity_collection
                .remove(&entity_id)
                .expect("entity was just added");
            let context = entity.id.clone();
            process_geometry(data_source, geometry_node, &mut entity, &style_entity, Some(&context));
            data_source.entity_collection.add(entity);
        }
        None => {
            let clamp_to_ground = data_source.clamp_to_ground;
            let entity = data_source
                .entity_collection
                .get_by_id_mut(&entity_id)
                .expect("entity was just added");
            merge_style_entity(entity, &style_entity);
            process_position_graphics(clamp_to_ground, entity, &style_entity, None);
        }
    }
}

/// Mirror of `processUnsupportedFeature`.
fn process_unsupported_feature(data_source: &KmlDataSource, node: &XmlElement) {
    data_source.unsupported_node_event.raise_event(&());
    // DEVIATION: `oneTimeWarning` console output is omitted.
    let _ = node;
}

/// Mirror of `processFeatureNode` (featureTypes dispatch; NetworkLink,
/// overlays and Tour are routed to the unsupported path in this port).
fn process_feature_node(
    data_source: &mut KmlDataSource,
    node: &XmlElement,
    processing: &ProcessingData,
) {
    match node.local_name.as_str() {
        "Document" => process_document(data_source, node, processing),
        "Folder" => process_folder(data_source, node, processing),
        "Placemark" => process_placemark(data_source, node, processing),
        _ => process_unsupported_feature(data_source, node),
    }
}
