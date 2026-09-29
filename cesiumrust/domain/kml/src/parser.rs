//! KML（Keyhole Markup Language，可标记语言）解析器。
//!
//! 映射到 CesiumJS `DataSources/KmlDataSource.js`：
//! - 地标（Placemark）解析
//! - 几何类型（Point、LineString、Polygon、MultiGeometry）
//! - 样式解析
//! - 扩展数据

use cesium_datasource::entity::{
    Entity, PointGraphics, PolygonGraphics, PolylineGraphics,
};
use cesium_datasource::entity_collection::DataSource;
use cesium_datasource::property::{Color, Property};
use cesium_geospatial::cartographic::Cartographic;

/// KML 坐标（经度、纬度、高度）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KmlCoordinate {
    /// 经度（度）。
    pub longitude: f64,
    /// 纬度（度）。
    pub latitude: f64,
    /// 高度（米）。
    pub altitude: f64,
}

impl KmlCoordinate {
    /// 创建一个新的 KML 坐标。
    pub fn new(longitude: f64, latitude: f64, altitude: f64) -> Self {
        Self {
            longitude,
            latitude,
            altitude,
        }
    }

    /// 转换为 Cartographic（弧度）。
    pub fn to_cartographic(&self) -> Cartographic {
        Cartographic::from_radians(
            self.longitude.to_radians(),
            self.latitude.to_radians(),
            self.altitude,
        )
    }
}

/// KML 几何类型。
#[derive(Debug, Clone, PartialEq)]
pub enum KmlGeometry {
    /// 单个点。
    Point {
        /// 坐标。
        coordinate: KmlCoordinate,
        /// 是否拉伸至地面。
        extrude: bool,
    },
    /// 一条线串。
    LineString {
        /// 坐标。
        coordinates: Vec<KmlCoordinate>,
        /// 是否拉伸至地面。
        extrude: bool,
        /// 细分（沿地形）。
        tessellate: bool,
    },
    /// 一个多边形（外边界 + 可选内边界）。
    Polygon {
        /// 外边界坐标。
        outer: Vec<KmlCoordinate>,
        /// 内边界（空洞）。
        inner: Vec<Vec<KmlCoordinate>>,
        /// 是否拉伸。
        extrude: bool,
        /// 拉伸高度。
        altitude: f64,
    },
    /// 多个几何。
    MultiGeometry {
        /// 子几何。
        geometries: Vec<KmlGeometry>,
    },
}

/// KML 样式定义。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KmlStyle {
    /// 样式 ID（用于引用）。
    pub id: Option<String>,
    /// 图标样式（用于点）。
    pub icon_style: Option<KmlIconStyle>,
    /// 线样式。
    pub line_style: Option<KmlLineStyle>,
    /// 多边形样式。
    pub poly_style: Option<KmlPolyStyle>,
    /// 标注样式。
    pub label_style: Option<KmlLabelStyle>,
}

/// KML 图标样式。
#[derive(Debug, Clone, PartialEq)]
pub struct KmlIconStyle {
    /// 图标颜色（aabbggrr 格式）。
    pub color: Option<String>,
    /// 图标缩放。
    pub scale: f64,
    /// 图标 URL。
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

/// KML 线样式。
#[derive(Debug, Clone, PartialEq)]
pub struct KmlLineStyle {
    /// 线颜色（aabbggrr 格式）。
    pub color: Option<String>,
    /// 线宽（像素）。
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

/// KML 多边形样式。
#[derive(Debug, Clone, PartialEq)]
pub struct KmlPolyStyle {
    /// 填充颜色（aabbggrr 格式）。
    pub color: Option<String>,
    /// 是否填充多边形。
    pub fill: bool,
    /// 是否绘制轮廓。
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

/// KML 标注样式。
#[derive(Debug, Clone, PartialEq)]
pub struct KmlLabelStyle {
    /// 标注颜色（aabbggrr 格式）。
    pub color: Option<String>,
    /// 标注缩放。
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

/// 一个 KML 地标（Placemark）。
#[derive(Debug, Clone)]
pub struct KmlPlacemark {
    /// 地标 ID。
    pub id: Option<String>,
    /// 地标名称。
    pub name: Option<String>,
    /// 地标描述。
    pub description: Option<String>,
    /// 几何。
    pub geometry: Option<KmlGeometry>,
    /// 样式 URL 引用。
    pub style_url: Option<String>,
    /// 内联样式。
    pub style: Option<KmlStyle>,
    /// 扩展数据（键值对）。
    pub extended_data: Vec<(String, String)>,
    /// 可见性。
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

/// KML 文档。
#[derive(Debug, Clone, Default)]
pub struct KmlDocument {
    /// 文档名称。
    pub name: Option<String>,
    /// 文档描述。
    pub description: Option<String>,
    /// 地标。
    pub placemarks: Vec<KmlPlacemark>,
    /// 样式（按 ID）。
    pub styles: Vec<KmlStyle>,
    /// 样式映射（按 ID）。
    pub style_maps: Vec<(String, String, String)>, // (id, normal_style, highlight_style)
}

/// 解析 KML 坐标字符串。
///
/// 格式："lon,lat,alt lon,lat,alt ..."
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

/// 将 KML 颜色（aabbggrr 格式）解析为 RGBA。
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

/// 将 KML 颜色转换为 f32 数组 [r, g, b, a]。
pub fn kml_color_to_f32(color: &str) -> [f32; 4] {
    match parse_kml_color(color) {
        Some(c) => c.to_f32_array(),
        None => [1.0, 1.0, 1.0, 1.0],
    }
}

/// 将 KML 文档转换为 DataSource。
pub fn kml_to_datasource(doc: &KmlDocument) -> DataSource {
    let mut ds = DataSource::new(doc.name.clone().unwrap_or_else(|| "KML".to_string()));

    for placemark in &doc.placemarks {
        if let Some(entity) = placemark_to_entity(placemark) {
            ds.entities.add(entity);
        }
    }

    ds
}

/// 将 KML 地标转换为 Entity。
fn placemark_to_entity(placemark: &KmlPlacemark) -> Option<Entity> {
    let geometry = placemark.geometry.as_ref()?;

    let mut entity = Entity::new(
        placemark.id.clone().unwrap_or_else(|| {
            placemark.name.clone().unwrap_or_else(|| "placemark".to_string())
        }),
    );

    entity.name = placemark.name.clone();
    entity.show = placemark.visibility;

    // 获取样式颜色
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
            // 为简化起见，使用第一个几何
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

/// 获取地标的线颜色与填充颜色。
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

/// 简单的 KML 解析器（基本实现）。
///
/// 这是一个简化版解析器，处理常见的 KML 结构。
/// 生产环境请考虑使用完整的 XML 解析器。
pub fn parse_kml_simple(xml: &str) -> Result<KmlDocument, String> {
    let mut doc = KmlDocument::default();

    // 提取文档名称
    if let Some(name) = extract_tag_content(xml, "name") {
        doc.name = Some(name);
    }

    // 提取地标
    let placemarks = extract_all_tags(xml, "Placemark");
    for pm_xml in placemarks {
        let placemark = parse_placemark(&pm_xml)?;
        doc.placemarks.push(placemark);
    }

    Ok(doc)
}

/// 提取标签之间的内容。
fn extract_tag_content(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;

    Some(xml[start..end].trim().to_string())
}

/// 提取某个标签的所有出现。
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

/// 解析一个地标 XML 片段。
fn parse_placemark(xml: &str) -> Result<KmlPlacemark, String> {
    // 解析几何
    let geometry = if xml.contains("<Point>") {
        Some(parse_point_geometry(xml))
    } else if xml.contains("<LineString>") {
        Some(parse_linestring_geometry(xml))
    } else if xml.contains("<Polygon>") {
        Some(parse_polygon_geometry(xml))
    } else {
        None
    };

    // 解析样式
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

/// 解析一个 Point 几何。
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

/// 解析一个 LineString 几何。
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

/// 解析一个 Polygon 几何。
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

/// 解析一个 Style 元素。
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

/// 从 XML 标签中提取属性值。
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
        // KML 颜色格式：aabbggrr
        let color = parse_kml_color("ff0000ff").unwrap(); // 红色，完全不透明
        assert!((color.red - 1.0).abs() < 1e-10);
        assert!((color.green - 0.0).abs() < 1e-10);
        assert!((color.blue - 0.0).abs() < 1e-10);
        assert!((color.alpha - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_kml_color_green() {
        let color = parse_kml_color("8000ff00").unwrap(); // 绿色，50% 透明
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
