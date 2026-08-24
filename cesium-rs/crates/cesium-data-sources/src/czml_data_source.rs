//! Ported from `packages/engine/Source/DataSources/CzmlDataSource.js`.
//!
//! A data source that loads CZML (Cesium Language) files.
//! CZML is a JSON-based format for describing time-dynamic 3D scenes.
//!
//! DEVIATION (simplified value model): CesiumJS stores every CZML property as
//! a time-dynamic `Property` object (`ConstantProperty`, `SampledProperty`,
//! `TimeIntervalCollectionProperty`, `CompositeProperty`, ...). This port
//! materializes the *constant* subset of CZML directly onto the entity and
//! graphics structs; sampled (`epoch`/packed time arrays), `interval`-based
//! and `reference` properties are intentionally skipped.

use serde_json::Value;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::color::Color;
use cesium_core::credit::Credit;
use cesium_core::event::Event;
use cesium_core::get_filename_from_uri::get_filename_from_uri;
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_core::quaternion::Quaternion;
use cesium_core::time_interval::TimeInterval;

use crate::billboard_graphics::BillboardGraphics;
use crate::data_source::DataSource;
use crate::data_source_clock::DataSourceClock;
use crate::entity::Entity;
use crate::entity_collection::EntityCollection;
use crate::label_graphics::LabelGraphics;
use crate::point_graphics::PointGraphics;
use crate::polyline_graphics::PolylineGraphics;
use crate::property::PropertyResult;

/// Error raised when the first CZML packet is not the document packet.
pub const FIRST_PACKET_ERROR: &str =
    "The first CZML packet is required to be the document object.";
/// Error raised when the CZML version is unsupported.
pub const VERSION_ERROR: &str = "Cesium only supports CZML version 1.";
/// Error raised when the document packet carries no/invalid version info.
pub const VERSION_INVALID_ERROR: &str = "CZML version information invalid.  It is expected to be a property on the document object in the <Major>.<Minor> version format.";

/// Initialization options for the load/process methods (mirror of
/// `CzmlDataSource.LoadOptions`).
#[derive(Debug, Clone, Default)]
pub struct CzmlLoadOptions {
    /// Overrides the uri to use for resolving relative links.
    pub source_uri: Option<String>,
    /// A credit for the data source, which is displayed on the canvas.
    pub credit: Option<String>,
}

/// The aggregated document packet state (mirror of the internal
/// `DocumentPacket`).
#[derive(Debug, Clone, Default)]
struct DocumentPacket {
    name: Option<String>,
    clock: Option<DocumentClockPacket>,
}

/// The raw clock packet fields recorded from the document packet.
#[derive(Debug, Clone, Default)]
struct DocumentClockPacket {
    interval: Option<String>,
    current_time: Option<String>,
    range: Option<String>,
    step: Option<String>,
    multiplier: Option<f64>,
}

/// A [`DataSource`] which processes CZML.
pub struct CzmlDataSource {
    name: Option<String>,
    changed_event: Event,
    error_event: Event,
    loading_event: Event,
    is_loading: bool,
    is_destroyed: bool,
    show: bool,
    clock: Option<DataSourceClock>,
    document_packet: DocumentPacket,
    version: Option<String>,
    entity_collection: EntityCollection,
    credit: Option<Credit>,
}

impl CzmlDataSource {
    /// Creates a new CZML data source.
    ///
    /// `name` is an optional name for the data source. This value will be
    /// overwritten if a loaded document contains a name.
    pub fn new() -> Self {
        Self::with_name(None)
    }

    /// Creates a new CZML data source with the provided name.
    pub fn with_name(name: Option<&str>) -> Self {
        Self {
            name: name.map(|n| n.to_string()),
            changed_event: Event::new(),
            error_event: Event::new(),
            loading_event: Event::new(),
            is_loading: false,
            is_destroyed: false,
            show: true,
            clock: None,
            document_packet: DocumentPacket::default(),
            version: None,
            entity_collection: EntityCollection::new(),
            credit: None,
        }
    }

    /// Creates a new instance loaded with the provided CZML data (mirror of
    /// the static `CzmlDataSource.load`).
    pub fn load(czml: &Value, options: Option<&CzmlLoadOptions>) -> Result<CzmlDataSource, String> {
        let mut data_source = CzmlDataSource::new();
        data_source.load_value(czml, options)?;
        Ok(data_source)
    }

    /// Returns the human-readable name of this instance (may be unset).
    pub fn display_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets the name of this data source.
    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
    }

    /// Returns the clock settings defined by the loaded CZML, if any. If no
    /// clock is explicitly defined in the CZML, the combined availability of
    /// all objects is returned. If only static data exists, this value is
    /// `None`.
    pub fn clock(&self) -> Option<&DataSourceClock> {
        self.clock.as_ref()
    }

    /// Returns the credit of this data source, if any.
    pub fn credit(&self) -> Option<&Credit> {
        self.credit.as_ref()
    }

    /// Returns the entity collection.
    pub fn entities(&self) -> &EntityCollection {
        &self.entity_collection
    }

    /// Returns the `changed` event.
    pub fn changed_event(&self) -> &Event {
        &self.changed_event
    }

    /// Returns the `error` event.
    pub fn error_event(&self) -> &Event {
        &self.error_event
    }

    /// Returns the `loading` event.
    pub fn loading_event(&self) -> &Event {
        &self.loading_event
    }

    /// Loads the provided CZML value, replacing any existing data (mirror of
    /// `CzmlDataSource.prototype.load`).
    pub fn load_value(
        &mut self,
        czml: &Value,
        options: Option<&CzmlLoadOptions>,
    ) -> Result<(), String> {
        self.load_inner(czml, options, true)
    }

    /// Processes the provided CZML value without clearing any existing data
    /// (mirror of `CzmlDataSource.prototype.process`).
    pub fn process_value(
        &mut self,
        czml: &Value,
        options: Option<&CzmlLoadOptions>,
    ) -> Result<(), String> {
        self.load_inner(czml, options, false)
    }

    /// Loads CZML from a JSON string, replacing any existing data.
    pub fn load_json(
        &mut self,
        json: &str,
        options: Option<&CzmlLoadOptions>,
    ) -> Result<(), String> {
        let value: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.load_value(&value, options)
    }

    /// Processes CZML from a JSON string without clearing existing data.
    pub fn process_json(
        &mut self,
        json: &str,
        options: Option<&CzmlLoadOptions>,
    ) -> Result<(), String> {
        let value: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.process_value(&value, options)
    }

    /// Loads CZML from a file path, replacing any existing data. When no
    /// `sourceUri` is provided in the options, the path itself is used for
    /// resolving relative links and deriving the data source name.
    pub fn load_file(
        &mut self,
        path: &str,
        options: Option<&CzmlLoadOptions>,
    ) -> Result<(), String> {
        let mut options = options.cloned().unwrap_or_default();
        if options.source_uri.is_none() {
            options.source_uri = Some(path.replace('\\', "/"));
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {}", path, e))
            .and_then(|contents| serde_json::from_str::<Value>(&contents).map_err(|e| e.to_string()));
        let value = match contents {
            Ok(value) => value,
            Err(error) => {
                // Mirrors the JS reject path: the error event is raised on
                // load failure.
                self.error_event.raise_event(&());
                return Err(error);
            }
        };
        self.load_value(&value, Some(&options))
    }

    /// Processes CZML from a file path without clearing existing data.
    pub fn process_file(
        &mut self,
        path: &str,
        options: Option<&CzmlLoadOptions>,
    ) -> Result<(), String> {
        let mut options = options.cloned().unwrap_or_default();
        if options.source_uri.is_none() {
            options.source_uri = Some(path.replace('\\', "/"));
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {}", path, e))
            .and_then(|contents| serde_json::from_str::<Value>(&contents).map_err(|e| e.to_string()));
        let value = match contents {
            Ok(value) => value,
            Err(error) => {
                // Mirrors the JS reject path: the error event is raised on
                // process failure.
                self.error_event.raise_event(&());
                return Err(error);
            }
        };
        self.process_value(&value, Some(&options))
    }

    /// Common load path (mirror of the internal `load` function): applies the
    /// credit option, tracks the loading state and raises the error event on
    /// failure.
    fn load_inner(
        &mut self,
        czml: &Value,
        options: Option<&CzmlLoadOptions>,
        clear: bool,
    ) -> Result<(), String> {
        let options = options.cloned().unwrap_or_default();

        // User specified credit
        self.credit = options.credit.as_deref().map(|html| Credit::new(html, false));

        self.set_loading(true);
        let result = self.load_czml(czml, options.source_uri.as_deref(), clear);
        if let Err(ref error) = result {
            self.set_loading(false);
            self.error_event.raise_event(&());
            return Err(error.clone());
        }
        Ok(())
    }

    /// Core CZML processing (mirror of the internal `loadCzml`).
    fn load_czml(
        &mut self,
        czml: &Value,
        source_uri: Option<&str>,
        clear: bool,
    ) -> Result<(), String> {
        self.set_loading(true);

        if clear {
            self.version = None;
            self.document_packet = DocumentPacket::default();
            self.entity_collection.remove_all();
        }

        self.process_czml(czml, source_uri)?;

        let mut raise_changed_event = self.update_clock();

        if let Some(ref packet_name) = self.document_packet.name.clone() {
            if self.name.as_deref() != Some(packet_name.as_str()) {
                self.name = Some(packet_name.clone());
                raise_changed_event = true;
            }
        } else if self.name.is_none() && source_uri.is_some() {
            self.name = Some(get_filename_from_uri(source_uri));
            raise_changed_event = true;
        }

        self.set_loading(false);
        if raise_changed_event {
            self.changed_event.raise_event(&());
        }

        Ok(())
    }

    /// Sets the loading state, raising the loading event on change (mirror of
    /// `DataSource.setLoading`).
    fn set_loading(&mut self, value: bool) {
        if self.is_loading != value {
            self.is_loading = value;
            self.loading_event.raise_event(&());
        }
    }

    /// Processes the provided CZML value (a packet array or a single packet;
    /// mirror of `CzmlDataSource._processCzml`).
    fn process_czml(&mut self, czml: &Value, source_uri: Option<&str>) -> Result<(), String> {
        if let Some(packets) = czml.as_array() {
            for packet in packets {
                self.process_czml_packet(packet, source_uri)?;
            }
        } else {
            self.process_czml_packet(czml, source_uri)?;
        }
        Ok(())
    }

    /// Processes a single CZML packet (mirror of `processCzmlPacket`).
    fn process_czml_packet(&mut self, packet: &Value, source_uri: Option<&str>) -> Result<(), String> {
        let object_id = packet
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(create_guid);

        if self.version.is_none() && object_id != "document" {
            return Err(FIRST_PACKET_ERROR.to_string());
        }

        if packet.get("delete").and_then(|v| v.as_bool()) == Some(true) {
            self.entity_collection.remove(&object_id);
        } else if object_id == "document" {
            self.process_document(packet)?;
        } else {
            let parent_id = packet
                .get("parent")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(ref parent_id) = parent_id {
                // Ensure the parent exists before wiring it up (mirrors
                // `getOrCreateEntity(parentId)`).
                self.entity_collection.get_or_create_entity(parent_id);
            }

            // The updaters run in reverse order, mirroring CesiumJS's
            // `for (i = updaterFunctions.length - 1; i > -1; i--)` loop over
            // the registered updater list. Only the constant-value subset of
            // the updaters supported by the simplified value model is wired
            // here.
            let entity = self.entity_collection.get_or_create_entity(&object_id);
            if let Some(parent_id) = parent_id {
                entity.parent_id = Some(parent_id);
            }
            process_availability(entity, packet);
            process_orientation(entity, packet);
            process_view_from(entity, packet);
            process_position(entity, packet);
            process_properties(entity, packet);
            process_polyline(entity, packet);
            process_polygon(entity, packet);
            process_point(entity, packet);
            process_label(entity, packet);
            process_billboard(entity, packet, source_uri);
            process_description(entity, packet);
            process_name(entity, packet);
        }

        Ok(())
    }

    /// Processes the document packet (mirror of `processDocument`): validates
    /// the version and records the document name and clock packet.
    fn process_document(&mut self, packet: &Value) -> Result<(), String> {
        if let Some(version) = packet.get("version").and_then(|v| v.as_str()) {
            let tokens: Vec<&str> = version.split('.').collect();
            if tokens.len() == 2 {
                if tokens[0] != "1" {
                    return Err(VERSION_ERROR.to_string());
                }
                self.version = Some(version.to_string());
            }
        }

        if self.version.is_none() {
            return Err(VERSION_INVALID_ERROR.to_string());
        }

        if let Some(name) = packet.get("name").and_then(|v| v.as_str()) {
            self.document_packet.name = Some(name.to_string());
        }

        if let Some(clock_packet) = packet.get("clock") {
            if clock_packet.is_object() {
                let interval = czml_opt_string(clock_packet.get("interval"));
                let current_time = czml_opt_string(clock_packet.get("currentTime"));
                let range = czml_opt_string(clock_packet.get("range"));
                let step = czml_opt_string(clock_packet.get("step"));
                let multiplier = clock_packet.get("multiplier").and_then(|v| v.as_f64());

                if let Some(ref mut clock) = self.document_packet.clock {
                    // Merge with `??` semantics: keep the previous value when
                    // the new packet does not define the field.
                    if interval.is_some() { clock.interval = interval; }
                    if current_time.is_some() { clock.current_time = current_time; }
                    if range.is_some() { clock.range = range; }
                    if step.is_some() { clock.step = step; }
                    if multiplier.is_some() { clock.multiplier = multiplier; }
                } else {
                    self.document_packet.clock = Some(DocumentClockPacket {
                        interval,
                        current_time,
                        range,
                        step,
                        multiplier,
                    });
                }
            }
        }

        Ok(())
    }

    /// Updates the clock from the document packet, deriving one from the
    /// entity availability when no clock packet is present (mirror of
    /// `updateClock`). Returns whether the clock changed.
    fn update_clock(&mut self) -> bool {
        let clock_packet = self.document_packet.clock.clone();
        let Some(clock_packet) = clock_packet else {
            if self.clock.is_none() {
                let availability = self.entity_collection.compute_availability();
                if !JulianDate::equals(&availability.start, Iso8601::minimum_value()) {
                    let start_time = availability.start.clone();
                    let stop_time = availability.stop.clone();
                    let total_seconds =
                        JulianDate::seconds_difference(&stop_time, &start_time);
                    let multiplier = (total_seconds / 120.0).round();

                    self.clock = Some(DataSourceClock {
                        start_time: start_time.clone(),
                        stop_time,
                        current_time: start_time,
                        clock_range: ClockRange::LoopStop,
                        clock_step: ClockStep::SystemClockMultiplier,
                        multiplier,
                    });
                    return true;
                }
            }
            return false;
        };

        let mut clock = if let Some(ref existing) = self.clock {
            existing.clone_clock()
        } else {
            let mut clock = DataSourceClock::new();
            clock.start_time = Iso8601::minimum_value().clone();
            clock.stop_time = Iso8601::maximum_value().clone();
            clock.current_time = Iso8601::minimum_value().clone();
            clock.clock_range = ClockRange::LoopStop;
            clock.clock_step = ClockStep::SystemClockMultiplier;
            clock.multiplier = 1.0;
            clock
        };

        if let Some(ref interval_string) = clock_packet.interval {
            if let Some(interval) = TimeInterval::from_iso8601(interval_string, None, None) {
                clock.start_time = interval.start;
                clock.stop_time = interval.stop;
            }
        }

        if let Some(ref current_time) = clock_packet.current_time {
            if let Some(date) = JulianDate::from_iso8601(current_time) {
                clock.current_time = date;
            }
        }
        if let Some(ref range) = clock_packet.range {
            clock.clock_range = clock_range_from_name(range).unwrap_or(ClockRange::LoopStop);
        }
        if let Some(ref step) = clock_packet.step {
            clock.clock_step =
                clock_step_from_name(step).unwrap_or(ClockStep::SystemClockMultiplier);
        }
        if let Some(multiplier) = clock_packet.multiplier {
            clock.multiplier = multiplier;
        }

        if !clock.equals(self.clock.as_ref()) {
            self.clock = Some(clock);
            return true;
        }

        false
    }
}

impl Default for CzmlDataSource {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for CzmlDataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CzmlDataSource")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl DataSource for CzmlDataSource {
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
// Updater functions (constant subset; each mirrors the corresponding
// `processXxx` function from CzmlDataSource.js)
// ============================================================================

fn process_name(entity: &mut Entity, packet: &Value) {
    if let Some(name) = packet.get("name").and_then(|v| v.as_str()) {
        entity.name = Some(name.to_string());
    }
}

fn process_description(entity: &mut Entity, packet: &Value) {
    let Some(description_data) = packet.get("description") else {
        return;
    };
    if let Some(description) = czml_string_constant(description_data) {
        entity.description = Some(description);
    }
}

fn process_position(entity: &mut Entity, packet: &Value) {
    let Some(position_data) = packet.get("position") else {
        return;
    };
    // DEVIATION: references, sampled data and interval-constrained positions
    // require the full property system and are skipped in this port.
    if position_data.get("reference").is_some() || position_data.get("interval").is_some() {
        return;
    }
    if let Some(position) = czml_cartesian3_constant(position_data) {
        entity.position = Some(position);
    }
}

fn process_view_from(entity: &mut Entity, packet: &Value) {
    let Some(view_from_data) = packet.get("viewFrom") else {
        return;
    };
    if let Some(view_from) = czml_cartesian3_constant(view_from_data) {
        entity.view_from = Some(view_from);
    }
}

fn process_orientation(entity: &mut Entity, packet: &Value) {
    let Some(orientation_data) = packet.get("orientation") else {
        return;
    };
    // DEVIATION: velocityReference and sampled orientations are skipped.
    if orientation_data.get("velocityReference").is_some() {
        return;
    }
    if let Some(orientation) = czml_quaternion_constant(orientation_data) {
        entity.orientation = Some(orientation);
    }
}

fn process_properties(entity: &mut Entity, packet: &Value) {
    let Some(properties_data) = packet.get("properties") else {
        return;
    };
    let Some(object) = properties_data.as_object() else {
        return;
    };

    for (key, property_data) in object {
        if let Some(result) = process_custom_property(property_data) {
            entity.properties.set(key, result);
        }
    }
}

/// Extracts a constant custom property value (mirror of the constant paths of
/// `processProperty` + `getPropertyType`/`unwrapInterval` for the `Object`
/// type). Returns `None` for interval/sampled/reference definitions.
fn process_custom_property(property_data: &Value) -> Option<PropertyResult> {
    match property_data {
        Value::Bool(b) => Some(PropertyResult::Boolean(*b)),
        Value::Number(n) => Some(PropertyResult::Number(n.as_f64()?)),
        Value::String(s) => Some(PropertyResult::String(s.clone())),
        Value::Object(map) => {
            // Interval/sampled definitions are not supported by the
            // simplified value model.
            if map.contains_key("interval") || map.contains_key("epoch") {
                return None;
            }
            if let Some(value) = map.get("object").or_else(|| map.get("value")) {
                return Some(PropertyResult::Json(value.clone()));
            }
            if let Some(array) = map.get("array") {
                return Some(PropertyResult::Json(array.clone()));
            }
            if let Some(s) = map.get("string").and_then(|v| v.as_str()) {
                return Some(PropertyResult::String(s.to_string()));
            }
            if let Some(n) = map.get("number").and_then(|v| v.as_f64()) {
                return Some(PropertyResult::Number(n));
            }
            if let Some(b) = map.get("boolean").and_then(|v| v.as_bool()) {
                return Some(PropertyResult::Boolean(b));
            }
            None
        }
        // Arrays of interval definitions (multi-interval properties) are not
        // supported by the simplified value model.
        _ => None,
    }
}

fn process_availability(entity: &mut Entity, packet: &Value) {
    let Some(packet_data) = packet.get("availability") else {
        return;
    };

    let mut intervals: Vec<TimeInterval> = Vec::new();
    if let Some(array) = packet_data.as_array() {
        for item in array {
            if let Some(s) = item.as_str() {
                if let Some(interval) = TimeInterval::from_iso8601(s, None, None) {
                    intervals.push(interval);
                }
            }
        }
    } else if let Some(s) = packet_data.as_str() {
        if let Some(interval) = TimeInterval::from_iso8601(s, None, None) {
            intervals.push(interval);
        }
    }

    if !intervals.is_empty() {
        entity.availability = intervals;
    }
}

fn process_billboard(entity: &mut Entity, packet: &Value, source_uri: Option<&str>) {
    let Some(billboard_data) = packet.get("billboard") else {
        return;
    };
    let billboard = entity.billboard.get_or_insert_with(BillboardGraphics::new);

    if billboard_data.get("interval").is_some() {
        // DEVIATION: interval-constrained graphics are skipped.
        return;
    }

    if let Some(show) = czml_bool_constant(billboard_data.get("show")) {
        billboard.show = show;
    }
    if let Some(uri) = billboard_data
        .get("image")
        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| {
            v.get("uri").and_then(|u| u.as_str()).map(|s| s.to_string())
        }))
    {
        billboard.image = Some(resolve_uri(source_uri, &uri));
    }
    if let Some(scale) = czml_number_constant(billboard_data.get("scale")) {
        billboard.scale = scale;
    }
    if let Some(color) = billboard_data.get("color").and_then(czml_color_constant) {
        billboard.color = Some(color);
    }
    if let Some(rotation) = czml_number_constant(billboard_data.get("rotation")) {
        billboard.rotation = rotation;
    }
    if let Some(name) = billboard_data.get("heightReference").and_then(|v| v.as_str()) {
        billboard.height_reference = height_reference_from_name(name).unwrap_or(0);
    }
    if let Some(name) = billboard_data.get("horizontalOrigin").and_then(|v| v.as_str()) {
        billboard.horizontal_origin = horizontal_origin_from_name(name).unwrap_or(0);
    }
    if let Some(name) = billboard_data.get("verticalOrigin").and_then(|v| v.as_str()) {
        billboard.vertical_origin = vertical_origin_from_name(name).unwrap_or(0);
    }
    if let Some(eye_offset) = billboard_data.get("eyeOffset").and_then(czml_cartesian3_constant) {
        billboard.eye_offset = Some(eye_offset);
    }
    if let Some(pixel_offset) = billboard_data.get("pixelOffset").and_then(czml_cartesian2_constant) {
        billboard.pixel_offset = Some(pixel_offset);
    }
    if let Some(aligned_axis) = billboard_data.get("alignedAxis").and_then(czml_cartesian3_constant) {
        billboard.aligned_axis = Some(aligned_axis);
    }
    if let Some(size_in_meters) = czml_bool_constant(billboard_data.get("sizeInMeters")) {
        billboard.size_in_meters = Some(size_in_meters);
    }
    if let Some(width) = czml_number_constant(billboard_data.get("width")) {
        billboard.width = Some(width);
    }
    if let Some(height) = czml_number_constant(billboard_data.get("height")) {
        billboard.height = Some(height);
    }
    if let Some(nfs) = billboard_data.get("scaleByDistance").and_then(czml_near_far_scalar_constant) {
        billboard.scale_by_distance = Some(nfs);
    }
    if let Some(nfs) = billboard_data.get("translucencyByDistance").and_then(czml_near_far_scalar_constant) {
        billboard.translucency_by_distance = Some(nfs);
    }
    if let Some(nfs) = billboard_data.get("pixelOffsetScaleByDistance").and_then(czml_near_far_scalar_constant) {
        billboard.pixel_offset_scale_by_distance = Some(nfs);
    }
    if let Some(region) = billboard_data.get("imageSubRegion").and_then(|v| {
        v.get("boundingRectangle")
            .and_then(|a| a.as_array())
            .filter(|a| a.len() == 4)
            .map(|a| {
                (
                    a[0].as_f64().unwrap_or(0.0),
                    a[1].as_f64().unwrap_or(0.0),
                    a[2].as_f64().unwrap_or(0.0),
                    a[3].as_f64().unwrap_or(0.0),
                )
            })
    }) {
        billboard.image_sub_region = Some(region);
    }
}

fn process_label(entity: &mut Entity, packet: &Value) {
    let Some(label_data) = packet.get("label") else {
        return;
    };
    let label = entity.label.get_or_insert_with(LabelGraphics::new);

    if label_data.get("interval").is_some() {
        // DEVIATION: interval-constrained graphics are skipped.
        return;
    }

    if let Some(text) = label_data.get("text").and_then(|v| v.as_str()) {
        label.text = Some(text.to_string());
    }
    if let Some(font) = label_data.get("font").and_then(|v| v.as_str()) {
        label.font = Some(font.to_string());
    }
    if let Some(name) = label_data.get("style").and_then(|v| v.as_str()) {
        label.style = label_style_from_name(name).unwrap_or(0);
    }
    if let Some(fill_color) = label_data.get("fillColor").and_then(czml_color_constant) {
        label.fill_color = fill_color;
    }
    if let Some(outline_color) = label_data.get("outlineColor").and_then(czml_color_constant) {
        label.outline_color = outline_color;
    }
    if let Some(outline_width) = czml_number_constant(label_data.get("outlineWidth")) {
        label.outline_width = outline_width;
    }
    if let Some(name) = label_data.get("horizontalOrigin").and_then(|v| v.as_str()) {
        label.horizontal_origin = horizontal_origin_from_name(name).unwrap_or(0);
    }
    if let Some(name) = label_data.get("verticalOrigin").and_then(|v| v.as_str()) {
        label.vertical_origin = vertical_origin_from_name(name).unwrap_or(0);
    }
    if let Some(eye_offset) = label_data.get("eyeOffset").and_then(czml_cartesian3_constant) {
        label.eye_offset = Some(eye_offset);
    }
    if let Some(pixel_offset) = label_data.get("pixelOffset").and_then(czml_cartesian2_constant) {
        label.pixel_offset = Some(pixel_offset);
    }
    if let Some(scale) = czml_number_constant(label_data.get("scale")) {
        label.scale = scale;
    }
    if let Some(show) = czml_bool_constant(label_data.get("show")) {
        label.show = show;
    }
    if let Some(nfs) = label_data.get("translucencyByDistance").and_then(czml_near_far_scalar_constant) {
        label.translucency_by_distance = Some(nfs);
    }
    if let Some(nfs) = label_data.get("pixelOffsetScaleByDistance").and_then(czml_near_far_scalar_constant) {
        label.pixel_offset_scale_by_distance = Some(nfs);
    }
}

fn process_point(entity: &mut Entity, packet: &Value) {
    let Some(point_data) = packet.get("point") else {
        return;
    };
    let point = entity.point.get_or_insert_with(PointGraphics::new);

    if point_data.get("interval").is_some() {
        // DEVIATION: interval-constrained graphics are skipped.
        return;
    }

    if let Some(color) = point_data.get("color").and_then(czml_color_constant) {
        point.color = color;
    }
    if let Some(pixel_size) = czml_number_constant(point_data.get("pixelSize")) {
        point.pixel_size = pixel_size;
    }
    if let Some(outline_color) = point_data.get("outlineColor").and_then(czml_color_constant) {
        point.outline_color = outline_color;
    }
    if let Some(outline_width) = czml_number_constant(point_data.get("outlineWidth")) {
        point.outline_width = outline_width;
    }
    if let Some(show) = czml_bool_constant(point_data.get("show")) {
        point.show = show;
    }
    if let Some(nfs) = point_data.get("scaleByDistance").and_then(czml_near_far_scalar_constant) {
        point.scale_by_distance = Some(nfs);
    }
    if let Some(nfs) = point_data.get("translucencyByDistance").and_then(czml_near_far_scalar_constant) {
        point.translucency_by_distance = Some(nfs);
    }
    if let Some(name) = point_data.get("heightReference").and_then(|v| v.as_str()) {
        point.height_reference = height_reference_from_name(name).unwrap_or(0);
    }
}

fn process_polyline(entity: &mut Entity, packet: &Value) {
    let Some(polyline_data) = packet.get("polyline") else {
        return;
    };

    if let Some(positions_data) = polyline_data.get("positions") {
        // DEVIATION: references and interval-constrained position arrays are
        // skipped in this port.
        if positions_data.get("references").is_some() || positions_data.get("interval").is_some() {
            return;
        }
        let positions = if let Some(cartesian) = positions_data
            .get("cartesian")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
        {
            unpack_cartesian_array(&cartesian)
        } else if let Some(radians) = positions_data
            .get("cartographicRadians")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
        {
            Cartesian3::from_radians_array_heights(&radians, None, None)
        } else if let Some(degrees) = positions_data
            .get("cartographicDegrees")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
        {
            Cartesian3::from_degrees_array_heights(&degrees, None, None)
        } else {
            return;
        };

        let polyline = entity.polyline.get_or_insert_with(PolylineGraphics::new);
        polyline.positions = positions;
    }
}

fn process_polygon(entity: &mut Entity, packet: &Value) {
    let Some(polygon_data) = packet.get("polygon") else {
        return;
    };

    // Constant subset: the outer ring positions of the hierarchy.
    if let Some(hierarchy_data) = polygon_data.get("positions") {
        if hierarchy_data.get("references").is_some() || hierarchy_data.get("interval").is_some() {
            return;
        }
        let positions = if let Some(cartesian) = hierarchy_data
            .get("cartesian")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
        {
            unpack_cartesian_array(&cartesian)
        } else if let Some(degrees) = hierarchy_data
            .get("cartographicDegrees")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
        {
            Cartesian3::from_degrees_array_heights(&degrees, None, None)
        } else if let Some(radians) = hierarchy_data
            .get("cartographicRadians")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
        {
            Cartesian3::from_radians_array_heights(&radians, None, None)
        } else {
            return;
        };

        let polygon = entity.polygon.get_or_insert_with(crate::polygon_graphics::PolygonGraphics::new);
        polygon.hierarchy = positions;
    }
}

// ============================================================================
// CZML constant-value extraction helpers (mirror the constant paths of
// `unwrapInterval`/`unwrapColorInterval`/`unwrapCartesianInterval` & friends)
// ============================================================================

/// Returns the value as a plain string (JSON null maps to `None`, mirroring
/// JS `undefined`).
fn czml_opt_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extracts a constant boolean (mirror of `czmlInterval["boolean"] ?? czmlInterval`).
fn czml_bool_constant(value: Option<&Value>) -> Option<bool> {
    let value = value?;
    if let Some(b) = value.as_bool() {
        return Some(b);
    }
    value.get("boolean").and_then(|v| v.as_bool())
}

/// Extracts a constant number (mirror of `czmlInterval.number ?? czmlInterval`).
fn czml_number_constant(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    if let Some(n) = value.as_f64() {
        return Some(n);
    }
    value.get("number").and_then(|v| v.as_f64())
}

/// Extracts a constant string (mirror of `czmlInterval.string ?? czmlInterval`).
fn czml_string_constant(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    value
        .get("string")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extracts a constant color (mirror of `unwrapColorInterval`): supports
/// `rgbaf` (floats) and `rgba` (bytes). Unknown encodings return `None`.
fn czml_color_constant(value: &Value) -> Option<Color> {
    if let Some(rgbaf) = value.get("rgbaf").and_then(|v| v.as_array()) {
        if rgbaf.len() == 4 {
            return Some(Color::new(
                rgbaf[0].as_f64()?,
                rgbaf[1].as_f64()?,
                rgbaf[2].as_f64()?,
                rgbaf[3].as_f64()?,
            ));
        }
        return None;
    }
    if let Some(rgba) = value.get("rgba").and_then(|v| v.as_array()) {
        if rgba.len() == 4 {
            return Some(Color::new(
                byte_to_float(rgba[0].as_f64()?),
                byte_to_float(rgba[1].as_f64()?),
                byte_to_float(rgba[2].as_f64()?),
                byte_to_float(rgba[3].as_f64()?),
            ));
        }
    }
    None
}

/// Mirror of `Color.byteToFloat`.
fn byte_to_float(byte: f64) -> f64 {
    byte / 255.0
}

/// Extracts a constant Cartesian3 (mirror of the constant path of
/// `unwrapCartesianInterval`): supports `cartesian`, `unitCartesian`
/// (normalized), `cartographicRadians` and `cartographicDegrees`. Sampled
/// (packed time) arrays return `None`.
fn czml_cartesian3_constant(value: &Value) -> Option<Cartesian3> {
    if let Some(cartesian) = value
        .get("cartesian")
        .or_else(|| value.get("unitCartesian"))
        .and_then(|v| v.as_array())
    {
        if cartesian.len() == 3 {
            let mut result = Cartesian3::new(
                cartesian[0].as_f64()?,
                cartesian[1].as_f64()?,
                cartesian[2].as_f64()?,
            );
            if value.get("unitCartesian").is_some() {
                let input = result;
                Cartesian3::normalize(&input, &mut result);
            }
            return Some(result);
        }
        return None;
    }
    if let Some(radians) = value.get("cartographicRadians").and_then(|v| v.as_array()) {
        if radians.len() == 3 {
            return Some(Cartesian3::from_radians_new(
                radians[0].as_f64()?,
                radians[1].as_f64()?,
                Some(radians[2].as_f64()?),
                None,
            ));
        }
        return None;
    }
    if let Some(degrees) = value.get("cartographicDegrees").and_then(|v| v.as_array()) {
        if degrees.len() == 3 {
            return Some(Cartesian3::from_degrees_new(
                degrees[0].as_f64()?,
                degrees[1].as_f64()?,
                Some(degrees[2].as_f64()?),
                None,
            ));
        }
    }
    None
}

/// Extracts a constant Cartesian2 (mirror of `czmlInterval.cartesian2`).
fn czml_cartesian2_constant(value: &Value) -> Option<(f64, f64)> {
    let array = value.get("cartesian2").and_then(|v| v.as_array())?;
    if array.len() == 2 {
        return Some((array[0].as_f64()?, array[1].as_f64()?));
    }
    None
}

/// Extracts a constant NearFarScalar (mirror of `czmlInterval.nearFarScalar`).
fn czml_near_far_scalar_constant(value: &Value) -> Option<NearFarScalar> {
    let array = value.get("nearFarScalar").and_then(|v| v.as_array())?;
    if array.len() == 4 {
        return Some(NearFarScalar {
            near: array[0].as_f64()?,
            near_value: array[1].as_f64()?,
            far: array[2].as_f64()?,
            far_value: array[3].as_f64()?,
        });
    }
    None
}

/// Extracts a constant quaternion (mirror of the constant path of
/// `unwrapQuaternionInterval`): unpacks `unitQuaternion` and normalizes it.
fn czml_quaternion_constant(value: &Value) -> Option<Quaternion> {
    let array = value.get("unitQuaternion").and_then(|v| v.as_array())?;
    if array.len() == 4 {
        let mut result = Quaternion::new(
            array[0].as_f64()?,
            array[1].as_f64()?,
            array[2].as_f64()?,
            array[3].as_f64()?,
        );
        let input = result;
        Quaternion::normalize(&input, &mut result);
        return Some(result);
    }
    None
}

/// Unpacks a packed cartesian array into positions (mirror of
/// `Cartesian3.unpackArray`).
fn unpack_cartesian_array(array: &[f64]) -> Vec<Cartesian3> {
    let mut result = Vec::with_capacity(array.len() / 3);
    let mut i = 0;
    while i + 3 <= array.len() {
        result.push(Cartesian3::new(array[i], array[i + 1], array[i + 2]));
        i += 3;
    }
    result
}

/// Resolves a relative uri against the source uri (simplified mirror of
/// `sourceUri.getDerivedResource({ url })` / `combineUris`).
fn resolve_uri(source_uri: Option<&str>, uri: &str) -> String {
    let Some(source_uri) = source_uri else {
        return uri.to_string();
    };
    if source_uri.ends_with('/') {
        format!("{}{}", source_uri, uri)
    } else {
        format!("{}/{}", source_uri, uri)
    }
}

/// Maps a CZML `ClockRange` name to the enum value.
fn clock_range_from_name(name: &str) -> Option<ClockRange> {
    match name {
        "UNBOUNDED" => Some(ClockRange::Unbounded),
        "CLAMPED" => Some(ClockRange::Clamped),
        "LOOP_STOP" => Some(ClockRange::LoopStop),
        _ => None,
    }
}

/// Maps a CZML `ClockStep` name to the enum value.
fn clock_step_from_name(name: &str) -> Option<ClockStep> {
    match name {
        "TICK_DEPENDENT" => Some(ClockStep::TickDependent),
        "SYSTEM_CLOCK_MULTIPLIER" => Some(ClockStep::SystemClockMultiplier),
        "SYSTEM_CLOCK" => Some(ClockStep::SystemClock),
        _ => None,
    }
}

/// Maps a CZML `HeightReference` name to the enum discriminant.
fn height_reference_from_name(name: &str) -> Option<i32> {
    match name {
        "NONE" => Some(0),
        "CLAMP_TO_GROUND" => Some(1),
        "RELATIVE_TO_GROUND" => Some(2),
        _ => None,
    }
}

/// Maps a CZML `HorizontalOrigin` name to the enum discriminant.
fn horizontal_origin_from_name(name: &str) -> Option<i32> {
    match name {
        "CENTER" => Some(0),
        "LEFT" => Some(cesium_scene::horizontal_origin::HorizontalOrigin::Left as i32),
        "RIGHT" => Some(cesium_scene::horizontal_origin::HorizontalOrigin::Right as i32),
        _ => None,
    }
}

/// Maps a CZML `VerticalOrigin` name to the enum discriminant.
fn vertical_origin_from_name(name: &str) -> Option<i32> {
    match name {
        "CENTER" => Some(0),
        "BOTTOM" => Some(cesium_scene::vertical_origin::VerticalOrigin::Bottom as i32),
        "TOP" => Some(cesium_scene::vertical_origin::VerticalOrigin::Top as i32),
        _ => None,
    }
}

/// Maps a CZML `LabelStyle` name to the enum discriminant.
fn label_style_from_name(name: &str) -> Option<i32> {
    match name {
        "FILL" => Some(0),
        "OUTLINE" => Some(1),
        "FILL_AND_OUTLINE" => Some(2),
        _ => None,
    }
}

/// Creates a GUID-like identifier for packets without an `id` (mirror of
/// `createGuid`).
fn create_guid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("czml-guid-{:x}-{:x}", t, n)
}
