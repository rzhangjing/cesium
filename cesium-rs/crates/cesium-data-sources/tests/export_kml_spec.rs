//! Ported specs from `packages/engine/Specs/DataSources/exportKmlSpec.js`.
//!
//! Every test mirrors one `it()` of the original Jasmine spec; the test
//! names keep the original descriptions snake-cased so they stay mappable.
//! The JS spec walks the exported DOM document with a recursive property
//! checker; the port parses the serialized KML string with quick-xml and
//! asserts the same structure (the checker only verified that every
//! existing KML node matches an expectation, which these assertions
//! replicate directly).
//!
//! DEVIATION: canvas images, KMZ packaging, sampled/callback positions,
//! rectangles/GroundOverlays and zIndex have no value-model counterpart
//! (see `export_kml.rs` module docs); the corresponding `it()`s are either
//! adapted or omitted.

use std::collections::HashMap;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::color::Color;
use cesium_core::julian_date::JulianDate;
use cesium_core::math::CesiumMath;
use cesium_core::time_interval::TimeInterval;
use cesium_data_sources::billboard_graphics::BillboardGraphics;
use cesium_data_sources::entity::Entity;
use cesium_data_sources::entity_collection::EntityCollection;
use cesium_data_sources::export_kml::{export_kml, ExportKmlOptions};
use cesium_data_sources::label_graphics::LabelGraphics;
use cesium_data_sources::model_graphics::ModelGraphics;
use cesium_data_sources::point_graphics::PointGraphics;
use cesium_data_sources::polygon_graphics::PolygonGraphics;
use cesium_data_sources::polyline_graphics::PolylineGraphics;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::horizontal_origin::HorizontalOrigin;
use cesium_scene::vertical_origin::VerticalOrigin;

// ============================================================================
// Minimal XML tree + helpers (stand-in for the JS DOM checker).
// ============================================================================

#[derive(Debug, Default, Clone)]
struct XmlNode {
    local_name: String,
    attributes: Vec<(String, String)>,
    children: Vec<XmlNode>,
    text: String,
}

fn parse_xml(xml: &str) -> XmlNode {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    fn node_from(e: &quick_xml::events::BytesStart) -> XmlNode {
        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
        let local_name = name.rsplit(':').next().unwrap_or(&name).to_string();
        let mut node = XmlNode {
            local_name,
            ..Default::default()
        };
        for attribute in e.attributes().flatten() {
            let key = String::from_utf8_lossy(attribute.key.as_ref()).to_string();
            let value = String::from_utf8_lossy(&attribute.value).to_string();
            node.attributes.push((key, value));
        }
        node
    }

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut root = XmlNode::default();
    let mut stack: Vec<XmlNode> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                stack.push(node_from(&e));
            }
            Ok(Event::Empty(e)) => {
                let node = node_from(&e);
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => root = node,
                }
            }
            Ok(Event::End(_)) => {
                let node = stack.pop().expect("unbalanced XML");
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => root = node,
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("XML parse error: {:?}", error),
        }
        buf.clear();
    }
    root
}

fn find<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    node.children.iter().find(|child| child.local_name == name)
}

fn find_descendant<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    for child in &node.children {
        if child.local_name == name {
            return Some(child);
        }
        if let Some(found) = find_descendant(child, name) {
            return Some(found);
        }
    }
    None
}

fn attr<'a>(node: &'a XmlNode, name: &str) -> Option<&'a str> {
    node.attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn text_of<'a>(node: &'a XmlNode, name: &str) -> Option<&'a str> {
    find(node, name).map(|child| child.text.as_str())
}

// ============================================================================
// Spec fixtures (verbatim mirrors of the JS consts at the top of the spec).
// ============================================================================

fn point_position() -> Cartesian3 {
    Cartesian3::from_degrees_new(-75.59777, 40.03883, Some(12.0), None)
}

fn create_entity(counter: u32) -> Entity {
    let mut entity = Entity::new(&format!("e{}", counter));
    entity.name = Some(format!("entity{}", counter));
    entity.show = true;
    entity.description = Some(format!("This is entity number {}", counter));
    entity.position = Some(point_position());
    entity
}

fn polyline_positions() -> Vec<Cartesian3> {
    vec![
        Cartesian3::from_degrees_new(-1.0, -1.0, Some(12.0), None),
        Cartesian3::from_degrees_new(1.0, -1.0, Some(12.0), None),
        Cartesian3::from_degrees_new(1.0, 1.0, Some(12.0), None),
        Cartesian3::from_degrees_new(-1.0, 1.0, Some(12.0), None),
    ]
}

/// Exports and parses the KML, then runs the shared `checkKmlDoc` asserts
/// (root is <kml>, exactly one child, xmlns attributes present). Returns
/// the <Document> node.
fn check_kml_doc(entities: &EntityCollection, options: ExportKmlOptions) -> (XmlNode, XmlNode) {
    let result = export_kml(entities, options).expect("export should succeed");
    let root = parse_xml(&result.kml);
    assert_eq!(root.local_name, "kml");
    assert_eq!(attr(&root, "xmlns"), Some("http://www.opengis.net/kml/2.2"));
    assert_eq!(
        attr(&root, "xmlns:gx"),
        Some("http://www.google.com/kml/ext/2.2")
    );
    assert_eq!(root.children.len(), 1);
    let document = root.children[0].clone();
    assert_eq!(document.local_name, "Document");
    (root, document)
}

// Mirror of the spec helper `checkPointCoord`.
fn check_point_coord(text: &str, expected: &Cartesian3) {
    let values: Vec<f64> = text
        .split(',')
        .map(|value| value.trim().parse().unwrap())
        .collect();
    assert_eq!(values.len(), 3);

    let cartographic1 = Cartographic::from_cartesian_new(expected, None).unwrap();
    let cartographic2 = Cartographic::from_degrees_new(values[0], values[1], Some(values[2]));
    assert!(
        (cartographic1.longitude - cartographic2.longitude).abs() < 1e-7
            && (cartographic1.latitude - cartographic2.latitude).abs() < 1e-7
            && (cartographic1.height - cartographic2.height).abs() < 1e-3
    );
}

// Mirror of the spec helper `checkCoords` (asserts every ring position is
// present with the provided height override).
fn check_coords(text: &str, positions: &[Cartesian3], height: Option<f64>) {
    let coordinates: Vec<&str> = text.split(' ').collect();
    assert_eq!(coordinates.len(), positions.len());
    for (i, coordinate) in coordinates.iter().enumerate() {
        let values: Vec<f64> = coordinate.split(',').map(|v| v.parse().unwrap()).collect();
        assert_eq!(values.len(), 3);
        let mut cartographic1 = Cartographic::from_cartesian_new(&positions[i], None).unwrap();
        if let Some(height) = height {
            cartographic1.height = height;
        }
        let cartographic2 = Cartographic::from_degrees_new(values[0], values[1], Some(values[2]));
        assert!(
            (cartographic1.longitude - cartographic2.longitude).abs() < 1e-7
                && (cartographic1.latitude - cartographic2.latitude).abs() < 1e-7
                && (cartographic1.height - cartographic2.height).abs() < 1e-3,
            "coordinate {} mismatch: {} vs {:?}",
            i,
            coordinate,
            positions[i]
        );
    }
}

// ============================================================================
// Hierarchy
// ============================================================================

#[test]
fn hierarchy() {
    let mut entity1 = create_entity(1);
    entity1.show = false;
    entity1.position = None;

    let mut entity2 = create_entity(2);
    entity2.position = None;
    entity2.parent_id = Some(String::from("e1"));

    let mut entity3 = create_entity(3);
    entity3.parent_id = Some(String::from("e2"));
    entity3.point = Some(PointGraphics::new());

    let mut entities = EntityCollection::new();
    entities.add(entity1);
    entities.add(entity2);
    entities.add(entity3);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    // Document children: Style (saved first), then the e1 Folder.
    let style = find(&document, "Style").expect("Style");
    assert_eq!(attr(style, "id"), Some("style-1"));
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    assert!(icon_style.children.is_empty());

    let folder1 = find(&document, "Folder").expect("Folder");
    assert_eq!(attr(folder1, "id"), Some("e1"));
    assert_eq!(text_of(folder1, "name"), Some("entity1"));
    assert_eq!(text_of(folder1, "visibility"), Some("0"));
    assert_eq!(
        text_of(folder1, "description"),
        Some("This is entity number 1")
    );

    let folder2 = find(folder1, "Folder").expect("nested Folder");
    assert_eq!(attr(folder2, "id"), Some("e2"));
    assert_eq!(text_of(folder2, "name"), Some("entity2"));
    assert_eq!(text_of(folder2, "visibility"), Some("1"));

    let placemark = find(folder2, "Placemark").expect("Placemark");
    assert_eq!(attr(placemark, "id"), Some("e3"));
    let point = find(placemark, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("absolute"));
    check_point_coord(text_of(point, "coordinates").unwrap(), &point_position());
    assert_eq!(text_of(placemark, "name"), Some("entity3"));
    assert_eq!(text_of(placemark, "visibility"), Some("1"));
    assert_eq!(text_of(placemark, "styleUrl"), Some("#style-1"));
}

// ============================================================================
// Point Geometry
// ============================================================================

#[test]
fn point_with_constant_position() {
    let mut entity1 = create_entity(1);
    entity1.point = Some(PointGraphics {
        color: Color::LINEN,
        pixel_size: 3.0,
        height_reference: HeightReference::ClampToGround as i32,
        ..PointGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    assert_eq!(attr(style, "id"), Some("style-1"));
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    assert_eq!(text_of(icon_style, "color"), Some("ffe6f0fa"));
    assert_eq!(text_of(icon_style, "colorMode"), Some("normal"));
    assert_eq!(text_of(icon_style, "scale"), Some("0.09375"));

    let placemark = find(&document, "Placemark").expect("Placemark");
    assert_eq!(attr(placemark, "id"), Some("e1"));
    assert_eq!(text_of(placemark, "name"), Some("entity1"));
    assert_eq!(text_of(placemark, "visibility"), Some("1"));
    assert_eq!(text_of(placemark, "styleUrl"), Some("#style-1"));
    let point = find(placemark, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("clampToGround"));
    check_point_coord(text_of(point, "coordinates").unwrap(), &point_position());
}

#[test]
fn point_with_label() {
    let mut entity1 = create_entity(1);
    entity1.point = Some(PointGraphics {
        color: Color::LINEN,
        pixel_size: 3.0,
        height_reference: HeightReference::ClampToGround as i32,
        ..PointGraphics::new()
    });
    entity1.label = Some(LabelGraphics {
        text: Some(String::from("Im a label")),
        fill_color: Color::ORANGE,
        scale: 2.0,
        ..LabelGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    assert_eq!(text_of(icon_style, "color"), Some("ffe6f0fa"));
    assert_eq!(text_of(icon_style, "scale"), Some("0.09375"));
    let label_style = find(style, "LabelStyle").expect("LabelStyle");
    assert_eq!(text_of(label_style, "color"), Some("ff00a5ff"));
    assert_eq!(text_of(label_style, "colorMode"), Some("normal"));
    assert_eq!(text_of(label_style, "scale"), Some("2"));

    let placemark = find(&document, "Placemark").expect("Placemark");
    // KML only shows the name as a label, so the label text replaces it.
    assert_eq!(text_of(placemark, "name"), Some("Im a label"));
    let point = find(placemark, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("clampToGround"));
}

#[test]
fn billboard_with_constant_position() {
    let mut entity1 = create_entity(1);
    entity1.billboard = Some(BillboardGraphics {
        image: Some(String::from("http://test.invalid/image.jpg")),
        image_sub_region: Some((12.0, 0.0, 24.0, 36.0)),
        color: Some(Color::LINEN),
        scale: 2.0,
        pixel_offset: Some((2.0, 3.0)),
        width: Some(24.0),
        height: Some(36.0),
        horizontal_origin: HorizontalOrigin::Left as i32,
        vertical_origin: VerticalOrigin::Bottom as i32,
        rotation: CesiumMath::to_radians(10.0),
        aligned_axis: Some(Cartesian3::UNIT_Z),
        height_reference: HeightReference::ClampToGround as i32,
        ..BillboardGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");

    let icon = find(icon_style, "Icon").expect("Icon");
    assert_eq!(
        text_of(icon, "href"),
        Some("http://test.invalid/image.jpg")
    );
    assert_eq!(text_of(icon, "x"), Some("12"));
    assert_eq!(text_of(icon, "y"), Some("0"));
    assert_eq!(text_of(icon, "w"), Some("24"));
    assert_eq!(text_of(icon, "h"), Some("36"));

    assert_eq!(text_of(icon_style, "color"), Some("ffe6f0fa"));
    assert_eq!(text_of(icon_style, "colorMode"), Some("normal"));
    assert_eq!(text_of(icon_style, "scale"), Some("2"));

    let hot_spot = find(icon_style, "hotSpot").expect("hotSpot");
    assert_eq!(attr(hot_spot, "x"), Some("-1"));
    assert_eq!(attr(hot_spot, "y"), Some("1.5"));
    assert_eq!(attr(hot_spot, "xunits"), Some("pixels"));
    assert_eq!(attr(hot_spot, "yunits"), Some("pixels"));

    let heading: f64 = text_of(icon_style, "heading").unwrap().parse().unwrap();
    assert!((heading - -10.0).abs() < 1e-7);

    let placemark = find(&document, "Placemark").expect("Placemark");
    let point = find(placemark, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("clampToGround"));
    check_point_coord(text_of(point, "coordinates").unwrap(), &point_position());
}

#[test]
fn billboard_with_aligned_axis_not_z() {
    let mut entity1 = create_entity(1);
    entity1.billboard = Some(BillboardGraphics {
        rotation: CesiumMath::to_radians(10.0),
        aligned_axis: Some(Cartesian3::new(0.0, 1.0, 0.0)),
        ..BillboardGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    assert!(icon_style.children.is_empty());

    let placemark = find(&document, "Placemark").expect("Placemark");
    let point = find(placemark, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("absolute"));
}

#[test]
fn billboard_with_0_degree_heading_should_be_360() {
    let mut entity1 = create_entity(1);
    entity1.billboard = Some(BillboardGraphics {
        rotation: CesiumMath::to_radians(0.0),
        aligned_axis: Some(Cartesian3::UNIT_Z),
        ..BillboardGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    assert_eq!(text_of(icon_style, "heading"), Some("360"));

    let placemark = find(&document, "Placemark").expect("Placemark");
    let point = find(placemark, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("absolute"));
}

#[test]
fn billboard_with_hot_spot_at_the_center() {
    let mut entity1 = create_entity(1);
    entity1.billboard = Some(BillboardGraphics {
        pixel_offset: Some((2.0, 3.0)),
        width: Some(24.0),
        height: Some(36.0),
        horizontal_origin: HorizontalOrigin::Center as i32,
        vertical_origin: VerticalOrigin::Center as i32,
        ..BillboardGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    let hot_spot = find(icon_style, "hotSpot").expect("hotSpot");
    assert_eq!(attr(hot_spot, "x"), Some("10"));
    assert_eq!(attr(hot_spot, "y"), Some("21"));
}

#[test]
fn billboard_with_hot_spot_at_the_top_right() {
    let mut entity1 = create_entity(1);
    entity1.billboard = Some(BillboardGraphics {
        pixel_offset: Some((2.0, 3.0)),
        width: Some(24.0),
        height: Some(36.0),
        horizontal_origin: HorizontalOrigin::Right as i32,
        vertical_origin: VerticalOrigin::Top as i32,
        ..BillboardGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    let hot_spot = find(icon_style, "hotSpot").expect("hotSpot");
    assert_eq!(attr(hot_spot, "x"), Some("22"));
    assert_eq!(attr(hot_spot, "y"), Some("39"));
}

#[test]
fn billboard_with_a_data_uri_image() {
    // DEVIATION: the JS spec uses a canvas image exported as PNG; the value
    // model stores images as strings, so a data URI exercises the same
    // ExternalFileHandler path.
    let mut entity1 = create_entity(1);
    entity1.billboard = Some(BillboardGraphics {
        image: Some(String::from("data:image/png;base64,AAECAw==")),
        ..BillboardGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let result = export_kml(&entities, ExportKmlOptions::default()).unwrap();
    let root = parse_xml(&result.kml);
    let document = &root.children[0];

    let style = find(document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    let icon = find(icon_style, "Icon").expect("Icon");
    assert_eq!(text_of(icon, "href"), Some("texture_1.png"));

    assert_eq!(result.external_files.len(), 1);
    assert_eq!(
        result.external_files.get("texture_1.png"),
        Some(&vec![0u8, 1, 2, 3])
    );

    let placemark = find(document, "Placemark").expect("Placemark");
    let point = find(placemark, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("absolute"));
}

#[test]
fn billboard_with_a_data_uri_image_as_kmz_is_rejected() {
    // DEVIATION: the JS spec verifies a zip archive; the port rejects kmz.
    let mut entity1 = create_entity(1);
    entity1.billboard = Some(BillboardGraphics {
        image: Some(String::from("data:image/png;base64,AAECAw==")),
        ..BillboardGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let options = ExportKmlOptions {
        kmz: true,
        ..Default::default()
    };
    assert!(export_kml(&entities, options).is_err());
}

// ============================================================================
// Tracks: DEVIATION — the value model has no sampled/callback positions, so
// the JS "SampledPosition"/"CallbackProperty"/"With Path" specs have no
// counterpart (see export_kml.rs module docs).
// ============================================================================

// ============================================================================
// Polylines
// ============================================================================

#[test]
fn polyline_clamped_to_ground() {
    let mut entity1 = create_entity(1);
    entity1.polyline = Some(PolylineGraphics {
        positions: polyline_positions(),
        clamp_to_ground: true,
        material_color: Color::GREEN,
        width: 5.0,
        // DEVIATION: the JS spec sets zIndex 2; the value model has no
        // zIndex, so gx:drawOrder is skipped.
        ..PolylineGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let line_style = find(style, "LineStyle").expect("LineStyle");
    assert_eq!(text_of(line_style, "color"), Some("ff008000"));
    assert_eq!(text_of(line_style, "colorMode"), Some("normal"));
    assert_eq!(text_of(line_style, "width"), Some("5"));

    let placemark = find(&document, "Placemark").expect("Placemark");
    let line_string = find(placemark, "LineString").expect("LineString");
    assert_eq!(text_of(line_string, "altitudeMode"), Some("clampToGround"));
    check_coords(
        text_of(line_string, "coordinates").unwrap(),
        &polyline_positions(),
        None,
    );
    assert_eq!(text_of(line_string, "tessellate"), Some("1"));
}

#[test]
fn polyline_not_clamped_to_ground() {
    let mut entity1 = create_entity(1);
    entity1.polyline = Some(PolylineGraphics {
        positions: polyline_positions(),
        clamp_to_ground: false,
        material_color: Color::GREEN,
        width: 5.0,
        ..PolylineGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let line_style = find(style, "LineStyle").expect("LineStyle");
    assert_eq!(text_of(line_style, "color"), Some("ff008000"));
    assert_eq!(text_of(line_style, "width"), Some("5"));

    let placemark = find(&document, "Placemark").expect("Placemark");
    let line_string = find(placemark, "LineString").expect("LineString");
    assert_eq!(text_of(line_string, "altitudeMode"), Some("absolute"));
    assert!(find(line_string, "tessellate").is_none());
    check_coords(
        text_of(line_string, "coordinates").unwrap(),
        &polyline_positions(),
        None,
    );
}

// ============================================================================
// Polygons
// ============================================================================

fn polygon_outline_entity() -> Entity {
    let mut entity1 = create_entity(1);
    entity1.polygon = Some(PolygonGraphics {
        hierarchy: polyline_positions(),
        height: Some(10.0),
        per_position_height: Some(false),
        // DEVIATION: the JS spec sets heightReference CLAMP_TO_GROUND; the
        // value model has no polygon heightReference (altitudeMode absolute).
        extruded_height: Some(0.0),
        fill: true,
        material_color: Color::GREEN,
        outline: true,
        outline_width: 5.0,
        outline_color: Color::BLUE,
        ..PolygonGraphics::new()
    });
    entity1
}

#[test]
fn polygon_with_outline() {
    let mut entities = EntityCollection::new();
    entities.add(polygon_outline_entity());

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let poly_style = find(style, "PolyStyle").expect("PolyStyle");
    assert_eq!(text_of(poly_style, "color"), Some("ff008000"));
    assert_eq!(text_of(poly_style, "colorMode"), Some("normal"));
    // DEVIATION: the JS spec also asserts fill; the value model cannot
    // distinguish an explicit fill=true from the default, so no <fill>
    // element is emitted.
    assert_eq!(text_of(poly_style, "outline"), Some("1"));

    let line_style = find(style, "LineStyle").expect("LineStyle");
    assert_eq!(text_of(line_style, "color"), Some("ffff0000"));
    assert_eq!(text_of(line_style, "colorMode"), Some("normal"));
    assert_eq!(text_of(line_style, "width"), Some("5"));

    let placemark = find(&document, "Placemark").expect("Placemark");
    let polygon = find(placemark, "Polygon").expect("Polygon");
    assert_eq!(text_of(polygon, "altitudeMode"), Some("absolute"));
    let outer = find(polygon, "outerBoundaryIs").expect("outerBoundaryIs");
    let ring = find(outer, "LinearRing").expect("LinearRing");
    check_coords(
        text_of(ring, "coordinates").unwrap(),
        &polyline_positions(),
        Some(10.0),
    );
}

#[test]
fn polygon_with_extrusion() {
    let mut entity1 = create_entity(1);
    entity1.polygon = Some(PolygonGraphics {
        hierarchy: polyline_positions(),
        height: Some(10.0),
        per_position_height: Some(false),
        extruded_height: Some(20.0),
        ..PolygonGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let poly_style = find(style, "PolyStyle").expect("PolyStyle");
    assert!(poly_style.children.is_empty());

    let placemark = find(&document, "Placemark").expect("Placemark");
    let polygon = find(placemark, "Polygon").expect("Polygon");
    assert_eq!(text_of(polygon, "altitudeMode"), Some("absolute"));
    let outer = find(polygon, "outerBoundaryIs").expect("outerBoundaryIs");
    let ring = find(outer, "LinearRing").expect("LinearRing");
    // We use extrudedHeight
    check_coords(
        text_of(ring, "coordinates").unwrap(),
        &polyline_positions(),
        Some(20.0),
    );
    assert_eq!(text_of(polygon, "extrude"), Some("1"));
}

#[test]
fn polygon_with_extrusion_and_per_position_heights() {
    let mut entity1 = create_entity(1);
    entity1.polygon = Some(PolygonGraphics {
        hierarchy: polyline_positions(),
        height: Some(10.0),
        per_position_height: Some(true),
        extruded_height: Some(20.0),
        ..PolygonGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let poly_style = find(style, "PolyStyle").expect("PolyStyle");
    assert!(poly_style.children.is_empty());

    let placemark = find(&document, "Placemark").expect("Placemark");
    let polygon = find(placemark, "Polygon").expect("Polygon");
    let outer = find(polygon, "outerBoundaryIs").expect("outerBoundaryIs");
    let ring = find(outer, "LinearRing").expect("LinearRing");
    // Use per position height (12)
    check_coords(
        text_of(ring, "coordinates").unwrap(),
        &polyline_positions(),
        None,
    );
    assert_eq!(text_of(polygon, "extrude"), Some("1"));
}

#[test]
fn polygon_with_holes() {
    let mut entity1 = create_entity(1);
    entity1.polygon = Some(PolygonGraphics {
        hierarchy: polyline_positions(),
        holes: vec![polyline_positions()],
        height: Some(10.0),
        ..PolygonGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let poly_style = find(style, "PolyStyle").expect("PolyStyle");
    assert!(poly_style.children.is_empty());

    let placemark = find(&document, "Placemark").expect("Placemark");
    let polygon = find(placemark, "Polygon").expect("Polygon");
    assert_eq!(text_of(polygon, "altitudeMode"), Some("absolute"));
    let outer = find(polygon, "outerBoundaryIs").expect("outerBoundaryIs");
    check_coords(
        text_of(find(outer, "LinearRing").unwrap(), "coordinates").unwrap(),
        &polyline_positions(),
        Some(10.0),
    );
    let inner = find(polygon, "innerBoundaryIs").expect("innerBoundaryIs");
    check_coords(
        text_of(find(inner, "LinearRing").unwrap(), "coordinates").unwrap(),
        &polyline_positions(),
        Some(10.0),
    );
}

// ============================================================================
// Rectangles/GroundOverlays: DEVIATION — the entity model has no rectangle
// field, so the JS "Rectangle extruded"/"Rectangle not extruded"/
// "GroundOverlays Rectangle" specs have no counterpart.
// ============================================================================

// ============================================================================
// Models
// ============================================================================

#[test]
fn model_with_constant_position() {
    let mut entity1 = create_entity(1);
    entity1.model = Some(ModelGraphics {
        uri: Some(String::from("http://test.invalid/test.glb")),
        scale: 3.0,
        // DEVIATION: the JS spec sets heightReference CLAMP_TO_GROUND; the
        // value model has no model heightReference (altitudeMode absolute).
        ..ModelGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let cartographic = Cartographic::from_cartesian_new(&point_position(), None).unwrap();
    let options = ExportKmlOptions {
        model_callback: Some(Box::new(|model, _time, _external_files| {
            model
                .uri
                .as_deref()
                .unwrap_or_default()
                .replace(".glb", ".dae")
        })),
        ..Default::default()
    };

    let result = export_kml(&entities, options).unwrap();
    let root = parse_xml(&result.kml);
    assert_eq!(root.local_name, "kml");
    let document = &root.children[0];

    let placemark = find(document, "Placemark").expect("Placemark");
    let model = find(placemark, "Model").expect("Model");
    assert_eq!(text_of(model, "altitudeMode"), Some("absolute"));

    let location = find(model, "Location").expect("Location");
    let longitude: f64 = text_of(location, "longitude").unwrap().parse().unwrap();
    let latitude: f64 = text_of(location, "latitude").unwrap().parse().unwrap();
    let altitude: f64 = text_of(location, "altitude").unwrap().parse().unwrap();
    assert!((longitude - CesiumMath::to_degrees(cartographic.longitude)).abs() < 1e-7);
    assert!((latitude - CesiumMath::to_degrees(cartographic.latitude)).abs() < 1e-7);
    assert!((altitude - cartographic.height).abs() < 1e-3);

    let link = find(model, "Link").expect("Link");
    assert_eq!(text_of(link, "href"), Some("http://test.invalid/test.dae"));

    let scale = find(model, "scale").expect("scale");
    assert_eq!(text_of(scale, "x"), Some("3"));
    assert_eq!(text_of(scale, "y"), Some("3"));
    assert_eq!(text_of(scale, "z"), Some("3"));
}

#[test]
fn model_without_callback_errors() {
    // Mirror of the upstream RuntimeError when no model callback is given.
    let mut entity1 = create_entity(1);
    entity1.model = Some(ModelGraphics {
        uri: Some(String::from("http://test.invalid/test.glb")),
        ..ModelGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let error = export_kml(&entities, ExportKmlOptions::default()).unwrap_err();
    assert!(error.contains("no model callback was supplied"));
}

#[test]
fn model_callback_can_add_external_files() {
    let mut entity1 = create_entity(1);
    entity1.model = Some(ModelGraphics {
        uri: Some(String::from("http://test.invalid/test")),
        ..ModelGraphics::new()
    });

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let options = ExportKmlOptions {
        model_callback: Some(Box::new(|model, _time, external_files| {
            external_files.insert(String::from("test.dae"), Vec::new());
            model.uri.clone().unwrap_or_default()
        })),
        ..Default::default()
    };

    let result = export_kml(&entities, options).unwrap();
    assert_eq!(
        result.external_files.get("test.dae"),
        Some(&Vec::<u8>::new())
    );
}

// ============================================================================
// Multigeometry
// ============================================================================

#[test]
fn polygon_and_point_multigeometry() {
    let mut entity1 = create_entity(1);
    entity1.polygon = Some(PolygonGraphics {
        hierarchy: polyline_positions(),
        ..PolygonGraphics::new()
    });
    entity1.point = Some(PointGraphics::new());

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let style = find(&document, "Style").expect("Style");
    let icon_style = find(style, "IconStyle").expect("IconStyle");
    assert!(icon_style.children.is_empty());
    let poly_style = find(style, "PolyStyle").expect("PolyStyle");
    assert!(poly_style.children.is_empty());

    let placemark = find(&document, "Placemark").expect("Placemark");
    let multi = find(placemark, "MultiGeometry").expect("MultiGeometry");

    let point = find(multi, "Point").expect("Point");
    assert_eq!(text_of(point, "altitudeMode"), Some("absolute"));
    check_point_coord(text_of(point, "coordinates").unwrap(), &point_position());

    let polygon = find(multi, "Polygon").expect("Polygon");
    assert_eq!(text_of(polygon, "altitudeMode"), Some("absolute"));
    let outer = find(polygon, "outerBoundaryIs").expect("outerBoundaryIs");
    let ring = find(outer, "LinearRing").expect("LinearRing");
    check_coords(
        text_of(ring, "coordinates").unwrap(),
        &polyline_positions(),
        Some(0.0),
    );
}

// ============================================================================
// Supporting semantics not covered by individual JS `it()`s but required by
// the mirrored implementation.
// ============================================================================

#[test]
fn style_cache_deduplicates_identical_styles() {
    let entity1 = create_entity(1);
    let entity2 = create_entity(2);
    let mut entity1 = entity1;
    let mut entity2 = entity2;
    entity1.point = Some(PointGraphics::new());
    entity2.point = Some(PointGraphics::new());

    let mut entities = EntityCollection::new();
    entities.add(entity1);
    entities.add(entity2);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    // Both placemarks share the single cached empty IconStyle.
    let styles: Vec<&XmlNode> = document
        .children
        .iter()
        .filter(|child| child.local_name == "Style")
        .collect();
    assert_eq!(styles.len(), 1);
    assert_eq!(attr(styles[0], "id"), Some("style-1"));
    for placemark in document
        .children
        .iter()
        .filter(|child| child.local_name == "Placemark")
    {
        assert_eq!(text_of(placemark, "styleUrl"), Some("#style-1"));
    }
}

#[test]
fn id_manager_deduplicates_entity_with_geometry_and_children() {
    // An entity with both geometry and children gets a Placemark and a
    // Folder; the second id gets a "-1" suffix.
    let mut entity1 = create_entity(1);
    entity1.point = Some(PointGraphics::new());

    let mut entity2 = create_entity(2);
    entity2.parent_id = Some(String::from("e1"));
    entity2.point = Some(PointGraphics::new());

    let mut entities = EntityCollection::new();
    entities.add(entity1);
    entities.add(entity2);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let placemark = find(&document, "Placemark").expect("Placemark");
    assert_eq!(attr(placemark, "id"), Some("e1"));
    let folder = find(&document, "Folder").expect("Folder");
    assert_eq!(attr(folder, "id"), Some("e1-1"));
}

#[test]
fn availability_becomes_time_span() {
    let mut entity1 = create_entity(1);
    entity1.point = Some(PointGraphics::new());
    entity1.availability = vec![TimeInterval::new(
        JulianDate::from_iso8601("2019-06-17"),
        JulianDate::from_iso8601("2019-06-19"),
        Some(true),
        Some(false),
    )];

    let mut entities = EntityCollection::new();
    entities.add(entity1);

    let (_root, document) = check_kml_doc(&entities, ExportKmlOptions::default());

    let placemark = find(&document, "Placemark").expect("Placemark");
    let time_span = find(placemark, "TimeSpan").expect("TimeSpan");
    assert!(text_of(time_span, "begin")
        .unwrap()
        .starts_with("2019-06-17"));
    assert!(text_of(time_span, "end").unwrap().starts_with("2019-06-19"));
}
