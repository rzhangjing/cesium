//! Ported specs from `packages/engine/Specs/DataSources/GpxDataSourceSpec.js`.
//!
//! Every test mirrors one `it()` of the original Jasmine spec; the test
//! names keep the original descriptions snake-cased so they stay mappable.
//! Assertions that depend on browser-only facilities (DOM description
//! parsing, promise timing, image element objects) are adapted as
//! documented in the individual DEVIATION comments.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::julian_date::JulianDate;
use cesium_data_sources::data_source::DataSource;
use cesium_data_sources::gpx_data_source::{GpxDataSource, GpxLoadOptions};
use cesium_scene::height_reference::HeightReference;
use cesium_scene::vertical_origin::VerticalOrigin;
use cesium_specs::data_path;

// Mirror of the spec helper building GPX documents around a body.
fn gpx_doc(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"andre\">{}</gpx>",
        body
    )
}

fn assert_position_eq(actual: &Cartesian3, expected: &Cartesian3) {
    assert!(
        (actual.x - expected.x).abs() < 1e-6
            && (actual.y - expected.y).abs() < 1e-6
            && (actual.z - expected.z).abs() < 1e-6,
        "position mismatch: ({}, {}, {}) != ({}, {}, {})",
        actual.x,
        actual.y,
        actual.z,
        expected.x,
        expected.y,
        expected.z
    );
}

#[test]
fn default_constructor_has_expected_values() {
    let data_source = GpxDataSource::new();
    // DEVIATION: the JS `name` is undefined; the trait accessor returns "".
    assert!(data_source.name().is_empty());
    assert!(data_source.clock().is_none());
    assert!(!data_source.is_loading());
    assert!(data_source.show());
    // The events exist and are usable (mirror of `toBeInstanceOf(Event)`).
    assert_eq!(data_source.changed_event().number_of_listeners(), 0);
    assert_eq!(data_source.error_event().number_of_listeners(), 0);
    assert_eq!(data_source.loading_event().number_of_listeners(), 0);
}

// DEVIATION: "load throws with undefined GPX" has no counterpart because the
// Rust signature requires a string.

#[test]
fn load_works_with_a_gpx_url() {
    let mut data_source = GpxDataSource::new();
    let path = data_path("GPX/simple.gpx");
    data_source.load_file(&path.to_string_lossy(), None).unwrap();
    assert_eq!(data_source.entities().values().len(), 1);
}

#[test]
fn load_rejects_nonexistent_url() {
    let mut data_source = GpxDataSource::new();
    assert!(data_source.load_file("test.invalid", None).is_err());
}

#[test]
fn load_rejects_loading_non_gpx() {
    // Binary garbage (the JS Blue.png case) fails to parse as XML.
    let mut data_source = GpxDataSource::new();
    assert!(data_source
        .load_blob(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], None)
        .is_err());
}

#[test]
fn sets_data_source_creator_and_version_from_gpx() {
    let gpx = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"Test\">\
        </gpx>";
    let data_source = GpxDataSource::load(gpx, None).unwrap();
    assert_eq!(data_source.version(), Some("1.1"));
    assert_eq!(data_source.creator(), Some("Test"));
}

#[test]
fn sets_data_source_name_from_metadata() {
    let gpx = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"Test\">\
        <metadata>\
            <name>File Name</name>\
        </metadata>\
        </gpx>";
    let data_source = GpxDataSource::load(gpx, None).unwrap();
    assert_eq!(data_source.name(), "File Name");
}

#[test]
fn sets_data_source_metadata_object_correctly() {
    let gpx = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"Test\">\
        <metadata>\
            <name>The name</name>\
            <desc>The desc</desc>\
            <time>The time</time>\
            <keywords>The keyword</keywords>\
        </metadata>\
        </gpx>";
    let data_source = GpxDataSource::load(gpx, None).unwrap();
    let metadata = data_source.metadata().expect("metadata should be defined");
    assert_eq!(metadata.name.as_deref(), Some("The name"));
    assert_eq!(metadata.desc.as_deref(), Some("The desc"));
    assert_eq!(metadata.time.as_deref(), Some("The time"));
    assert_eq!(metadata.keywords.as_deref(), Some("The keyword"));
}

#[test]
fn metadata_handles_person_type_email_type_and_link_type() {
    let gpx = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"Test\">\
        <metadata>\
            <author>\
                <name>The name</name>\
                <email>\
                    <id>user</id>\
                    <domain>email.com</domain>\
                </email>\
                <link href=\"www.a.com\">\
                    <text>A website</text>\
                    <type>text/html</type>\
                </link>\
            </author>\
        </metadata>\
        </gpx>";
    let data_source = GpxDataSource::load(gpx, None).unwrap();
    let metadata = data_source.metadata().expect("metadata should be defined");

    let person = metadata.author.as_ref().expect("author should be defined");
    assert_eq!(person.name.as_deref(), Some("The name"));
    assert_eq!(person.email.as_deref(), Some("user@email.com"));
    let link = person.link.as_ref().expect("link should be defined");
    assert_eq!(link.href.as_deref(), Some("www.a.com"));
    assert_eq!(link.text.as_deref(), Some("A website"));
    assert_eq!(link.mime_type.as_deref(), Some("text/html"));
}

#[test]
fn metadata_handles_copyright_type() {
    let gpx = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"Test\">\
        <metadata>\
            <copyright author=\"The author\">\
                <year>2015</year>\
                <license>The license</license>\
            </copyright>\
        </metadata>\
        </gpx>";
    let data_source = GpxDataSource::load(gpx, None).unwrap();
    let metadata = data_source.metadata().expect("metadata should be defined");

    let copyright = metadata
        .copyright
        .as_ref()
        .expect("copyright should be defined");
    assert_eq!(copyright.author.as_deref(), Some("The author"));
    assert_eq!(copyright.year.as_deref(), Some("2015"));
    assert_eq!(copyright.license.as_deref(), Some("The license"));
}

#[test]
fn metadata_handles_bounds_type() {
    // DEVIATION (faithful mirror): the JS spec writes the bounds as child
    // elements (<minlat>...) instead of the schema attributes, and the JS
    // reader (`queryNumericValue`) reads children; the port mirrors both.
    let gpx = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"Test\">\
        <metadata>\
            <bounds>\
                <minlat>1</minlat>\
                <maxlat>2</maxlat>\
                <minlon>3</minlon>\
                <maxlon>4</maxlon>\
            </bounds>\
        </metadata>\
        </gpx>";
    let data_source = GpxDataSource::load(gpx, None).unwrap();
    let metadata = data_source.metadata().expect("metadata should be defined");

    let bounds = metadata.bounds.as_ref().expect("bounds should be defined");
    assert_eq!(bounds.min_lat, Some(1.0));
    assert_eq!(bounds.max_lat, Some(2.0));
    assert_eq!(bounds.min_lon, Some(3.0));
    assert_eq!(bounds.max_lon, Some(4.0));
}

#[test]
fn raises_changed_event_when_the_name_changes() {
    let gpx = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <gpx xmlns=\"http://www.topografix.com/GPX/1/1\" version=\"1.1\" creator=\"Test\">\
        <metadata>\
            <name>NameInGpx</name>\
        </metadata>\
        </gpx>";

    let mut data_source = GpxDataSource::new();

    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .changed_event()
        .add_listener(move |_: &()| spy.set(spy.get() + 1));

    // Initial load
    data_source.load_value(gpx, None).unwrap();
    assert_eq!(count.get(), 1);

    // Loading GPX with same name
    data_source.load_value(gpx, None).unwrap();
    assert_eq!(count.get(), 1);

    // Loading GPX with different name.
    let renamed = gpx.replace("NameInGpx", "newName");
    data_source.load_value(&renamed, None).unwrap();
    assert_eq!(count.get(), 2);
}

#[test]
fn raises_loading_event_event_at_start_and_end_of_load() {
    let mut data_source = GpxDataSource::new();

    // DEVIATION: the Rust event carries no payload, so the spec's
    // (dataSource, isLoading) arguments are reduced to firing counts: the
    // event must fire once at the start and once at the end of the load.
    let count = Rc::new(Cell::new(0u32));
    let spy = count.clone();
    let _remove = data_source
        .loading_event()
        .add_listener(move |_: &()| spy.set(spy.get() + 1));

    let path = data_path("GPX/simple.gpx");
    data_source.load_file(&path.to_string_lossy(), None).unwrap();
    assert_eq!(count.get(), 2);
}

#[test]
fn waypoint_sets_name() {
    let gpx = gpx_doc("<wpt lat=\"1\" lon=\"2\"><name>Test</name></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    assert_eq!(entity.name.as_deref(), Some("Test"));
    let label = entity.label.as_ref().expect("label should be defined");
    assert_eq!(label.text.as_deref(), Some("Test"));
}

#[test]
fn waypoint_throws_with_invalid_coordinates() {
    let gpx = gpx_doc("<wpt lat=\"hello\" lon=\"world\"></wpt>");
    assert!(GpxDataSource::load(&gpx, None).is_err());
}

#[test]
fn waypoint_throws_when_no_coordinates_are_given() {
    let gpx = gpx_doc("<wpt></wpt>");
    assert!(GpxDataSource::load(&gpx, None).is_err());
}

#[test]
fn waypoint_handles_simple_waypoint() {
    let gpx = gpx_doc("<wpt lon=\"38.737125\" lat=\"-9.139242\"><name>Position 1</name></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    assert_position_eq(
        entities[0].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(38.737125, -9.139242, None, None),
    );
    assert_eq!(entities[0].name.as_deref(), Some("Position 1"));
}

#[test]
fn waypoint_uses_default_billboard_style() {
    const BILLBOARD_SIZE: f64 = 32.0;
    let gpx = gpx_doc("<wpt lon=\"38.737125\" lat=\"-9.139242\"><name>Position 1</name></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    let billboard = entities[0]
        .billboard
        .as_ref()
        .expect("billboard should be defined");
    assert_eq!(billboard.height, Some(BILLBOARD_SIZE));
    assert_eq!(billboard.width, Some(BILLBOARD_SIZE));
    assert_eq!(
        billboard.vertical_origin,
        VerticalOrigin::Bottom as i32
    );
    assert_eq!(billboard.height_reference, HeightReference::None as i32);
}

#[test]
fn waypoint_uses_clamp_to_ground_billboards() {
    let gpx = gpx_doc("<wpt lon=\"38.737125\" lat=\"-9.139242\"><name>Position 1</name></wpt>");
    let options = GpxLoadOptions {
        clamp_to_ground: true,
        ..Default::default()
    };
    let data_source = GpxDataSource::load(&gpx, Some(&options)).unwrap();
    let entities = data_source.entities().values();
    let billboard = entities[0]
        .billboard
        .as_ref()
        .expect("billboard should be defined");
    assert_eq!(
        billboard.height_reference,
        HeightReference::ClampToGround as i32
    );
}

#[test]
fn waypoint_uses_custom_image_for_billboard() {
    let gpx = gpx_doc("<wpt lon=\"38.737125\" lat=\"-9.139242\"><name>Position 1</name></wpt>");
    // DEVIATION: the JS spec passes an HTML image element; the value model
    // stores the image as a string.
    let options = GpxLoadOptions {
        clamp_to_ground: true,
        waypoint_image: Some(String::from("wpt")),
        ..Default::default()
    };
    let data_source = GpxDataSource::load(&gpx, Some(&options)).unwrap();
    let entities = data_source.entities().values();
    let billboard = entities[0]
        .billboard
        .as_ref()
        .expect("billboard should be defined");
    assert_eq!(billboard.image.as_deref(), Some("wpt"));
}

#[test]
fn waypoint_handles_simple_waypoint_with_elevation() {
    let gpx = gpx_doc("<wpt lon=\"1\" lat=\"2\"><ele>3</ele><name>Position 1</name></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    assert_position_eq(
        entities[0].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(1.0, 2.0, Some(3.0), None),
    );
}

#[test]
fn waypoint_handles_multiple_waypoints() {
    let gpx = gpx_doc(
        "<wpt lon=\"1\" lat=\"2\"><name>Position 1</name></wpt>\
         <wpt lon=\"3\" lat=\"4\"><name>Position 2</name></wpt>\
         <wpt lon=\"5\" lat=\"6\"><name>Position 3</name></wpt>",
    );
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 3);
    assert_position_eq(
        entities[0].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(1.0, 2.0, None, None),
    );
    assert_position_eq(
        entities[1].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(3.0, 4.0, None, None),
    );
    assert_position_eq(
        entities[2].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(5.0, 6.0, None, None),
    );
}

// Mirror of the JS spec helper that checks the description wrapper div via
// DOM style parsing; the port asserts the style attributes and the entry
// text directly on the HTML string.
fn assert_description_div(description: &str, entry: &str) {
    assert!(
        description.contains("word-wrap:break-word"),
        "missing word-wrap style: {}",
        description
    );
    assert!(
        description.contains("background-color:rgb(255, 255, 255)"),
        "missing background color: {}",
        description
    );
    assert!(
        description.contains("color:rgb(0, 0, 0)"),
        "missing foreground color: {}",
        description
    );
    assert!(
        description.contains(entry),
        "missing entry {}: {}",
        entry,
        description
    );
}

#[test]
fn description_handles_desc() {
    let gpx = gpx_doc("<wpt lon=\"1\" lat=\"2\"><desc>The Description</desc></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    assert_description_div(description, "Description: The Description");
}

#[test]
fn description_handles_time() {
    let gpx = gpx_doc("<wpt lon=\"1\" lat=\"2\"><time>2015-08-17T00:06Z</time></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    assert_description_div(description, "Time: 2015-08-17T00:06Z");
}

#[test]
fn description_handles_comment() {
    let gpx = gpx_doc("<wpt lon=\"1\" lat=\"2\"><cmt>The comment</cmt></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    assert_description_div(description, "Comment: The comment");
}

#[test]
fn description_handles_source() {
    let gpx = gpx_doc("<wpt lon=\"1\" lat=\"2\"><src>The source</src></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    assert_description_div(description, "Source: The source");
}

#[test]
fn description_handles_gps_number() {
    let gpx = gpx_doc("<wpt lon=\"1\" lat=\"2\"><number>The number</number></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    assert_description_div(description, "GPS track/route number: The number");
}

#[test]
fn description_handles_type() {
    let gpx = gpx_doc("<wpt lon=\"1\" lat=\"2\"><type>The type</type></wpt>");
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    assert_description_div(description, "Type: The type");
}

#[test]
fn description_handles_multiple_fields() {
    let gpx = gpx_doc(
        "<wpt lon=\"1\" lat=\"2\"><cmt>The comment</cmt><desc>The description</desc><type>The type</type></wpt>",
    );
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    // DEVIATION: the JS asserts the concatenated textContent of the three
    // paragraphs; the port asserts each entry text individually.
    assert_description_div(description, "Comment: The comment");
    assert_description_div(description, "Description: The description");
    assert_description_div(description, "Type: The type");
}

#[test]
fn description_handles_route_description() {
    let gpx = gpx_doc(
        "<rte>\
            <cmt>The comment</cmt>\
            <desc>The description</desc>\
            <type>The type</type>\
            <rtept lon=\"1\" lat=\"2\"><ele>1</ele><name>Position 1</name></rtept>\
            <rtept lon=\"3\" lat=\"4\"><ele>1</ele><name>Position 2</name></rtept>\
            <rtept lon=\"5\" lat=\"6\"><ele>1</ele><name>Position 3</name></rtept>\
            <rtept lon=\"7\" lat=\"8\"><ele>1</ele><name>Position 4</name></rtept>\
        </rte>",
    );
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entity = &data_source.entities().values()[0];
    let description = entity.description.as_ref().expect("description");
    assert_description_div(description, "Comment: The comment");
    assert_description_div(description, "Description: The description");
    assert_description_div(description, "Type: The type");
}

#[test]
fn route_handles_simple_route() {
    let gpx = gpx_doc(
        "<rte>\
            <name>Test Route</name>\
            <rtept lon=\"1\" lat=\"2\"><ele>1</ele><name>Position 1</name></rtept>\
            <rtept lon=\"3\" lat=\"4\"><ele>1</ele><name>Position 2</name></rtept>\
            <rtept lon=\"5\" lat=\"6\"><ele>1</ele><name>Position 3</name></rtept>\
            <rtept lon=\"7\" lat=\"8\"><ele>1</ele><name>Position 4</name></rtept>\
        </rte>",
    );
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    // 1 for the route and 4 routepoints
    assert_eq!(entities.len(), 5);
    assert_position_eq(
        entities[1].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(1.0, 2.0, Some(1.0), None),
    );
    assert_position_eq(
        entities[2].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(3.0, 4.0, Some(1.0), None),
    );
    assert_position_eq(
        entities[3].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(5.0, 6.0, Some(1.0), None),
    );
    assert_position_eq(
        entities[4].position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(7.0, 8.0, Some(1.0), None),
    );
}

const SIMPLE_TRACK_BODY: &str = "<trk>\
    <name>Test Track</name>\
        <trkseg>\
            <trkpt lon=\"1\" lat=\"2\"><ele>1.0</ele><name>Position 1</name></trkpt>\
            <trkpt lon=\"3\" lat=\"4\"><ele>1.0</ele><name>Position 2</name></trkpt>\
        </trkseg>\
    </trk>";

#[test]
fn track_handles_simple_track() {
    let gpx = gpx_doc(SIMPLE_TRACK_BODY);
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);

    let entity = &entities[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(polyline.positions.len(), 2);
    assert_position_eq(
        &polyline.positions[0],
        &Cartesian3::from_degrees_new(1.0, 2.0, Some(1.0), None),
    );
    assert_position_eq(
        &polyline.positions[1],
        &Cartesian3::from_degrees_new(3.0, 4.0, Some(1.0), None),
    );
}

#[test]
fn track_uses_default_polyline_style() {
    let gpx = gpx_doc(SIMPLE_TRACK_BODY);
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);

    let entity = &entities[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(polyline.width, 4.0);
    assert_eq!(polyline.material_color, Color::RED);
    // DEVIATION: the JS default track material is a
    // PolylineOutlineMaterialProperty (outline width 2 / black outline);
    // the value model only keeps the primary color.
}

#[test]
fn track_uses_custom_polyline_color_for_tracks() {
    let gpx = gpx_doc(SIMPLE_TRACK_BODY);
    let options = GpxLoadOptions {
        track_color: Some(Color::BLUE),
        ..Default::default()
    };
    let data_source = GpxDataSource::load(&gpx, Some(&options)).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let entity = &entities[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(polyline.material_color, Color::BLUE);
    assert!(!polyline.clamp_to_ground);
}

#[test]
fn track_uses_custom_polyline_color_for_routes() {
    let gpx = gpx_doc(
        "<rte>\
            <rtept lon=\"9.860624216140083\" lat=\"54.9328621088893\"><ele>0.0</ele><name>Position 1</name></rtept>\
            <rtept lon=\"9.86092208681491\" lat=\"54.93293237320851\"><ele>0.0</ele><name>Position 2</name></rtept>\
            <rtept lon=\"9.86187816543752\" lat=\"54.93327743521187\"><ele>0.0</ele><name>Position 3</name></rtept>\
            <rtept lon=\"9.862439849679859\" lat=\"54.93342326167919\"><ele>0.0</ele><name>Position 4</name></rtept>\
        </rte>",
    );
    let options = GpxLoadOptions {
        route_color: Some(Color::BLUE),
        ..Default::default()
    };
    let data_source = GpxDataSource::load(&gpx, Some(&options)).unwrap();
    let entities = data_source.entities().values();
    // 4 waypoints + 1 polyline
    assert_eq!(entities.len(), 5);
    let entity = &entities[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(polyline.material_color, Color::BLUE);
    assert!(!polyline.clamp_to_ground);
}

#[test]
fn track_uses_clamp_to_ground_polylines() {
    let gpx = gpx_doc(SIMPLE_TRACK_BODY);
    let options = GpxLoadOptions {
        clamp_to_ground: true,
        ..Default::default()
    };
    let data_source = GpxDataSource::load(&gpx, Some(&options)).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);
    let entity = &entities[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert!(polyline.clamp_to_ground);
}

const TIME_DYNAMIC_TRACK_BODY: &str = "<trk>\
    <name>Test Track</name>\
        <trkseg>\
            <trkpt lon=\"1\" lat=\"2\"><ele>1.0</ele><name>Position 1</name><time>2000-01-01T00:00:00Z</time></trkpt>\
            <trkpt lon=\"3\" lat=\"4\"><ele>1.0</ele><name>Position 2</name><time>2000-01-01T00:00:01Z</time></trkpt>\
            <trkpt lon=\"5\" lat=\"6\"><ele>1.0</ele><name>Position 3</name><time>2000-01-01T00:00:02Z</time></trkpt>\
        </trkseg>\
    </trk>";

#[test]
fn track_handles_time_dynamic_track() {
    let gpx = gpx_doc(TIME_DYNAMIC_TRACK_BODY);
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let time1 = JulianDate::from_iso8601("2000-01-01T00:00:00Z").unwrap();
    let time3 = JulianDate::from_iso8601("2000-01-01T00:00:02Z").unwrap();

    let entity = &data_source.entities().values()[0];
    // DEVIATION: the JS port samples the position property per time; the
    // value model keeps the first sample as the constant position.
    assert_position_eq(
        entity.position.as_ref().unwrap(),
        &Cartesian3::from_degrees_new(1.0, 2.0, Some(1.0), None),
    );
    assert!(entity.polyline.is_some());

    assert!(!entity.availability.is_empty());
    assert!(JulianDate::equals(&entity.availability[0].start, &time1));
    assert!(JulianDate::equals(&entity.availability[0].stop, &time3));
}

#[test]
fn track_time_dynamic_uses_default_path_style() {
    let gpx = gpx_doc(TIME_DYNAMIC_TRACK_BODY);
    let data_source = GpxDataSource::load(&gpx, None).unwrap();
    let entities = data_source.entities().values();
    assert_eq!(entities.len(), 1);

    let entity = &entities[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert_eq!(polyline.width, 4.0);
    assert_eq!(polyline.material_color, Color::RED);
}

#[test]
fn track_time_dynamic_track_uses_clamp_to_ground() {
    let gpx = gpx_doc(TIME_DYNAMIC_TRACK_BODY);
    let options = GpxLoadOptions {
        clamp_to_ground: true,
        ..Default::default()
    };
    let data_source = GpxDataSource::load(&gpx, Some(&options)).unwrap();
    let entities = data_source.entities().values();

    let entity = &entities[0];
    let polyline = entity.polyline.as_ref().expect("polyline should be defined");
    assert!(polyline.clamp_to_ground);
}

#[test]
fn update_returns_true() {
    let gpx = gpx_doc(SIMPLE_TRACK_BODY);
    let options = GpxLoadOptions {
        clamp_to_ground: true,
        ..Default::default()
    };
    let data_source = GpxDataSource::load(&gpx, Some(&options)).unwrap();
    assert!(data_source.update(&JulianDate::default_date()));
}
