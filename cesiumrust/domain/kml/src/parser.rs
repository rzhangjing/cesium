//! KML (Keyhole Markup Language) parser.
//!
//! Maps to CesiumJS `DataSources/KmlDataSource.js`:
//! - Placemark parsing
//! - Geometry types (Point, LineString, Polygon, MultiGeometry)
//! - Style resolution
//! - Extended data

use cesium_datasource::entity::{
    Entity, PointGraphics, PolygonGraphics, PolylineGraphics,
};
use cesium_datasource::entity_collection::DataSource;
use cesium_datasource::property::{Color, Property};
use cesium_geospatial::cartographic::Cartographic;

/// KML coordinate (longitude, latitude, altitude).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KmlCoordinate {
    /// Longitude in degrees.
    pub longitude: f64,
    /// Latitude in degrees.
    pub latitude: f64,
    /// Altitude in meters.
    pub altitude: f64,
}

impl KmlCoordinate {
    /// Creates a new KML coordinate.
    pub fn new(longitude: f64, latitude: f64, altitude: f64) -> Self {
        Self {
            longitude,
            latitude,
            altitude,
        }
    }

    /// Converts to Cartographic (radians).
    pub fn to_cartographic(&self) -> Cartographic {
        Cartographic::from_radians(
            self.longitude.to_radians(),
            self.latitude.to_radians(),
            self.altitude,
        )
    }
}

/// KML geometry types.
#[derive(Debug, Clone, PartialEq)]
pub enum KmlGeometry {
    /// A single point.
    Point {
        /// The coordinate.
        coordinate: KmlCoordinate,
        /// Whether to extrude to ground.
        extrude: bool,
    },
    /// A line string.
    LineString {
        /// The coordinates.
        coordinates: Vec<KmlCoordinate>,
        /// Whether to extrude to ground.
        extrude: bool,
        /// Tessellation (follow terrain).
        tessellate: bool,
    },
    /// A polygon (outer boundary + optional inner boundaries).
    Polygon {
        /// Outer boundary coordinates.
        outer: Vec<KmlCoordinate>,
        /// Inner boundaries (holes).
        inner: Vec<Vec<KmlCoordinate>>,
        /// Whether to extrude.
        extrude: bool,
        /// Extrude height.
        altitude: f64,
    },
    /// Multiple geometries.
    MultiGeometry {
        /// Child geometries.
        geometries: Vec<KmlGeometry>,
    },
}

/// KML style definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KmlStyle {
    /// Style ID (for reference).
    pub id: Option<String>,
    /// Icon style (for points).
    pub icon_style: Option<KmlIconStyle>,
    /// Line style.
    pub line_style: Option<KmlLineStyle>,
    /// Polygon style.
    pub poly_style: Option<KmlPolyStyle>,
    /// Label style.
    pub label_style: Option<KmlLabelStyle>,
}

/// KML icon style.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlIconStyle {
    /// Icon color (aabbggrr format).
    pub color: Option<String>,
    /// Icon scale.
    pub scale: f64,
    /// Icon URL.
    pub href: Option<String>,
}

impl Default for KmlIconStyle {
    fn default() -> Self {
        Self {
            color: None,
            scale: 1.0,
            href: None,
        }
    }
}

/// KML line style.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlLineStyle {
    /// Line color (aabbggrr format).
    pub color: Option<String>,
    /// Line width in pixels.
    pub width: f64,
}

impl Default for KmlLineStyle {
    fn default() -> Self {
        Self {
            color: None,
            width: 1.0,
        }
    }
}

/// KML polygon style.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlPolyStyle {
    /// Fill color (aabbggrr format).
    pub color: Option<String>,
    /// Whether to fill the polygon.
    pub fill: bool,
    /// Whether to draw the outline.
    pub outline: bool,
}

impl Default for KmlPolyStyle {
    fn default() -> Self {
        Self {
            color: None,
            fill: true,
            outline: true,
        }
    }
}

/// KML label style.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlLabelStyle {
    /// Label color (aabbggrr format).
    pub color: Option<String>,
    /// Label scale.
    pub scale: f64,
}

impl Default for KmlLabelStyle {
    fn default() -> Self {
        Self {
            color: None,
            scale: 1.0,
        }
    }
}

/// A KML Placemark.
#[derive(Debug, Clone)]
pub struct KmlPlacemark {
    /// Placemark ID.
    pub id: Option<String>,
    /// Placemark name.
    pub name: Option<String>,
    /// Placemark description.
    pub description: Option<String>,
    /// The geometry.
    pub geometry: Option<KmlGeometry>,
    /// Style URL reference.
    pub style_url: Option<String>,
    /// Inline style.
    pub style: Option<KmlStyle>,
    /// Extended data (key-value pairs).
    pub extended_data: Vec<(String, String)>,
    /// Visibility.
    pub visibility: bool,
}

impl Default for KmlPlacemark {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            description: None,
            geometry: None,
            style_url: None,
            style: None,
            extended_data: Vec::new(),
            visibility: true,
        }
    }
}

/// KML document.
#[derive(Debug, Clone, Default)]
pub struct KmlDocument {
    /// Document name.
    pub name: Option<String>,
    /// Document description.
    pub description: Option<String>,
    /// Placemarks.
    pub placemarks: Vec<KmlPlacemark>,
    /// Styles (by ID).
    pub styles: Vec<KmlStyle>,
    /// Style maps (by ID).
    pub style_maps: Vec<(String, String, String)>, // (id, normal_style, highlight_style)
}

/// Parses KML coordinate string.
///
/// Format: "lon,lat,alt lon,lat,alt ..."
pub fn parse_coordinates(s: &str) -> Vec<KmlCoordinate> {
    s.split_whitespace()
        .filter_map(|tuple| {
            let parts: Vec<&str> = tuple.split(',').collect();
            if parts.len() >= 2 {
                let lon = parts[0].parse().ok()?;
                let lat = parts[1].parse().ok()?;
                let alt = if parts.len() >= 3 {
                    parts[2].parse().unwrap_or(0.0)
                } else {
                    0.0
                };
                Some(KmlCoordinate::new(lon, lat, alt))
            } else {
                None
            }
        })
        .collect()
}

/// Parses KML color (aabbggrr format) to RGBA.
pub fn parse_kml_color(color: &str) -> Option<Color> {
    if color.len() != 8 {
        return None;
    }

    let aa = u8::from_str_radix(&color[0..2], 16).ok()?;
    let bb = u8::from_str_radix(&color[2..4], 16).ok()?;
    let gg = u8::from_str_radix(&color[4..6], 16).ok()?;
    let rr = u8::from_str_radix(&color[6..8], 16).ok()?;

    Some(Color::new(
        rr as f64 / 255.0,
        gg as f64 / 255.0,
        bb as f64 / 255.0,
        aa as f64 / 255.0,
    ))
}

/// Converts KML color to f32 array [r, g, b, a].
pub fn kml_color_to_f32(color: &str) -> [f32; 4] {
    match parse_kml_color(color) {
        Some(c) => c.to_f32_array(),
        None => [1.0, 1.0, 1.0, 1.0],
    }
}

/// Converts a KML document to a DataSource.
pub fn kml_to_datasource(doc: &KmlDocument) -> DataSource {
    let mut ds = DataSource::new(doc.name.clone().unwrap_or_else(|| "KML".to_string()));

    for placemark in &doc.placemarks {
        if let Some(entity) = placemark_to_entity(placemark) {
            ds.entities.add(entity);
        }
    }

    ds
}

/// Converts a KML placemark to an Entity.
fn placemark_to_entity(placemark: &KmlPlacemark) -> Option<Entity> {
    let geometry = placemark.geometry.as_ref()?;

    let mut entity = Entity::new(
        placemark.id.clone().unwrap_or_else(|| {
            placemark.name.clone().unwrap_or_else(|| "placemark".to_string())
        }),
    );

    entity.name = placemark.name.clone();
    entity.show = placemark.visibility;

    // Get style colors
    let (line_color, fill_color) = get_placemark_colors(placemark);

    match geometry {
        KmlGeometry::Point { coordinate, .. } => {
            entity.position = Property::Constant([
                coordinate.longitude.to_radians(),
                coordinate.latitude.to_radians(),
                coordinate.altitude,
            ]);
            entity.point = Some(PointGraphics {
                color: Property::Constant(fill_color),
                pixel_size: Property::Constant(8.0),
                ..Default::default()
            });
        }
        KmlGeometry::LineString { coordinates, .. } => {
            let positions: Vec<[f64; 3]> = coordinates
                .iter()
                .map(|c| [c.longitude.to_radians(), c.latitude.to_radians(), c.altitude])
                .collect();
            entity.polyline = Some(PolylineGraphics {
                positions: Property::Constant(positions),
                width: Property::Constant(2.0),
                color: Property::Constant(line_color),
                ..Default::default()
            });
        }
        KmlGeometry::Polygon { outer, .. } => {
            let positions: Vec<[f64; 3]> = outer
                .iter()
                .map(|c| [c.longitude.to_radians(), c.latitude.to_radians(), c.altitude])
                .collect();
            entity.polygon = Some(PolygonGraphics {
                positions: Property::Constant(positions),
                material: Property::Constant(fill_color),
                ..Default::default()
            });
        }
        KmlGeometry::MultiGeometry { geometries } => {
            // Use the first geometry for simplicity
            if let Some(first) = geometries.first() {
                let temp_placemark = KmlPlacemark {
                    geometry: Some(first.clone()),
                    ..placemark.clone()
                };
                return placemark_to_entity(&temp_placemark);
            }
        }
    }

    Some(entity)
}

/// Gets the line and fill colors for a placemark.
fn get_placemark_colors(placemark: &KmlPlacemark) -> (Color, Color) {
    let style = placemark.style.as_ref();

    let line_color = style
        .and_then(|s| s.line_style.as_ref())
        .and_then(|ls| ls.color.as_ref())
        .map(|c| parse_kml_color(c).unwrap_or(Color::WHITE))
        .unwrap_or(Color::WHITE);

    let fill_color = style
        .and_then(|s| s.poly_style.as_ref())
        .and_then(|ps| ps.color.as_ref())
        .map(|c| parse_kml_color(c).unwrap_or(Color::WHITE))
        .unwrap_or(Color::WHITE);

    (line_color, fill_color)
}

/// Simple KML parser (basic implementation).
///
/// This is a simplified parser that handles common KML structures.
/// For production use, consider using a full XML parser.
pub fn parse_kml_simple(xml: &str) -> Result<KmlDocument, String> {
    let mut doc = KmlDocument::default();

    // Extract document name
    if let Some(name) = extract_tag_content(xml, "name") {
        doc.name = Some(name);
    }

    // Extract placemarks
    let placemarks = extract_all_tags(xml, "Placemark");
    for pm_xml in placemarks {
        let placemark = parse_placemark(&pm_xml)?;
        doc.placemarks.push(placemark);
    }

    Ok(doc)
}

/// Extracts content between tags.
fn extract_tag_content(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;

    Some(xml[start..end].trim().to_string())
}

/// Extracts all occurrences of a tag.
fn extract_all_tags(xml: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let start_tag = format!("<{}", tag);
    let end_tag = format!("</{}>", tag);

    let mut search_start = 0;
    while let Some(start) = xml[search_start..].find(&start_tag) {
        let abs_start = search_start + start;
        if let Some(end) = xml[abs_start..].find(&end_tag) {
            let abs_end = abs_start + end + end_tag.len();
            results.push(xml[abs_start..abs_end].to_string());
            search_start = abs_end;
        } else {
            break;
        }
    }

    results
}

/// Parses a placemark XML fragment.
fn parse_placemark(xml: &str) -> Result<KmlPlacemark, String> {
    // Parse geometry
    let geometry = if xml.contains("<Point>") {
        Some(parse_point_geometry(xml))
    } else if xml.contains("<LineString>") {
        Some(parse_linestring_geometry(xml))
    } else if xml.contains("<Polygon>") {
        Some(parse_polygon_geometry(xml))
    } else {
        None
    };

    // Parse style
    let style = extract_all_tags(xml, "Style").first().map(|s| parse_style(s));

    Ok(KmlPlacemark {
        id: extract_attribute(xml, "id"),
        name: extract_tag_content(xml, "name"),
        description: extract_tag_content(xml, "description"),
        geometry,
        style_url: None,
        style,
        extended_data: Vec::new(),
        visibility: true,
    })
}

/// Parses a Point geometry.
fn parse_point_geometry(xml: &str) -> KmlGeometry {
    let coordinates = extract_tag_content(xml, "coordinates")
        .map(|c| parse_coordinates(&c))
        .unwrap_or_default();

    let extrude = xml.contains("<extrude>1</extrude>");

    KmlGeometry::Point {
        coordinate: coordinates.first().copied().unwrap_or(KmlCoordinate::new(0.0, 0.0, 0.0)),
        extrude,
    }
}

/// Parses a LineString geometry.
fn parse_linestring_geometry(xml: &str) -> KmlGeometry {
    let coordinates = extract_tag_content(xml, "coordinates")
        .map(|c| parse_coordinates(&c))
        .unwrap_or_default();

    let extrude = xml.contains("<extrude>1</extrude>");
    let tessellate = xml.contains("<tessellate>1</tessellate>");

    KmlGeometry::LineString {
        coordinates,
        extrude,
        tessellate,
    }
}

/// Parses a Polygon geometry.
fn parse_polygon_geometry(xml: &str) -> KmlGeometry {
    let outer = extract_tag_content(xml, "outerBoundaryIs")
        .and_then(|ob| extract_tag_content(&ob, "coordinates"))
        .map(|c| parse_coordinates(&c))
        .unwrap_or_default();

    let extrude = xml.contains("<extrude>1</extrude>");

    KmlGeometry::Polygon {
        outer,
        inner: Vec::new(),
        extrude,
        altitude: 0.0,
    }
}

/// Parses a Style element.
fn parse_style(xml: &str) -> KmlStyle {
    let icon_style = extract_all_tags(xml, "IconStyle").first().map(|icon_xml| {
        KmlIconStyle {
            color: extract_tag_content(icon_xml, "color"),
            scale: extract_tag_content(icon_xml, "scale")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0),
            href: extract_tag_content(icon_xml, "href"),
        }
    });

    let line_style = extract_all_tags(xml, "LineStyle").first().map(|line_xml| {
        KmlLineStyle {
            color: extract_tag_content(line_xml, "color"),
            width: extract_tag_content(line_xml, "width")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0),
        }
    });

    let poly_style = extract_all_tags(xml, "PolyStyle").first().map(|poly_xml| {
        KmlPolyStyle {
            color: extract_tag_content(poly_xml, "color"),
            fill: !poly_xml.contains("<fill>0</fill>"),
            outline: !poly_xml.contains("<outline>0</outline>"),
        }
    });

    KmlStyle {
        id: extract_attribute(xml, "id"),
        icon_style,
        line_style,
        poly_style,
        label_style: None,
    }
}

/// Extracts an attribute value from an XML tag.
fn extract_attribute(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = xml.find(&pattern)? + pattern.len();
    let end = xml[start..].find('"')? + start;
    Some(xml[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coordinates() {
        let coords = parse_coordinates("-122.0822035425683,37.42228990140251,0 -122.0822035425683,37.42228990140251,100");
        assert_eq!(coords.len(), 2);
        assert!((coords[0].longitude - (-122.0822035425683)).abs() < 1e-10);
        assert!((coords[0].latitude - 37.42228990140251).abs() < 1e-10);
        assert!((coords[0].altitude - 0.0).abs() < 1e-10);
        assert!((coords[1].altitude - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_coordinates_no_altitude() {
        let coords = parse_coordinates("-122.0,37.0");
        assert_eq!(coords.len(), 1);
        assert!((coords[0].altitude - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_kml_color() {
        // KML color format: aabbggrr
        let color = parse_kml_color("ff0000ff").unwrap(); // Red, fully opaque
        assert!((color.red - 1.0).abs() < 1e-10);
        assert!((color.green - 0.0).abs() < 1e-10);
        assert!((color.blue - 0.0).abs() < 1e-10);
        assert!((color.alpha - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_kml_color_green() {
        let color = parse_kml_color("8000ff00").unwrap(); // Green, 50% transparent
        assert!((color.red - 0.0).abs() < 1e-10);
        assert!((color.green - 1.0).abs() < 1e-10);
        assert!((color.blue - 0.0).abs() < 1e-10);
        assert!((color.alpha - 128.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_kml_color_to_f32() {
        let rgba = kml_color_to_f32("ffffffff");
        assert!((rgba[0] - 1.0).abs() < 0.01);
        assert!((rgba[1] - 1.0).abs() < 0.01);
        assert!((rgba[2] - 1.0).abs() < 0.01);
        assert!((rgba[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_kml_coordinate_to_cartographic() {
        let coord = KmlCoordinate::new(180.0, 90.0, 1000.0);
        let carto = coord.to_cartographic();

        assert!((carto.longitude - std::f64::consts::PI).abs() < 1e-10);
        assert!((carto.latitude - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
        assert!((carto.height - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_kml_simple_point() {
        let kml = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>Test</name>
    <Placemark>
      <name>Point</name>
      <Point>
        <coordinates>-122.0822035425683,37.42228990140251,0</coordinates>
      </Point>
    </Placemark>
  </Document>
</kml>"#;

        let doc = parse_kml_simple(kml).unwrap();
        assert_eq!(doc.name, Some("Test".to_string()));
        assert_eq!(doc.placemarks.len(), 1);
        assert_eq!(doc.placemarks[0].name, Some("Point".to_string()));

        if let Some(KmlGeometry::Point { coordinate, .. }) = &doc.placemarks[0].geometry {
            assert!((coordinate.longitude - (-122.0822035425683)).abs() < 1e-10);
        } else {
            panic!("Expected Point geometry");
        }
    }

    #[test]
    fn test_parse_kml_simple_linestring() {
        let kml = r#"<kml>
  <Placemark>
    <name>Line</name>
    <LineString>
      <coordinates>-122.0,37.0,0 -122.1,37.1,0</coordinates>
    </LineString>
  </Placemark>
</kml>"#;

        let doc = parse_kml_simple(kml).unwrap();
        assert_eq!(doc.placemarks.len(), 1);

        if let Some(KmlGeometry::LineString { coordinates, .. }) = &doc.placemarks[0].geometry {
            assert_eq!(coordinates.len(), 2);
        } else {
            panic!("Expected LineString geometry");
        }
    }

    #[test]
    fn test_parse_kml_simple_polygon() {
        let kml = r#"<kml>
  <Placemark>
    <name>Polygon</name>
    <Polygon>
      <outerBoundaryIs>
        <coordinates>-122.0,37.0,0 -122.1,37.0,0 -122.1,37.1,0 -122.0,37.0,0</coordinates>
      </outerBoundaryIs>
    </Polygon>
  </Placemark>
</kml>"#;

        let doc = parse_kml_simple(kml).unwrap();
        assert_eq!(doc.placemarks.len(), 1);

        if let Some(KmlGeometry::Polygon { outer, .. }) = &doc.placemarks[0].geometry {
            assert_eq!(outer.len(), 4);
        } else {
            panic!("Expected Polygon geometry");
        }
    }

    #[test]
    fn test_parse_style() {
        let style_xml = r#"<Style id="myStyle">
  <LineStyle>
    <color>ff0000ff</color>
    <width>3.0</width>
  </LineStyle>
  <PolyStyle>
    <color>8000ff00</color>
    <fill>1</fill>
    <outline>0</outline>
  </PolyStyle>
</Style>"#;

        let style = parse_style(style_xml);
        assert_eq!(style.id, Some("myStyle".to_string()));

        let line_style = style.line_style.unwrap();
        assert_eq!(line_style.color, Some("ff0000ff".to_string()));
        assert!((line_style.width - 3.0).abs() < 1e-10);

        let poly_style = style.poly_style.unwrap();
        assert_eq!(poly_style.color, Some("8000ff00".to_string()));
        assert!(poly_style.fill);
        assert!(!poly_style.outline);
    }

    #[test]
    fn test_kml_to_datasource() {
        let doc = KmlDocument {
            name: Some("Test".to_string()),
            placemarks: vec![KmlPlacemark {
                id: Some("pm1".to_string()),
                name: Some("Point".to_string()),
                geometry: Some(KmlGeometry::Point {
                    coordinate: KmlCoordinate::new(-122.0, 37.0, 0.0),
                    extrude: false,
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        let ds = kml_to_datasource(&doc);
        assert_eq!(ds.name, "Test");
        assert_eq!(ds.entities.len(), 1);
    }

    #[test]
    fn test_extract_tag_content() {
        let xml = "<name>Test Name</name>";
        assert_eq!(extract_tag_content(xml, "name"), Some("Test Name".to_string()));
    }

    #[test]
    fn test_extract_attribute() {
        let xml = r#"<Style id="myStyle">"#;
        assert_eq!(extract_attribute(xml, "id"), Some("myStyle".to_string()));
    }

    #[test]
    fn test_kml_style_default() {
        let style = KmlStyle::default();
        assert!(style.id.is_none());
        assert!(style.icon_style.is_none());
        assert!(style.line_style.is_none());
        assert!(style.poly_style.is_none());
    }

    #[test]
    fn test_kml_placemark_default() {
        let pm = KmlPlacemark::default();
        assert!(pm.id.is_none());
        assert!(pm.name.is_none());
        assert!(pm.geometry.is_none());
        assert!(pm.visibility);
    }
}
