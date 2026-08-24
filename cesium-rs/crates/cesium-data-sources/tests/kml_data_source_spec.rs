//! Ported specs from `packages/engine/Specs/DataSources/KmlDataSourceSpec.js`.
//!
//! Every test mirrors one `it()` of the original Jasmine spec; the test
//! names keep the original descriptions snake-cased so they stay mappable.
//!
//! DEVIATION (simplified value model): CesiumJS stores KML style/geometry
//! values as time-dynamic `Property` objects evaluated with `getValue`.
//! This port materializes the constant subset directly, so assertions read
//! the stored values. Scalar properties that were never set keep their
//! default value instead of being `undefined`, so a few "toBeUndefined"
//! assertions are mirrored as default-value checks or skipped.
//!
//! DEVIATION (browser facilities): specs that require KMZ decoding, URL or
//! `Resource` fetching, deferred loading, ScreenOverlay DOM nodes,
//! BalloonStyle HTML rewriting, `PinBuilder` images, NetworkLink timers,
//! Tour playback, gx:Track/MultiTrack, Model/Point-extrude drop lines,
//! moon ellipsoid options, external style documents and atom:author/link
//! metadata are not mirrored (see the module note in `kml_data_source.rs`).

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::color::Color;
use cesium_core::iso8601::Iso8601;
use cesium_core::julian_date::JulianDate;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_data_sources::data_source::DataSource;
use cesium_data_sources::entity::Entity;
use cesium_data_sources::kml_data_source::{KmlDataSource, KmlLoadOptions};
use cesium_data_sources::property::PropertyResult;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::horizontal_origin::HorizontalOrigin;
use cesium_scene::label_style::LabelStyle;

// ============================================================================
// Helpers
// ============================================================================

fn load(kml: &str) -> KmlDataSource {
    KmlDataSource::load(kml, None).unwrap()
}

fn load_with(kml: &str, options: &KmlLoadOptions) -> KmlDataSource {
    KmlDataSource::load(kml, Some(options)).unwrap()
}

/// Extracts the `kml` feature metadata object stored on an entity.
fn kml_metadata(entity: &Entity) -> serde_json::Value {
    match entity.properties.get("kml").expect("kml metadata present") {
        PropertyResult::Json(value) => value.clone(),
        other => panic!("expected json metadata, got {:?}", other),
    }
}

fn first_entity(data_source: &KmlDataSource) -> &Entity {
    data_source
        .entities()
        .values()
        .into_iter()
        .next()
        .expect("at least one entity")
}

fn position_equals(left: &Cartesian3, right: &Cartesian3) -> bool {
    Cartesian3::equals_epsilon(Some(left), Some(right), Some(1e-13), None)
}

/// Mirrors the spec `uberStyle` constant.
const UBER_STYLE: &str = r#"
        <Style>
            <LineStyle>
              <color>aaaaaaaa</color>
              <width>2</width>
            </LineStyle>
            <PolyStyle>
              <color>cccccccc</color>
              <fill>0</fill>
              <outline>0</outline>
            </PolyStyle>
            <IconStyle>
              <color>dddddddd</color>
              <scale>3</scale>
              <heading>45</heading>
              <Icon>
                <href>test.png</href>
              </Icon>
              <hotSpot x="1" y="2" xunits="pixels" yunits="pixels"/>
            </IconStyle>
            <LabelStyle>
              <color>eeeeeeee</color>
              <scale>4</scale>
            </LabelStyle>
        </Style>"#;

// ============================================================================
// Constructor / name / show specs
// ============================================================================

#[test]
fn default_constructor_has_expected_values() {
    let data_source = KmlDataSource::new();
    assert!(data_source.display_name().is_none());
    assert!(data_source.clock().is_none());
    assert_eq!(data_source.entities().length(), 0);
    assert!(data_source.show());
    assert!(!data_source.is_loading());
}

#[test]
fn setting_name_raises_changed_event() {
    let mut data_source = KmlDataSource::new();
    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    data_source.set_name(Some("new name"));
    assert_eq!(count.get(), 1);
    assert_eq!(data_source.display_name(), Some("new name"));

    data_source.set_name(None);
    assert_eq!(count.get(), 2);
    assert!(data_source.display_name().is_none());
}

#[test]
fn show_sets_underlying_entity_collection_show() {
    let mut data_source = KmlDataSource::new();
    data_source.set_show(false);
    assert!(!data_source.show());
    assert!(!data_source.entities().show);

    data_source.set_show(true);
    assert!(data_source.show());
    assert!(data_source.entities().show);
}

#[test]
fn load_rejects_loading_non_kml_text() {
    let mut data_source = KmlDataSource::new();
    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .error_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    assert!(data_source.load_value("this is not kml", None).is_err());
    assert_eq!(count.get(), 1);
}

#[test]
fn sets_data_source_name_from_document() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            <name>NameInKml</name>
            </Document>"#;

    let mut options = KmlLoadOptions::default();
    options.source_uri = Some("NameFromUri.kml".to_string());
    let data_source = load_with(kml, &options);
    assert_eq!(data_source.display_name(), Some("NameInKml"));
}

#[test]
fn sets_data_source_name_from_document_with_kml_element() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <kml>
            <Document>
            <name>NameInKml</name>
            </Document>
            </kml>"#;

    let mut options = KmlLoadOptions::default();
    options.source_uri = Some("NameFromUri.kml".to_string());
    let data_source = load_with(kml, &options);
    assert_eq!(data_source.display_name(), Some("NameInKml"));
}

#[test]
fn sets_data_source_name_from_source_uri_when_not_in_file() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            </Document>"#;

    let mut options = KmlLoadOptions::default();
    options.source_uri = Some("NameFromUri.kml".to_string());
    let data_source = load_with(kml, &options);
    assert_eq!(data_source.display_name(), Some("NameFromUri.kml"));
}

#[test]
fn raises_changed_event_when_the_name_changes() {
    let mut kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            <name>NameInKml</name>
            </Document>"#
        .to_string();

    let mut data_source = KmlDataSource::new();
    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    // Initial load
    data_source.load_value(&kml, None).unwrap();
    assert_eq!(count.get(), 1);

    // Loading KML with same name
    data_source.load_value(&kml, None).unwrap();
    assert_eq!(count.get(), 1);

    // Loading KML with different name.
    kml = kml.replace("NameInKml", "newName");
    data_source.load_value(&kml, None).unwrap();
    assert_eq!(count.get(), 2);
}

#[test]
fn raises_loading_event_at_start_and_end_of_load() {
    let mut data_source = KmlDataSource::new();
    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .loading_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            <name>NameInKml</name>
            </Document>"#;
    data_source.load_value(kml, None).unwrap();
    // The synchronous port raises loading(true) then loading(false).
    assert_eq!(count.get(), 2);
    assert!(!data_source.is_loading());
}

#[test]
fn raises_unsupported_node_event_when_parsing_an_unsupported_kml_node_type() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
            <PhotoOverlay>
            </PhotoOverlay>
        </Document>"#;

    let mut data_source = KmlDataSource::new();
    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .unsupported_node_event()
        .add_listener(move |_a: &()| spy.set(spy.get() + 1));

    data_source.load_value(kml, None).unwrap();
    assert_eq!(count.get(), 1);
}

#[test]
fn sets_data_source_clock_based_on_feature_availability() {
    // DEVIATION: the JS spec derives availability from a GroundOverlay
    // TimeSpan and a gx:Track, neither of which this port materializes;
    // two Placemark TimeSpans produce the same union interval instead.
    let begin_date = JulianDate::from_iso8601("2000-01-01").unwrap();
    let end_date = JulianDate::from_iso8601("2000-01-04").unwrap();

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <Placemark>
            <TimeSpan>
              <begin>2000-01-01</begin>
              <end>2000-01-03</end>
            </TimeSpan>
          </Placemark>
          <Placemark>
            <TimeSpan>
              <begin>2000-01-02</begin>
              <end>2000-01-04</end>
            </TimeSpan>
          </Placemark>
        </Document>"#;

    let mut data_source = load(kml);
    let clock = data_source.clock().expect("clock is defined");
    assert!(JulianDate::equals(&clock.start_time, &begin_date));
    assert!(JulianDate::equals(&clock.stop_time, &end_date));
    assert!(JulianDate::equals(&clock.current_time, &begin_date));
    assert_eq!(clock.clock_range, ClockRange::LoopStop);
    assert_eq!(clock.clock_step, ClockStep::SystemClockMultiplier);
    assert_eq!(
        clock.multiplier,
        JulianDate::seconds_difference(&end_date, &begin_date) / 60.0
    );

    // Loading a static data set should clear the clock.
    let static_kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            </Document>"#;
    data_source.load_value(static_kml, None).unwrap();
    assert!(data_source.clock().is_none());
}

// ============================================================================
// Feature specs
// ============================================================================

#[test]
fn feature_id() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark id="Bob">
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.id, "Bob");
}

#[test]
fn feature_duplicate_id() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
            <Placemark id="Bob">
            </Placemark>
            <Placemark id="Bob">
            </Placemark>
        </Document>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities[0].id, "Bob");
    assert_ne!(entities[1].id, "Bob");
}

#[test]
fn feature_name() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <name>bob</name>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.name.as_deref(), Some("bob"));
    let label = entity.label.as_ref().expect("label is defined");
    assert_eq!(label.text.as_deref(), Some("bob"));
}

#[test]
fn feature_address() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <address>1826 South 16th Street</address>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let metadata = kml_metadata(&entity);
    assert_eq!(metadata["address"], "1826 South 16th Street");
}

#[test]
fn feature_phone_number() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <phoneNumber>555-555-5555</phoneNumber>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let metadata = kml_metadata(&entity);
    assert_eq!(metadata["phoneNumber"], "555-555-5555");
}

#[test]
fn feature_snippet() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <Snippet>Hey!</Snippet>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let metadata = kml_metadata(&entity);
    assert_eq!(metadata["snippet"], "Hey!");
}

#[test]
fn feature_time_span_with_begin_and_end() {
    let end_date = JulianDate::from_iso8601("1945-08-06").unwrap();
    let begin_date = JulianDate::from_iso8601("1941-12-07").unwrap();

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeSpan>
              <begin>1945-08-06</begin>
              <end>1941-12-07</end>
            </TimeSpan>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.availability.len(), 1);
    let interval = &entity.availability[0];
    assert!(JulianDate::equals(&interval.start, &begin_date));
    assert!(JulianDate::equals(&interval.stop, &end_date));
}

#[test]
fn feature_time_span_flips_dates_when_end_is_earlier() {
    let end_date = JulianDate::from_iso8601("1945-08-06").unwrap();
    let begin_date = JulianDate::from_iso8601("1941-12-07").unwrap();

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeSpan>
              <begin>1941-12-07</begin>
              <end>1945-08-06</end>
            </TimeSpan>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.availability.len(), 1);
    let interval = &entity.availability[0];
    assert!(JulianDate::equals(&interval.start, &begin_date));
    assert!(JulianDate::equals(&interval.stop, &end_date));
}

#[test]
fn feature_time_span_gracefully_handles_empty_fields() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeSpan>
            </TimeSpan>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(entity.availability.is_empty());
}

#[test]
fn feature_time_span_works_with_end_only_interval() {
    let date = JulianDate::from_iso8601("1941-12-07").unwrap();

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeSpan>
              <end>1941-12-07</end>
            </TimeSpan>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.availability.len(), 1);
    let interval = &entity.availability[0];
    assert!(JulianDate::equals(&interval.start, Iso8601::minimum_value()));
    assert!(JulianDate::equals(&interval.stop, &date));
}

#[test]
fn feature_time_span_works_with_begin_only_interval() {
    let date = JulianDate::from_iso8601("1941-12-07").unwrap();

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeSpan>
              <begin>1941-12-07</begin>
            </TimeSpan>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.availability.len(), 1);
    let interval = &entity.availability[0];
    assert!(JulianDate::equals(&interval.start, &date));
    assert!(JulianDate::equals(&interval.stop, Iso8601::maximum_value()));
}

#[test]
fn feature_time_stamp_works() {
    let date = JulianDate::from_iso8601("1941-12-07").unwrap();

    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeStamp>
              <when>1941-12-07</when>
            </TimeStamp>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.availability.len(), 1);
    let interval = &entity.availability[0];
    assert!(JulianDate::equals(&interval.start, &date));
    assert!(JulianDate::equals(&interval.stop, Iso8601::maximum_value()));
}

#[test]
fn feature_visibility_works() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <visibility>0</visibility>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(!entity.show);
}

#[test]
fn feature_time_stamp_gracefully_handles_empty_fields() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeStamp>
            </TimeStamp>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(entity.availability.is_empty());
}

#[test]
fn feature_time_stamp_gracefully_handles_empty_when_field() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <TimeStamp>
              <when></when>
            </TimeStamp>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(entity.availability.is_empty());
}

#[test]
fn feature_extended_data_schema() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Placemark>
            <ExtendedData>
                <Data name="prop1">
                    <displayName>Property 1</displayName>
                    <value>1</value>
                </Data>
                <Data name="prop2">
                    <value>2</value>
                </Data>
                <Data name="prop3">
                    <displayName>Property 3</displayName>
                </Data>
            </ExtendedData>
        </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let metadata = kml_metadata(&entity);
    let extended = &metadata["extendedData"];
    assert!(extended.is_object());

    assert_eq!(extended["prop1"]["displayName"], "Property 1");
    assert_eq!(extended["prop1"]["value"], "1");

    assert_eq!(extended["prop2"]["displayName"], serde_json::Value::Null);
    assert_eq!(extended["prop2"]["value"], "2");

    assert_eq!(extended["prop3"]["displayName"], "Property 3");
    assert_eq!(extended["prop3"]["value"], serde_json::Value::Null);
}

// ============================================================================
// Style specs
// ============================================================================

#[test]
fn styles_supports_local_styles_with_style_url() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            <Style id="testStyle">
              <IconStyle>
                  <scale>3</scale>
              </IconStyle>
            </Style>
            <Placemark>
              <styleUrl>#testStyle</styleUrl>
            </Placemark>
            </Document>"#;

    let data_source = load(kml);
    assert_eq!(data_source.entities().length(), 1);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().scale, 3.0);
}

#[test]
fn styles_supports_local_styles_with_style_url_missing_hash() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            <Style id="testStyle">
              <IconStyle>
                  <scale>3</scale>
              </IconStyle>
            </Style>
            <Placemark>
              <styleUrl>testStyle</styleUrl>
            </Placemark>
            </Document>"#;

    let data_source = load(kml);
    assert_eq!(data_source.entities().length(), 1);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().scale, 3.0);
}

#[test]
fn styles_inline_styles_take_precedence_over_shared_styles() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Document>
            <Style id="testStyle">
              <IconStyle>
                  <scale>3</scale>
                  <Icon>
                    <href>http://test.invalid</href>
                  </Icon>
              </IconStyle>
            </Style>
            <Placemark>
              <styleUrl>#testStyle</styleUrl>
              <Style>
                <IconStyle>
                  <scale>2</scale>
                  <heading>4</heading>
                </IconStyle>
              </Style>
            </Placemark>
            </Document>"#;

    let data_source = load(kml);
    assert_eq!(data_source.entities().length(), 1);

    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().expect("billboard defined");
    assert_eq!(billboard.scale, 2.0);
    assert_eq!(billboard.rotation, (-4.0f64).to_radians());
    assert_eq!(billboard.image.as_deref(), Some("http://test.invalid/"));
}

#[test]
fn styles_empty_color() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Placemark>
              <Style>
                  <IconStyle>
                      <color></color>
                  </IconStyle>
              </Style>
            </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(entity.billboard.as_ref().unwrap().color.is_none());
}

#[test]
fn styles_applies_expected_styles_to_point_geometry() {
    let kml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <Placemark>{}
            <name>TheName</name>
            <Point>
              <altitudeMode>absolute</altitudeMode>
              <coordinates>1,2,3</coordinates>
            </Point>
          </Placemark>
        </Document>"#,
        UBER_STYLE
    );

    let data_source = load(&kml);
    let entity = first_entity(&data_source);

    let label = entity.label.as_ref().expect("label defined");
    assert_eq!(label.text.as_deref(), Some("TheName"));
    assert_eq!(label.fill_color, Color::from_bytes(0xee, 0xee, 0xee, 0xee));
    assert_eq!(label.scale, 4.0);

    let billboard = entity.billboard.as_ref().expect("billboard defined");
    assert_eq!(billboard.color, Some(Color::from_bytes(0xdd, 0xdd, 0xdd, 0xdd)));
    assert_eq!(billboard.scale, 3.0);
    assert_eq!(billboard.rotation, (-45.0f64).to_radians());
    // DEVIATION: no sourceUri in this port's synchronous load, so the
    // relative href is kept as-is instead of being page-relative.
    assert_eq!(billboard.image.as_deref(), Some("test.png"));
    assert_eq!(billboard.pixel_offset, Some((45.0, -42.0)));

    assert!(entity.polyline.is_none());
}

#[test]
fn styles_applies_expected_styles_to_line_string_geometry() {
    let kml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <Placemark>{}
            <name>TheName</name>
            <LineString>
            <coordinates>1,2,3 4,5,6</coordinates>
            </LineString>
          </Placemark>
        </Document>"#,
        UBER_STYLE
    );

    let data_source = load(&kml);
    let entity = first_entity(&data_source);

    let polyline = entity.polyline.as_ref().expect("polyline defined");
    assert_eq!(polyline.material_color, Color::from_bytes(0xaa, 0xaa, 0xaa, 0xaa));
    assert_eq!(polyline.width, 2.0);

    assert!(entity.label.is_none());
}

#[test]
fn styles_applies_expected_styles_to_polygon_geometry() {
    let kml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <Placemark>{}
          <Polygon>
            <extrude>1</extrude>
            <altitudeMode>absolute</altitudeMode>
              <outerBoundaryIs>
                <LinearRing>
                  <coordinates>
                    1,2,3
                    4,5,6
                    7,8,9
                   </coordinates>
                </LinearRing>
              </outerBoundaryIs>
            </Polygon>
            </Placemark>
        </Document>"#,
        UBER_STYLE
    );

    let data_source = load(&kml);
    let entity = first_entity(&data_source);

    let polygon = entity.polygon.as_ref().expect("polygon defined");
    assert_eq!(polygon.material_color, Color::from_bytes(0xcc, 0xcc, 0xcc, 0xcc));
    assert!(!polygon.fill);
    assert!(!polygon.outline);
    assert_eq!(polygon.outline_color, Color::from_bytes(0xaa, 0xaa, 0xaa, 0xaa));
    assert_eq!(polygon.outline_width, 2.0);

    assert!(entity.label.is_none());
}

#[test]
fn styles_applies_local_style_map() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <Placemark>
            <StyleMap>
              <Pair>
                <key>normal</key>
                <Style>
                  <IconStyle>
                    <scale>2</scale>
                  </IconStyle>
                </Style>
              </Pair>
            </StyleMap>
          </Placemark>
        </Document>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().scale, 2.0);
}

#[test]
fn styles_applies_normal_style_url_style_map() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <StyleMap id="styleMapExample">
            <Pair>
              <key>normal</key>
              <Style>
                <IconStyle>
                  <scale>2</scale>
                </IconStyle>
              </Style>
            </Pair>
          </StyleMap>
          <Placemark>
            <styleUrl>#styleMapExample</styleUrl>
          </Placemark>
        </Document>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().scale, 2.0);
}

#[test]
fn styles_applies_normal_style_map_containing_style_url() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <Style id="normalStyle">
            <IconStyle>
              <scale>2</scale>
            </IconStyle>
          </Style>
          <StyleMap id="styleMapExample">
            <Pair>
              <key>normal</key>
              <styleUrl>#normalStyle</styleUrl>
            </Pair>
          </StyleMap>
          <Placemark>
            <styleUrl>#styleMapExample</styleUrl>
            </Placemark>
        </Document>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().scale, 2.0);
}

#[test]
fn styles_applies_normal_style_map_containing_style_url_without_hash() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Document>
          <Style id="normalStyle">
            <IconStyle>
              <scale>2</scale>
            </IconStyle>
          </Style>
          <StyleMap id="styleMapExample">
            <Pair>
              <key>normal</key>
              <styleUrl>normalStyle</styleUrl>
            </Pair>
          </StyleMap>
          <Placemark>
            <styleUrl>#styleMapExample</styleUrl>
            </Placemark>
        </Document>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().scale, 2.0);
}

// ============================================================================
// IconStyle specs
// ============================================================================

#[test]
fn icon_style_handles_empty_element() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <IconStyle>
              </IconStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(entity.billboard.is_some());
}

#[test]
fn icon_style_sets_billboard_image_absolute_path() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
              <Style>
                  <IconStyle>
                      <Icon>
                          <href>http://test.invalid/image.png</href>
                      </Icon>
                  </IconStyle>
              </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(billboard.image.as_deref(), Some("http://test.invalid/image.png"));
}

#[test]
fn icon_style_sets_billboard_with_root_url() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
              <Style>
                  <IconStyle>
                      <Icon>
                          <href>root://icons/palette-3</href>
                      </Icon>
                  </IconStyle>
              </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(
        billboard.image.as_deref(),
        Some("https://maps.google.com/mapfiles/kml/pal3/icon56.png")
    );
}

#[test]
fn icon_style_sets_billboard_image_relative_path() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
              <Style>
                  <IconStyle>
                      <Icon>
                          <href>image.png</href>
                      </Icon>
                  </IconStyle>
              </Style>
          </Placemark>"#;

    let mut options = KmlLoadOptions::default();
    options.source_uri = Some("http://test.invalid".to_string());
    let data_source = load_with(kml, &options);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(billboard.image.as_deref(), Some("http://test.invalid/image.png"));
}

#[test]
fn icon_style_sets_billboard_image_with_subregion() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <IconStyle>
                <Icon>
                  <href>whiteShapes.png</href>
                  <gx:x>49</gx:x>
                  <gx:y>43</gx:y>
                  <gx:w>18</gx:w>
                  <gx:h>18</gx:h>
                </Icon>
              </IconStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(billboard.image.as_deref(), Some("whiteShapes.png"));
    assert_eq!(billboard.image_sub_region, Some((49.0, 43.0, 18.0, 18.0)));
}

#[test]
fn icon_style_sets_billboard_image_with_hot_spot_fractions() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
                  <Placemark>
                    <Style>
                      <IconStyle>
                        <hotSpot x="0.25" y="0.75" xunits="fraction" yunits="fraction"/>
                      </IconStyle>
                    </Style>
                  </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(billboard.pixel_offset, Some((8.0, 8.0)));
}

#[test]
fn icon_style_sets_billboard_image_with_hot_spot_pixels() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
                  <Placemark>
                    <Style>
                      <IconStyle>
                        <hotSpot x="1" y="2" xunits="pixels" yunits="pixels"/>
                      </IconStyle>
                    </Style>
                  </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(billboard.pixel_offset, Some((15.0, -14.0)));
}

#[test]
fn icon_style_sets_billboard_image_with_hot_spot_inset_pixels() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
                  <Placemark>
                    <Style>
                      <IconStyle>
                        <hotSpot x="1" y="2" xunits="insetPixels" yunits="insetPixels"/>
                      </IconStyle>
                    </Style>
                  </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(billboard.pixel_offset, Some((-15.0, 14.0)));
}

#[test]
fn icon_style_sets_color() {
    let color = Color::from_bytes(0xcc, 0xdd, 0xee, 0xff);
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <IconStyle>
                <color>ffeeddcc</color>
              </IconStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().color, Some(color));
}

#[test]
fn icon_style_sets_scale() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Placemark>
            <Style>
              <IconStyle>
                <scale>2.2</scale>
              </IconStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.billboard.as_ref().unwrap().scale, 2.2);
}

#[test]
fn icon_style_sets_heading() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Placemark>
            <Style>
              <IconStyle>
                <heading>4</heading>
              </IconStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let billboard = entity.billboard.as_ref().unwrap();
    assert_eq!(billboard.rotation, (-4.0f64).to_radians());
    assert_eq!(billboard.aligned_axis, Some(Cartesian3::UNIT_Z));
}

// ============================================================================
// LabelStyle specs
// ============================================================================

#[test]
fn label_style_sets_defaults() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <LabelStyle>
              </LabelStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let label = entity.label.as_ref().expect("label defined");

    assert_eq!(label.font.as_deref(), Some("16px sans-serif"));
    assert_eq!(label.style, LabelStyle::FillAndOutline as i32);
    assert_eq!(label.horizontal_origin, HorizontalOrigin::Left as i32);
    assert_eq!(label.pixel_offset, Some((17.0, 0.0)));
    assert_eq!(
        label.translucency_by_distance,
        Some(NearFarScalar::new(3000000.0, 1.0, 5000000.0, 0.0))
    );
}

#[test]
fn label_style_sets_color() {
    let color = Color::from_bytes(0xcc, 0xdd, 0xee, 0xff);
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <LabelStyle>
                <color>ffeeddcc</color>
              </LabelStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.label.as_ref().unwrap().fill_color, color);
}

#[test]
fn label_style_sets_scale() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Placemark>
              <Style>
                <IconStyle>
                    <scale>2</scale>
                </IconStyle>
                <LabelStyle>
                </LabelStyle>
              </Style>
            </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.label.as_ref().unwrap().pixel_offset, Some((33.0, 0.0)));
}

#[test]
fn label_style_sets_pixel_offset_when_billboard_scaled() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <IconStyle>
                <scale>3</scale>
              </IconStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(
        entity.label.as_ref().unwrap().pixel_offset,
        Some((3.0 * 16.0 + 1.0, 0.0))
    );
}

#[test]
fn label_style_clears_pixel_offset_when_billboard_scale_is_zero() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <IconStyle>
                <scale>0</scale>
              </IconStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    // DEVIATION: the JS spec also asserts `horizontalOrigin` is undefined;
    // the scalar value model keeps the default origin instead.
    assert!(entity.label.as_ref().unwrap().pixel_offset.is_none());
}

// ============================================================================
// LineStyle / PolyStyle specs
// ============================================================================

#[test]
fn line_style_sets_defaults() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <LineStyle>
              </LineStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let polyline = entity.polyline.as_ref().expect("polyline defined");
    // DEVIATION: the JS spec asserts the untouched properties are
    // undefined; the value model keeps defaults, so only the existence of
    // the graphics and the empty positions are mirrored.
    assert!(polyline.positions.is_empty());
}

#[test]
fn line_style_sets_color() {
    let color = Color::from_bytes(0xcc, 0xdd, 0xee, 0xff);
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <LineStyle>
                <color>ffeeddcc</color>
              </LineStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.polyline.as_ref().unwrap().material_color, color);
}

#[test]
fn line_style_sets_width() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <LineStyle>
                <width>2.75</width>
              </LineStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.polyline.as_ref().unwrap().width, 2.75);
}

#[test]
fn poly_style_sets_defaults() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <PolyStyle>
              </PolyStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    let polygon = entity.polygon.as_ref().expect("polygon defined");
    assert!(polygon.outline);
    assert_eq!(polygon.outline_color, Color::WHITE);
}

#[test]
fn poly_style_sets_color() {
    let color = Color::from_bytes(0xcc, 0xdd, 0xee, 0xff);
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <PolyStyle>
                <color>ffeeddcc</color>
              </PolyStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert_eq!(entity.polygon.as_ref().unwrap().material_color, color);
}

#[test]
fn poly_style_sets_fill() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <PolyStyle>
                <fill>0</fill>
              </PolyStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(!entity.polygon.as_ref().unwrap().fill);
}

#[test]
fn poly_style_sets_outline() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Style>
              <PolyStyle>
                <outline>0</outline>
              </PolyStyle>
            </Style>
          </Placemark>"#;

    let data_source = load(kml);
    let entity = first_entity(&data_source);
    assert!(!entity.polygon.as_ref().unwrap().outline);
}

// ============================================================================
// Folder specs
// ============================================================================

#[test]
fn folder_sets_parent_property() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Folder id="parent">
            <Placemark id="child">
            </Placemark>
        </Folder>"#;

    let data_source = load(kml);
    let entities = data_source.entities();
    let folder = entities.get_by_id("parent").expect("folder exists");
    let placemark = entities.get_by_id("child").expect("placemark exists");

    assert_eq!(entities.length(), 2);
    assert_eq!(placemark.parent_id.as_deref(), Some(folder.id.as_str()));
}

#[test]
fn folder_timespan_for_folder() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Folder>
            <Placemark id="child">
            </Placemark>
            <TimeSpan>
              <begin>2000-01-01</begin>
              <end>2000-01-03</end>
            </TimeSpan>
          </Folder>"#;

    let start = JulianDate::from_iso8601("2000-01-01").unwrap();
    let stop = JulianDate::from_iso8601("2000-01-03").unwrap();

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 2);
    let folder = &entities[0];
    assert_eq!(folder.availability.len(), 1);
    assert!(JulianDate::equals(&folder.availability[0].start, &start));
    assert!(JulianDate::equals(&folder.availability[0].stop, &stop));

    let child = &entities[1];
    assert_eq!(child.availability.len(), folder.availability.len());
    assert!(JulianDate::equals(
        &child.availability[0].start,
        &folder.availability[0].start
    ));
    assert!(JulianDate::equals(
        &child.availability[0].stop,
        &folder.availability[0].stop
    ));
}

#[test]
fn folder_timespan_for_folder_and_feature() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Folder>
            <Placemark id="child">
                <TimeSpan>
                  <begin>2000-01-02</begin>
                  <end>2000-01-03</end>
                </TimeSpan>
            </Placemark>
            <TimeSpan>
              <begin>2000-01-01</begin>
              <end>2000-01-04</end>
            </TimeSpan>
          </Folder>"#;

    let start_folder = JulianDate::from_iso8601("2000-01-01").unwrap();
    let stop_folder = JulianDate::from_iso8601("2000-01-04").unwrap();
    let start_feature = JulianDate::from_iso8601("2000-01-02").unwrap();
    let stop_feature = JulianDate::from_iso8601("2000-01-03").unwrap();

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 2);

    let folder = &entities[0];
    assert!(JulianDate::equals(&folder.availability[0].start, &start_folder));
    assert!(JulianDate::equals(&folder.availability[0].stop, &stop_folder));

    let child = &entities[1];
    assert!(JulianDate::equals(&child.availability[0].start, &start_feature));
    assert!(JulianDate::equals(&child.availability[0].stop, &stop_feature));
}

#[test]
fn folder_timestamp_for_folder() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Folder>
            <Placemark id="child">
            </Placemark>
            <TimeStamp>
              <when>2000-01-03</when>
            </TimeStamp>
          </Folder>"#;

    let start = JulianDate::from_iso8601("2000-01-03").unwrap();

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 2);
    let folder = &entities[0];
    assert!(JulianDate::equals(&folder.availability[0].start, &start));
    assert!(JulianDate::equals(
        &folder.availability[0].stop,
        Iso8601::maximum_value()
    ));

    let child = &entities[1];
    assert_eq!(child.availability.len(), folder.availability.len());
    assert!(JulianDate::equals(
        &child.availability[0].start,
        &folder.availability[0].start
    ));
    assert!(JulianDate::equals(
        &child.availability[0].stop,
        &folder.availability[0].stop
    ));
}

#[test]
fn folder_timestamp_for_folder_and_feature() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Folder>
            <Placemark id="child">
                <TimeSpan>
                  <begin>2000-01-04</begin>
                  <end>2000-01-05</end>
                </TimeSpan>
            </Placemark>
            <TimeStamp>
              <when>2000-01-03</when>
            </TimeStamp>
          </Folder>"#;

    let start_folder = JulianDate::from_iso8601("2000-01-03").unwrap();
    let start_feature = JulianDate::from_iso8601("2000-01-04").unwrap();
    let stop_feature = JulianDate::from_iso8601("2000-01-05").unwrap();

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 2);

    let folder = &entities[0];
    assert!(JulianDate::equals(&folder.availability[0].start, &start_folder));
    assert!(JulianDate::equals(
        &folder.availability[0].stop,
        Iso8601::maximum_value()
    ));

    let child = &entities[1];
    assert!(JulianDate::equals(&child.availability[0].start, &start_feature));
    assert!(JulianDate::equals(&child.availability[0].stop, &stop_feature));
}

// ============================================================================
// Geometry: Point specs
// ============================================================================

#[test]
fn geometry_point_handles_empty_point() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::from_degrees_new(0.0, 0.0, Some(0.0), None);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_handles_invalid_coordinates() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
            <altitudeMode>absolute</altitudeMode>
            <coordinates>1,2,3,4</coordinates>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::from_degrees_new(1.0, 2.0, Some(3.0), None);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_handles_empty_coordinates() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
            <coordinates></coordinates>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::from_degrees_new(0.0, 0.0, Some(0.0), None);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_sets_height_reference_to_clamp_to_ground() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
              <coordinates>1,2,3</coordinates>
            </Point>
          </Placemark>"#;

    let options = KmlLoadOptions {
        source_uri: None,
        clamp_to_ground: true,
        credit: None,
    };
    let data_source = load_with(kml, &options);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let billboard = entities[0].billboard.as_ref().unwrap();
    assert_eq!(billboard.height_reference, HeightReference::ClampToGround as i32);
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_sets_position_altitude_mode_absolute() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
              <altitudeMode>absolute</altitudeMode>
              <coordinates>1,2,3</coordinates>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::from_degrees_new(1.0, 2.0, Some(3.0), None);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_sets_position_altitude_mode_relative_to_ground() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
              <altitudeMode>relativeToGround</altitudeMode>
              <coordinates>1,2,3</coordinates>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::from_degrees_new(1.0, 2.0, Some(3.0), None);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_does_not_extrude_when_altitude_mode_is_clamp_to_ground() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
              <altitudeMode>clampToGround</altitudeMode>
              <coordinates>1,2</coordinates>
              <extrude>1</extrude>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::from_degrees_new(1.0, 2.0, Some(0.0), None);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_does_not_extrude_when_gx_altitude_mode_is_clamp_to_sea_floor() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark xmlns="http://www.opengis.net/kml/2.2"
                     xmlns:gx="http://www.google.com/kml/ext/2.2">
            <Point>
              <gx:altitudeMode>clampToSeaFloor</gx:altitudeMode>
              <coordinates>1,2</coordinates>
              <extrude>1</extrude>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::from_degrees_new(1.0, 2.0, Some(0.0), None);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
    assert!(entities[0].polyline.is_none());
}

#[test]
fn geometry_point_correctly_converts_coordinates_using_earth_ellipsoid() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Point>
              <coordinates>24.070617695061806,87.90173269295278,0</coordinates>
            </Point>
          </Placemark>"#;

    let data_source = load(kml);
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let expected = Cartesian3::new(213935.5635247161, 95566.36983235707, 6352461.425213023);
    assert!(position_equals(
        entities[0].position.as_ref().unwrap(),
        &expected
    ));
}

// ============================================================================
// Geometry: Polygon specs
// ============================================================================

#[test]
fn geometry_polygon_handles_empty_coordinates() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Polygon>
              <outerBoundaryIs>
                <LinearRing>
                  <coordinates>
                 </coordinates>
                </LinearRing>
              </outerBoundaryIs>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    // DEVIATION: the undefined hierarchy is mirrored as an empty one.
    assert!(entity.polygon.as_ref().unwrap().hierarchy.is_empty());
}

#[test]
fn geometry_polygon_without_holes() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Polygon>
              <outerBoundaryIs>
                <LinearRing>
                  <coordinates>
                    1,2,3
                    4,5,6
                    7,8,9
                 </coordinates>
                </LinearRing>
              </outerBoundaryIs>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    let expected = vec![
        Cartesian3::from_degrees_new(1.0, 2.0, Some(3.0), None),
        Cartesian3::from_degrees_new(4.0, 5.0, Some(6.0), None),
        Cartesian3::from_degrees_new(7.0, 8.0, Some(9.0), None),
    ];
    assert_eq!(polygon.hierarchy.len(), expected.len());
    for (left, right) in polygon.hierarchy.iter().zip(expected.iter()) {
        assert!(position_equals(left, right));
    }
}

#[test]
fn geometry_polygon_with_holes() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Polygon>
            <outerBoundaryIs>
            <LinearRing>
              <coordinates>
                1,2,3
                4,5,6
                7,8,9
             </coordinates>
            </LinearRing>
            </outerBoundaryIs>
            <innerBoundaryIs>
            <LinearRing>
              <coordinates>
                1.1,2.1,3.1
                4.1,5.1,6.1
                7.1,8.1,9.1
             </coordinates>
            </LinearRing>
            </innerBoundaryIs>
            <innerBoundaryIs>
            <LinearRing>
              <coordinates>
                1.2,2.2,3.2
                4.2,5.2,6.2
                7.2,8.2,9.2
             </coordinates>
            </LinearRing>
            </innerBoundaryIs>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();

    let expected = vec![
        Cartesian3::from_degrees_new(1.0, 2.0, Some(3.0), None),
        Cartesian3::from_degrees_new(4.0, 5.0, Some(6.0), None),
        Cartesian3::from_degrees_new(7.0, 8.0, Some(9.0), None),
    ];
    let hole_one = vec![
        Cartesian3::from_degrees_new(1.1, 2.1, Some(3.1), None),
        Cartesian3::from_degrees_new(4.1, 5.1, Some(6.1), None),
        Cartesian3::from_degrees_new(7.1, 8.1, Some(9.1), None),
    ];
    let hole_two = vec![
        Cartesian3::from_degrees_new(1.2, 2.2, Some(3.2), None),
        Cartesian3::from_degrees_new(4.2, 5.2, Some(6.2), None),
        Cartesian3::from_degrees_new(7.2, 8.2, Some(9.2), None),
    ];

    assert_eq!(polygon.hierarchy.len(), expected.len());
    for (left, right) in polygon.hierarchy.iter().zip(expected.iter()) {
        assert!(position_equals(left, right));
    }
    assert_eq!(polygon.holes.len(), 2);
    assert_eq!(polygon.holes[0].len(), hole_one.len());
    for (left, right) in polygon.holes[0].iter().zip(hole_one.iter()) {
        assert!(position_equals(left, right));
    }
    assert_eq!(polygon.holes[1].len(), hole_two.len());
    for (left, right) in polygon.holes[1].iter().zip(hole_two.iter()) {
        assert!(position_equals(left, right));
    }
}

#[test]
fn geometry_polygon_altitude_mode_relative_to_ground_and_can_extrude() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Polygon>
              <altitudeMode>relativeToGround</altitudeMode>
              <extrude>1</extrude>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    assert_eq!(polygon.per_position_height, Some(true));
    assert_eq!(polygon.extruded_height, Some(0.0));
}

#[test]
fn geometry_polygon_altitude_mode_absolute_and_can_extrude() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Polygon>
              <altitudeMode>absolute</altitudeMode>
              <extrude>1</extrude>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    assert_eq!(polygon.per_position_height, Some(true));
    assert_eq!(polygon.extruded_height, Some(0.0));
}

#[test]
fn geometry_polygon_altitude_mode_clamp_to_ground_and_cannot_extrude() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark xmlns="http://www.opengis.net/kml/2.2"
                     xmlns:gx="http://www.google.com/kml/ext/2.2">
            <Polygon>
              <altitudeMode>clampToGround</altitudeMode>
              <extrude>1</extrude>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    assert_eq!(polygon.per_position_height, None);
    assert_eq!(polygon.extruded_height, None);
}

#[test]
fn geometry_polygon_gx_altitude_mode_relative_to_sea_floor_and_can_extrude() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark xmlns="http://www.opengis.net/kml/2.2"
                     xmlns:gx="http://www.google.com/kml/ext/2.2">
            <Polygon>
              <gx:altitudeMode>relativeToSeaFloor</gx:altitudeMode>
              <extrude>1</extrude>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    assert_eq!(polygon.per_position_height, Some(true));
    assert_eq!(polygon.extruded_height, Some(0.0));
}

#[test]
fn geometry_polygon_gx_altitude_mode_clamp_to_sea_floor_and_can_extrude() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark xmlns="http://www.opengis.net/kml/2.2"
                     xmlns:gx="http://www.google.com/kml/ext/2.2">
            <Polygon>
              <gx:altitudeMode>clampToSeaFloor</gx:altitudeMode>
              <extrude>1</extrude>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    assert_eq!(polygon.per_position_height, None);
    assert_eq!(polygon.extruded_height, None);
}

#[test]
fn when_clamp_to_ground_is_false_height_is_not_set_if_the_polygon_is_extrudable() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Polygon>
              <altitudeMode>relativeToGround</altitudeMode>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    assert_eq!(polygon.per_position_height, Some(true));
    assert_eq!(polygon.height, None);
}

#[test]
fn when_clamp_to_ground_is_false_height_is_set_to_zero_if_polygon_is_not_extrudable() {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <Placemark>
            <Polygon>
              <altitudeMode>clampToGround</altitudeMode>
            </Polygon>
          </Placemark>"#;

    let binding = load(kml);
    let entity = first_entity(&binding);
    let polygon = entity.polygon.as_ref().unwrap();
    assert_eq!(polygon.per_position_height, None);
    assert_eq!(polygon.height, Some(0.0));
}

#[test]
fn when_a_line_string_is_clamped_to_ground_and_tesselated_entity_has_a_polyline_geometry_and_color_property(
) {
    let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <Placemark>
                <Style>
                    <LineStyle>
                        <color>FFFF0000</color>
                    </LineStyle>
                </Style>
                <LineString>
                    <altitudeMode>clampToGround</altitudeMode>
                    <tessellate>true</tessellate>
                    <coordinates>1,2,3
                                4,5,6
                    </coordinates>
                </LineString>
            </Placemark>"#;

    let options = KmlLoadOptions {
        source_uri: None,
        clamp_to_ground: true,
        credit: None,
    };
    let binding = load_with(kml, &options);
    let entity = first_entity(&binding);
    let polyline = entity.polyline.as_ref().unwrap();
    assert!(polyline.clamp_to_ground);
    assert_eq!(polyline.material_color, Color::from_bytes(0, 0, 255, 255));
}
