//! GPX（GPS Exchange Format）解析器。
//!
//! 映射到 CesiumJS 的 `DataSources/GpxDataSource.js`：
//! - 航点（waypoint）解析
//! - 轨迹（track）解析
//! - 航线（route）解析

use cesium_datasource::entity::{Entity, PointGraphics, PolylineGraphics};
use cesium_datasource::entity_collection::DataSource;
use cesium_datasource::property::{Color, Property};
use cesium_geospatial::cartographic::Cartographic;

/// 一个 GPX 航点（wpt）。
#[derive(Debug, Clone, PartialEq)]
pub struct GpxWaypoint {
    /// 纬度（度）。
    pub latitude: f64,
    /// 经度（度）。
    pub longitude: f64,
    /// 高程（米）。
    pub elevation: Option<f64>,
    /// 时间戳（ISO 8601）。
    pub time: Option<String>,
    /// 名称。
    pub name: Option<String>,
    /// 备注。
    pub comment: Option<String>,
    /// 描述。
    pub description: Option<String>,
    /// 符号名称。
    pub symbol: Option<String>,
    /// 类型。
    pub waypoint_type: Option<String>,
}

impl GpxWaypoint {
    /// 创建一个新的航点。
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            elevation: None,
            time: None,
            name: None,
            comment: None,
            description: None,
            symbol: None,
            waypoint_type: None,
        }
    }

    /// 转换为 Cartographic（弧度）。
    pub fn to_cartographic(&self) -> Cartographic {
        Cartographic::from_radians(
            self.longitude.to_radians(),
            self.latitude.to_radians(),
            self.elevation.unwrap_or(0.0),
        )
    }
}

/// 一个 GPX 轨迹（trk）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpxTrack {
    /// 轨迹名称。
    pub name: Option<String>,
    /// 轨迹备注。
    pub comment: Option<String>,
    /// 轨迹描述。
    pub description: Option<String>,
    /// 轨迹段。
    pub segments: Vec<GpxTrackSegment>,
}

/// 一个 GPX 轨迹段（trkseg）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpxTrackSegment {
    /// 轨迹点。
    pub points: Vec<GpxTrackPoint>,
}

/// 一个 GPX 轨迹点（trkpt）。
#[derive(Debug, Clone, PartialEq)]
pub struct GpxTrackPoint {
    /// 纬度（度）。
    pub latitude: f64,
    /// 经度（度）。
    pub longitude: f64,
    /// 高程（米）。
    pub elevation: Option<f64>,
    /// 时间戳（ISO 8601）。
    pub time: Option<String>,
}

impl GpxTrackPoint {
    /// 创建一个新的轨迹点。
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            elevation: None,
            time: None,
        }
    }

    /// 转换为 Cartographic（弧度）。
    pub fn to_cartographic(&self) -> Cartographic {
        Cartographic::from_radians(
            self.longitude.to_radians(),
            self.latitude.to_radians(),
            self.elevation.unwrap_or(0.0),
        )
    }
}

/// 一个 GPX 航线（rte）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpxRoute {
    /// 航线名称。
    pub name: Option<String>,
    /// 航线备注。
    pub comment: Option<String>,
    /// 航线描述。
    pub description: Option<String>,
    /// 航线点。
    pub points: Vec<GpxRoutePoint>,
}

/// 一个 GPX 航线点（rtept）。
#[derive(Debug, Clone, PartialEq)]
pub struct GpxRoutePoint {
    /// 纬度（度）。
    pub latitude: f64,
    /// 经度（度）。
    pub longitude: f64,
    /// 高程（米）。
    pub elevation: Option<f64>,
    /// 名称。
    pub name: Option<String>,
}

impl GpxRoutePoint {
    /// 创建一个新的航线点。
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            elevation: None,
            name: None,
        }
    }

    /// 转换为 Cartographic（弧度）。
    pub fn to_cartographic(&self) -> Cartographic {
        Cartographic::from_radians(
            self.longitude.to_radians(),
            self.latitude.to_radians(),
            self.elevation.unwrap_or(0.0),
        )
    }
}

/// GPX 元数据。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpxMetadata {
    /// 文档名称。
    pub name: Option<String>,
    /// 文档描述。
    pub description: Option<String>,
    /// 作者名称。
    pub author: Option<String>,
    /// 创建时间（ISO 8601）。
    pub time: Option<String>,
    /// 关键词。
    pub keywords: Option<String>,
}

/// 一个 GPX 文档。
#[derive(Debug, Clone, Default)]
pub struct GpxDocument {
    /// 元数据。
    pub metadata: GpxMetadata,
    /// 航点。
    pub waypoints: Vec<GpxWaypoint>,
    /// 轨迹。
    pub tracks: Vec<GpxTrack>,
    /// 航线。
    pub routes: Vec<GpxRoute>,
}

/// 将一个 GPX 文档转换为 DataSource。
pub fn gpx_to_datasource(doc: &GpxDocument) -> DataSource {
    let name = doc
        .metadata
        .name
        .clone()
        .unwrap_or_else(|| "GPX".to_string());
    let mut ds = DataSource::new(name);

    // 将航点作为 point 添加
    for (i, wpt) in doc.waypoints.iter().enumerate() {
        let mut entity = Entity::new(format!("waypoint_{}", i));
        entity.name = wpt.name.clone();

        entity.position = Property::Constant([
            wpt.longitude.to_radians(),
            wpt.latitude.to_radians(),
            wpt.elevation.unwrap_or(0.0),
        ]);

        entity.point = Some(PointGraphics {
            color: Property::Constant(Color::RED),
            pixel_size: Property::Constant(8.0),
            ..Default::default()
        });

        ds.entities.add(entity);
    }

    // 将轨迹作为折线添加
    for (i, track) in doc.tracks.iter().enumerate() {
        for (j, segment) in track.segments.iter().enumerate() {
            let mut entity = Entity::new(format!("track_{}_{}", i, j));
            entity.name = track.name.clone();

            let positions: Vec<[f64; 3]> = segment
                .points
                .iter()
                .map(|p| {
                    [
                        p.longitude.to_radians(),
                        p.latitude.to_radians(),
                        p.elevation.unwrap_or(0.0),
                    ]
                })
                .collect();

            entity.polyline = Some(PolylineGraphics {
                positions: Property::Constant(positions),
                width: Property::Constant(3.0),
                color: Property::Constant(Color::BLUE),
                ..Default::default()
            });

            ds.entities.add(entity);
        }
    }

    // 将航线作为折线添加
    for (i, route) in doc.routes.iter().enumerate() {
        let mut entity = Entity::new(format!("route_{}", i));
        entity.name = route.name.clone();

        let positions: Vec<[f64; 3]> = route
            .points
            .iter()
            .map(|p| {
                [
                    p.longitude.to_radians(),
                    p.latitude.to_radians(),
                    p.elevation.unwrap_or(0.0),
                ]
            })
            .collect();

        entity.polyline = Some(PolylineGraphics {
            positions: Property::Constant(positions),
            width: Property::Constant(3.0),
            color: Property::Constant(Color::GREEN),
            ..Default::default()
        });

        ds.entities.add(entity);
    }

    ds
}

/// 简单的 GPX 解析器（基础实现）。
pub fn parse_gpx_simple(xml: &str) -> Result<GpxDocument, String> {
    let mut doc = GpxDocument::default();

    // 解析元数据
    if let Some(name) = extract_tag_content(xml, "name") {
        doc.metadata.name = Some(name);
    }
    if let Some(desc) = extract_tag_content(xml, "desc") {
        doc.metadata.description = Some(desc);
    }

    // 解析航点
    for wpt_xml in extract_all_tags(xml, "wpt") {
        let wpt = parse_waypoint(&wpt_xml)?;
        doc.waypoints.push(wpt);
    }

    // 解析轨迹
    for trk_xml in extract_all_tags(xml, "trk") {
        let track = parse_track(&trk_xml)?;
        doc.tracks.push(track);
    }

    // 解析航线
    for rte_xml in extract_all_tags(xml, "rte") {
        let route = parse_route(&rte_xml)?;
        doc.routes.push(route);
    }

    Ok(doc)
}

/// 解析一个航点元素。
fn parse_waypoint(xml: &str) -> Result<GpxWaypoint, String> {
    let lat = extract_attribute(xml, "lat")
        .and_then(|s| s.parse().ok())
        .ok_or("Missing lat attribute")?;
    let lon = extract_attribute(xml, "lon")
        .and_then(|s| s.parse().ok())
        .ok_or("Missing lon attribute")?;

    let mut wpt = GpxWaypoint::new(lat, lon);
    wpt.elevation = extract_tag_content(xml, "ele").and_then(|s| s.parse().ok());
    wpt.time = extract_tag_content(xml, "time");
    wpt.name = extract_tag_content(xml, "name");
    wpt.comment = extract_tag_content(xml, "cmt");
    wpt.description = extract_tag_content(xml, "desc");
    wpt.symbol = extract_tag_content(xml, "sym");
    wpt.waypoint_type = extract_tag_content(xml, "type");

    Ok(wpt)
}

/// 解析一个轨迹元素。
fn parse_track(xml: &str) -> Result<GpxTrack, String> {
    let mut segments = Vec::new();
    for seg_xml in extract_all_tags(xml, "trkseg") {
        let segment = parse_track_segment(&seg_xml)?;
        segments.push(segment);
    }

    Ok(GpxTrack {
        name: extract_tag_content(xml, "name"),
        comment: extract_tag_content(xml, "cmt"),
        description: extract_tag_content(xml, "desc"),
        segments,
    })
}

/// 解析一个轨迹段元素。
fn parse_track_segment(xml: &str) -> Result<GpxTrackSegment, String> {
    let mut segment = GpxTrackSegment::default();

    for pt_xml in extract_all_tags(xml, "trkpt") {
        let lat = extract_attribute(&pt_xml, "lat")
            .and_then(|s| s.parse().ok())
            .ok_or("Missing lat attribute")?;
        let lon = extract_attribute(&pt_xml, "lon")
            .and_then(|s| s.parse().ok())
            .ok_or("Missing lon attribute")?;

        let mut pt = GpxTrackPoint::new(lat, lon);
        pt.elevation = extract_tag_content(&pt_xml, "ele").and_then(|s| s.parse().ok());
        pt.time = extract_tag_content(&pt_xml, "time");

        segment.points.push(pt);
    }

    Ok(segment)
}

/// 解析一个航线元素。
fn parse_route(xml: &str) -> Result<GpxRoute, String> {
    let mut points = Vec::new();
    for pt_xml in extract_all_tags(xml, "rtept") {
        let lat = extract_attribute(&pt_xml, "lat")
            .and_then(|s| s.parse().ok())
            .ok_or("Missing lat attribute")?;
        let lon = extract_attribute(&pt_xml, "lon")
            .and_then(|s| s.parse().ok())
            .ok_or("Missing lon attribute")?;

        let mut pt = GpxRoutePoint::new(lat, lon);
        pt.elevation = extract_tag_content(&pt_xml, "ele").and_then(|s| s.parse().ok());
        pt.name = extract_tag_content(&pt_xml, "name");

        points.push(pt);
    }

    Ok(GpxRoute {
        name: extract_tag_content(xml, "name"),
        comment: extract_tag_content(xml, "cmt"),
        description: extract_tag_content(xml, "desc"),
        points,
    })
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
        // 检查是否为自闭合标签
        let tag_end = match xml[abs_start..].find('>') {
            Some(pos) => pos + abs_start,
            None => break,
        };
        if xml.as_bytes()[tag_end - 1] == b'/' {
            // 自闭合标签
            results.push(xml[abs_start..=tag_end].to_string());
            search_start = tag_end + 1;
        } else if let Some(end) = xml[abs_start..].find(&end_tag) {
            let abs_end = abs_start + end + end_tag.len();
            results.push(xml[abs_start..abs_end].to_string());
            search_start = abs_end;
        } else {
            break;
        }
    }

    results
}

/// 从一个 XML 标签中提取属性值。
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
    fn test_gpx_waypoint_creation() {
        let wpt = GpxWaypoint::new(37.7749, -122.4194);
        assert!((wpt.latitude - 37.7749).abs() < 1e-10);
        assert!((wpt.longitude - (-122.4194)).abs() < 1e-10);
        assert!(wpt.elevation.is_none());
    }

    #[test]
    fn test_gpx_waypoint_to_cartographic() {
        let mut wpt = GpxWaypoint::new(37.7749, -122.4194);
        wpt.elevation = Some(100.0);

        let carto = wpt.to_cartographic();
        assert!((carto.latitude - 37.7749_f64.to_radians()).abs() < 1e-10);
        assert!((carto.longitude - (-122.4194_f64.to_radians())).abs() < 1e-10);
        assert!((carto.height - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_gpx_track_point() {
        let mut pt = GpxTrackPoint::new(37.0, -122.0);
        pt.elevation = Some(50.0);
        pt.time = Some("2024-01-01T12:00:00Z".to_string());

        assert!((pt.latitude - 37.0).abs() < 1e-10);
        assert!(pt.elevation.is_some());
        assert!(pt.time.is_some());
    }

    #[test]
    fn test_gpx_route_point() {
        let mut pt = GpxRoutePoint::new(37.0, -122.0);
        pt.name = Some("Checkpoint".to_string());

        assert!((pt.latitude - 37.0).abs() < 1e-10);
        assert_eq!(pt.name, Some("Checkpoint".to_string()));
    }

    #[test]
    fn test_parse_gpx_simple_waypoint() {
        let gpx = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1">
  <metadata>
    <name>Test GPX</name>
  </metadata>
  <wpt lat="37.7749" lon="-122.4194">
    <ele>10.5</ele>
    <name>San Francisco</name>
    <desc>A city</desc>
  </wpt>
</gpx>"#;

        let doc = parse_gpx_simple(gpx).unwrap();
        assert_eq!(doc.metadata.name, Some("Test GPX".to_string()));
        assert_eq!(doc.waypoints.len(), 1);

        let wpt = &doc.waypoints[0];
        assert!((wpt.latitude - 37.7749).abs() < 1e-10);
        assert!((wpt.longitude - (-122.4194)).abs() < 1e-10);
        assert!((wpt.elevation.unwrap() - 10.5).abs() < 1e-10);
        assert_eq!(wpt.name, Some("San Francisco".to_string()));
    }

    #[test]
    fn test_parse_gpx_simple_track() {
        let gpx = r#"<gpx>
  <trk>
    <name>Morning Run</name>
    <trkseg>
      <trkpt lat="37.0" lon="-122.0">
        <ele>10</ele>
      </trkpt>
      <trkpt lat="37.1" lon="-122.1">
        <ele>20</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let doc = parse_gpx_simple(gpx).unwrap();
        assert_eq!(doc.tracks.len(), 1);

        let track = &doc.tracks[0];
        assert_eq!(track.name, Some("Morning Run".to_string()));
        assert_eq!(track.segments.len(), 1);
        assert_eq!(track.segments[0].points.len(), 2);
    }

    #[test]
    fn test_parse_gpx_simple_route() {
        let gpx = r#"<gpx>
  <rte>
    <name>Route 1</name>
    <rtept lat="37.0" lon="-122.0">
      <name>Start</name>
    </rtept>
    <rtept lat="37.5" lon="-122.5">
      <name>End</name>
    </rtept>
  </rte>
</gpx>"#;

        let doc = parse_gpx_simple(gpx).unwrap();
        assert_eq!(doc.routes.len(), 1);

        let route = &doc.routes[0];
        assert_eq!(route.name, Some("Route 1".to_string()));
        assert_eq!(route.points.len(), 2);
        assert_eq!(route.points[0].name, Some("Start".to_string()));
    }

    #[test]
    fn test_gpx_to_datasource() {
        let doc = GpxDocument {
            metadata: GpxMetadata {
                name: Some("Test".to_string()),
                ..Default::default()
            },
            waypoints: vec![GpxWaypoint::new(37.0, -122.0)],
            tracks: vec![GpxTrack {
                name: Some("Track".to_string()),
                segments: vec![GpxTrackSegment {
                    points: vec![
                        GpxTrackPoint::new(37.0, -122.0),
                        GpxTrackPoint::new(37.1, -122.1),
                    ],
                }],
                ..Default::default()
            }],
            routes: Vec::new(),
        };

        let ds = gpx_to_datasource(&doc);
        assert_eq!(ds.name, "Test");
        assert_eq!(ds.entities.len(), 2); // 1 个航点 + 1 个轨迹段
    }

    #[test]
    fn test_extract_tag_content() {
        let xml = "<name>Test Name</name>";
        assert_eq!(extract_tag_content(xml, "name"), Some("Test Name".to_string()));
    }

    #[test]
    fn test_extract_attribute() {
        let xml = r#"<wpt lat="37.0" lon="-122.0">"#;
        assert_eq!(extract_attribute(xml, "lat"), Some("37.0".to_string()));
        assert_eq!(extract_attribute(xml, "lon"), Some("-122.0".to_string()));
    }

    #[test]
    fn test_gpx_metadata_default() {
        let metadata = GpxMetadata::default();
        assert!(metadata.name.is_none());
        assert!(metadata.description.is_none());
        assert!(metadata.author.is_none());
    }

    #[test]
    fn test_gpx_document_default() {
        let doc = GpxDocument::default();
        assert!(doc.waypoints.is_empty());
        assert!(doc.tracks.is_empty());
        assert!(doc.routes.is_empty());
    }

    #[test]
    fn test_multiple_waypoints() {
        let gpx = r#"<gpx>
  <wpt lat="37.0" lon="-122.0"><name>W1</name></wpt>
  <wpt lat="38.0" lon="-123.0"><name>W2</name></wpt>
  <wpt lat="39.0" lon="-124.0"><name>W3</name></wpt>
</gpx>"#;

        let doc = parse_gpx_simple(gpx).unwrap();
        assert_eq!(doc.waypoints.len(), 3);
        assert_eq!(doc.waypoints[0].name, Some("W1".to_string()));
        assert_eq!(doc.waypoints[2].name, Some("W3".to_string()));
    }
}
