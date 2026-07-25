//! KML export functionality.
//!
//! Maps to CesiumJS `DataSources/exportKml.js`.

use std::collections::HashMap;

// ============================================================================
// KmlExportOptions
// ============================================================================

/// Options for KML export.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExportOptions {
    /// Name of the KML document.
    pub name: String,
    /// Description of the KML document.
    pub description: Option<String>,
    /// Whether to export time-dynamic data.
    pub time_dynamic: bool,
    /// Whether to export model (glTF) references.
    pub export_model: bool,
    /// Whether to export image resources.
    pub export_images: bool,
    /// KML version (2.2 is standard).
    pub kml_version: String,
}

impl Default for KmlExportOptions {
    fn default() -> Self {
        Self {
            name: "Cesium Export".to_string(),
            description: None,
            time_dynamic: false,
            export_model: true,
            export_images: true,
            kml_version: "2.2".to_string(),
        }
    }
}

// ============================================================================
// KmlExportResult
// ============================================================================

/// Result of a KML export operation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KmlExportResult {
    /// The KML document content.
    pub kml: String,
    /// External files (images, models) referenced by the KML.
    pub external_files: HashMap<String, Vec<u8>>,
    /// The KMZ (zipped KML) content, if requested.
    pub kmz: Option<Vec<u8>>,
}

// ============================================================================
// KmlExporter
// ============================================================================

/// KML exporter for converting entities to KML format.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExporter {
    /// Export options.
    pub options: KmlExportOptions,
    /// Namespace declarations.
    namespaces: Vec<(String, String)>,
    /// Style definitions.
    styles: Vec<KmlExportStyle>,
    /// Placemarks.
    placemarks: Vec<KmlExportPlacemark>,
}

impl KmlExporter {
    /// Create a new exporter with default options.
    pub fn new() -> Self {
        Self {
            options: KmlExportOptions::default(),
            namespaces: vec![
                ("xmlns".to_string(), "http://www.opengis.net/kml/2.2".to_string()),
                ("xmlns:gx".to_string(), "http://www.google.com/kml/ext/2.2".to_string()),
                ("xmlns:atom".to_string(), "http://www.w3.org/2005/Atom".to_string()),
            ],
            styles: Vec::new(),
            placemarks: Vec::new(),
        }
    }

    /// Create with custom options.
    pub fn with_options(options: KmlExportOptions) -> Self {
        Self {
            options,
            ..Self::new()
        }
    }

    /// Add a style definition.
    pub fn add_style(&mut self, style: KmlExportStyle) {
        self.styles.push(style);
    }

    /// Add a placemark.
    pub fn add_placemark(&mut self, placemark: KmlExportPlacemark) {
        self.placemarks.push(placemark);
    }

    /// Generate the KML document.
    pub fn to_kml(&self) -> String {
        let mut kml = String::new();

        // XML declaration
        kml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

        // KML root element with namespaces
        kml.push_str("<kml");
        for (prefix, uri) in &self.namespaces {
            kml.push_str(&format!(" {}=\"{}\"", prefix, uri));
        }
        kml.push_str(">\n");

        // Document
        kml.push_str("  <Document>\n");

        // Name
        kml.push_str(&format!("    <name>{}</name>\n", escape_xml(&self.options.name)));

        // Description
        if let Some(ref desc) = self.options.description {
            kml.push_str(&format!("    <description>{}</description>\n", escape_xml(desc)));
        }

        // Styles
        for style in &self.styles {
            kml.push_str(&style.to_kml(4));
        }

        // Placemarks
        for placemark in &self.placemarks {
            kml.push_str(&placemark.to_kml(4));
        }

        kml.push_str("  </Document>\n");
        kml.push_str("</kml>\n");

        kml
    }

    /// Export to KmlExportResult.
    pub fn export(&self) -> KmlExportResult {
        KmlExportResult {
            kml: self.to_kml(),
            external_files: HashMap::new(),
            kmz: None,
        }
    }
}

impl Default for KmlExporter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// KmlExportStyle
// ============================================================================

/// A style definition for KML export.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExportStyle {
    /// Style ID.
    pub id: String,
    /// Icon style (for points).
    pub icon_style: Option<KmlExportIconStyle>,
    /// Line style (for lines).
    pub line_style: Option<KmlExportLineStyle>,
    /// Poly style (for polygons).
    pub poly_style: Option<KmlExportPolyStyle>,
    /// Label style.
    pub label_style: Option<KmlExportLabelStyle>,
}

impl KmlExportStyle {
    /// Create a new style.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            icon_style: None,
            line_style: None,
            poly_style: None,
            label_style: None,
        }
    }

    /// Generate KML for this style.
    pub fn to_kml(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let mut kml = format!("{}<Style id=\"{}\">\n", pad, self.id);

        if let Some(ref icon) = self.icon_style {
            kml.push_str(&format!("{}  <IconStyle>\n", pad));
            kml.push_str(&format!("{}    <color>{}</color>\n", pad, icon.color));
            kml.push_str(&format!("{}    <scale>{}</scale>\n", pad, icon.scale));
            if let Some(ref href) = icon.icon_href {
                kml.push_str(&format!("{}    <Icon><href>{}</href></Icon>\n", pad, href));
            }
            kml.push_str(&format!("{}  </IconStyle>\n", pad));
        }

        if let Some(ref line) = self.line_style {
            kml.push_str(&format!("{}  <LineStyle>\n", pad));
            kml.push_str(&format!("{}    <color>{}</color>\n", pad, line.color));
            kml.push_str(&format!("{}    <width>{}</width>\n", pad, line.width));
            kml.push_str(&format!("{}  </LineStyle>\n", pad));
        }

        if let Some(ref poly) = self.poly_style {
            kml.push_str(&format!("{}  <PolyStyle>\n", pad));
            kml.push_str(&format!("{}    <color>{}</color>\n", pad, poly.color));
            kml.push_str(&format!("{}    <fill>{}</fill>\n", pad, if poly.fill { 1 } else { 0 }));
            kml.push_str(&format!("{}    <outline>{}</outline>\n", pad, if poly.outline { 1 } else { 0 }));
            kml.push_str(&format!("{}  </PolyStyle>\n", pad));
        }

        if let Some(ref label) = self.label_style {
            kml.push_str(&format!("{}  <LabelStyle>\n", pad));
            kml.push_str(&format!("{}    <color>{}</color>\n", pad, label.color));
            kml.push_str(&format!("{}    <scale>{}</scale>\n", pad, label.scale));
            kml.push_str(&format!("{}  </LabelStyle>\n", pad));
        }

        kml.push_str(&format!("{}</Style>\n", pad));
        kml
    }
}

/// Icon style for KML export.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExportIconStyle {
    /// Color in KML format (aabbggrr).
    pub color: String,
    /// Scale factor.
    pub scale: f64,
    /// Icon href.
    pub icon_href: Option<String>,
}

/// Line style for KML export.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExportLineStyle {
    /// Color in KML format (aabbggrr).
    pub color: String,
    /// Line width in pixels.
    pub width: f64,
}

/// Poly style for KML export.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExportPolyStyle {
    /// Color in KML format (aabbggrr).
    pub color: String,
    /// Whether to fill the polygon.
    pub fill: bool,
    /// Whether to draw the outline.
    pub outline: bool,
}

/// Label style for KML export.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExportLabelStyle {
    /// Color in KML format (aabbggrr).
    pub color: String,
    /// Scale factor.
    pub scale: f64,
}

// ============================================================================
// KmlExportPlacemark
// ============================================================================

/// A placemark for KML export.
#[derive(Debug, Clone, PartialEq)]
pub struct KmlExportPlacemark {
    /// Placemark name.
    pub name: String,
    /// Placemark description.
    pub description: Option<String>,
    /// Style URL reference (e.g., "#style1").
    pub style_url: Option<String>,
    /// Geometry.
    pub geometry: KmlExportGeometry,
}

impl KmlExportPlacemark {
    /// Create a new placemark.
    pub fn new(name: &str, geometry: KmlExportGeometry) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            style_url: None,
            geometry,
        }
    }

    /// Generate KML for this placemark.
    pub fn to_kml(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let mut kml = format!("{}<Placemark>\n", pad);
        kml.push_str(&format!("{}  <name>{}</name>\n", pad, escape_xml(&self.name)));

        if let Some(ref desc) = self.description {
            kml.push_str(&format!("{}  <description>{}</description>\n", pad, escape_xml(desc)));
        }

        if let Some(ref style_url) = self.style_url {
            kml.push_str(&format!("{}  <styleUrl>{}</styleUrl>\n", pad, style_url));
        }

        kml.push_str(&self.geometry.to_kml(indent + 2));
        kml.push_str(&format!("{}</Placemark>\n", pad));
        kml
    }
}

/// Geometry types for KML export.
#[derive(Debug, Clone, PartialEq)]
pub enum KmlExportGeometry {
    /// Point geometry.
    Point {
        /// Longitude, latitude, altitude.
        coordinates: Vec<[f64; 3]>,
    },
    /// LineString geometry.
    LineString {
        /// Coordinates.
        coordinates: Vec<[f64; 3]>,
        /// Whether to tessellate (follow terrain).
        tessellate: bool,
    },
    /// Polygon geometry.
    Polygon {
        /// Outer boundary coordinates.
        outer_boundary: Vec<[f64; 3]>,
        /// Inner boundaries (holes).
        inner_boundaries: Vec<Vec<[f64; 3]>>,
    },
    /// Model (glTF) reference.
    Model {
        /// Model href.
        href: String,
        /// Location [lon, lat, alt].
        location: [f64; 3],
        /// Heading in degrees.
        heading: f64,
        /// Tilt in degrees.
        tilt: f64,
        /// Roll in degrees.
        roll: f64,
        /// Scale.
        scale: f64,
    },
}

impl KmlExportGeometry {
    /// Generate KML for this geometry.
    pub fn to_kml(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        match self {
            Self::Point { coordinates } => {
                let mut kml = format!("{}<Point>\n", pad);
                kml.push_str(&format!("{}  <coordinates>{}</coordinates>\n", pad, format_coordinates(coordinates)));
                kml.push_str(&format!("{}</Point>\n", pad));
                kml
            }
            Self::LineString { coordinates, tessellate } => {
                let mut kml = format!("{}<LineString>\n", pad);
                if *tessellate {
                    kml.push_str(&format!("{}  <tessellate>1</tessellate>\n", pad));
                }
                kml.push_str(&format!("{}  <coordinates>{}</coordinates>\n", pad, format_coordinates(coordinates)));
                kml.push_str(&format!("{}</LineString>\n", pad));
                kml
            }
            Self::Polygon { outer_boundary, inner_boundaries } => {
                let mut kml = format!("{}<Polygon>\n", pad);
                kml.push_str(&format!("{}  <outerBoundaryIs>\n", pad));
                kml.push_str(&format!("{}    <LinearRing>\n", pad));
                kml.push_str(&format!("{}      <coordinates>{}</coordinates>\n", pad, format_coordinates(outer_boundary)));
                kml.push_str(&format!("{}    </LinearRing>\n", pad));
                kml.push_str(&format!("{}  </outerBoundaryIs>\n", pad));

                for inner in inner_boundaries {
                    kml.push_str(&format!("{}  <innerBoundaryIs>\n", pad));
                    kml.push_str(&format!("{}    <LinearRing>\n", pad));
                    kml.push_str(&format!("{}      <coordinates>{}</coordinates>\n", pad, format_coordinates(inner)));
                    kml.push_str(&format!("{}    </LinearRing>\n", pad));
                    kml.push_str(&format!("{}  </innerBoundaryIs>\n", pad));
                }

                kml.push_str(&format!("{}</Polygon>\n", pad));
                kml
            }
            Self::Model { href, location, heading, tilt, roll, scale } => {
                let mut kml = format!("{}<Model>\n", pad);
                kml.push_str(&format!("{}  <Location>\n", pad));
                kml.push_str(&format!("{}    <longitude>{}</longitude>\n", pad, location[0]));
                kml.push_str(&format!("{}    <latitude>{}</latitude>\n", pad, location[1]));
                kml.push_str(&format!("{}    <altitude>{}</altitude>\n", pad, location[2]));
                kml.push_str(&format!("{}  </Location>\n", pad));
                kml.push_str(&format!("{}  <Orientation>\n", pad));
                kml.push_str(&format!("{}    <heading>{}</heading>\n", pad, heading));
                kml.push_str(&format!("{}    <tilt>{}</tilt>\n", pad, tilt));
                kml.push_str(&format!("{}    <roll>{}</roll>\n", pad, roll));
                kml.push_str(&format!("{}  </Orientation>\n", pad));
                kml.push_str(&format!("{}  <Scale>\n", pad));
                kml.push_str(&format!("{}    <x>{}</x><y>{}</y><z>{}</z>\n", pad, scale, scale, scale));
                kml.push_str(&format!("{}  </Scale>\n", pad));
                kml.push_str(&format!("{}  <Link><href>{}</href></Link>\n", pad, href));
                kml.push_str(&format!("{}</Model>\n", pad));
                kml
            }
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Format coordinates as KML coordinate string.
fn format_coordinates(coords: &[[f64; 3]]) -> String {
    coords
        .iter()
        .map(|c| format!("{},{},{}", c[0], c[1], c[2]))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Convert RGBA color to KML color format (aabbggrr).
pub fn rgba_to_kml_color(r: f64, g: f64, b: f64, a: f64) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        (a * 255.0) as u8,
        (b * 255.0) as u8,
        (g * 255.0) as u8,
        (r * 255.0) as u8
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kml_export_options_default() {
        let opts = KmlExportOptions::default();
        assert_eq!(opts.name, "Cesium Export");
        assert_eq!(opts.kml_version, "2.2");
        assert!(opts.export_model);
        assert!(opts.export_images);
    }

    #[test]
    fn test_kml_exporter_basic() {
        let exporter = KmlExporter::new();
        let kml = exporter.to_kml();

        assert!(kml.contains("<?xml version=\"1.0\""));
        assert!(kml.contains("<kml"));
        assert!(kml.contains("xmlns=\"http://www.opengis.net/kml/2.2\""));
        assert!(kml.contains("<Document>"));
        assert!(kml.contains("<name>Cesium Export</name>"));
    }

    #[test]
    fn test_kml_exporter_with_placemark() {
        let mut exporter = KmlExporter::new();
        exporter.add_placemark(KmlExportPlacemark::new(
            "Test Point",
            KmlExportGeometry::Point {
                coordinates: vec![[-122.0, 37.0, 0.0]],
            },
        ));

        let kml = exporter.to_kml();
        assert!(kml.contains("<Placemark>"));
        assert!(kml.contains("<name>Test Point</name>"));
        assert!(kml.contains("<Point>"));
        assert!(kml.contains("-122,37,0"));
    }

    #[test]
    fn test_kml_export_style() {
        let style = KmlExportStyle {
            id: "style1".to_string(),
            icon_style: Some(KmlExportIconStyle {
                color: "ff0000ff".to_string(),
                scale: 1.5,
                icon_href: Some("icon.png".to_string()),
            }),
            line_style: None,
            poly_style: None,
            label_style: None,
        };

        let kml = style.to_kml(2);
        assert!(kml.contains("<Style id=\"style1\">"));
        assert!(kml.contains("<IconStyle>"));
        assert!(kml.contains("<color>ff0000ff</color>"));
        assert!(kml.contains("<scale>1.5</scale>"));
    }

    #[test]
    fn test_kml_export_polygon() {
        let geometry = KmlExportGeometry::Polygon {
            outer_boundary: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
            inner_boundaries: vec![],
        };

        let kml = geometry.to_kml(2);
        assert!(kml.contains("<Polygon>"));
        assert!(kml.contains("<outerBoundaryIs>"));
        assert!(kml.contains("<LinearRing>"));
    }

    #[test]
    fn test_kml_export_model() {
        let geometry = KmlExportGeometry::Model {
            href: "model.gltf".to_string(),
            location: [-122.0, 37.0, 100.0],
            heading: 45.0,
            tilt: 0.0,
            roll: 0.0,
            scale: 1.0,
        };

        let kml = geometry.to_kml(2);
        assert!(kml.contains("<Model>"));
        assert!(kml.contains("<Location>"));
        assert!(kml.contains("<longitude>-122</longitude>"));
        assert!(kml.contains("<href>model.gltf</href>"));
    }

    #[test]
    fn test_rgba_to_kml_color() {
        // Red, fully opaque
        assert_eq!(rgba_to_kml_color(1.0, 0.0, 0.0, 1.0), "ff0000ff");
        // Blue, semi-transparent
        assert_eq!(rgba_to_kml_color(0.0, 0.0, 1.0, 0.5), "7fff0000");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_kml_export_result() {
        let exporter = KmlExporter::new();
        let result = exporter.export();

        assert!(!result.kml.is_empty());
        assert!(result.external_files.is_empty());
        assert!(result.kmz.is_none());
    }
}
