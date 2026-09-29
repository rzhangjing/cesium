//! GeoJSON 数据源解析。
//!
//! 映射到 CesiumJS `DataSources/GeoJsonDataSource.js`
//! 将 GeoJSON（RFC 7946）解析为实体。

use crate::entity::{Entity, PointGraphics, PolygonGraphics, PolylineGraphics};
use crate::entity_collection::DataSource;
use crate::property::{Color, Property};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// GeoJSON 解析错误。
#[derive(Debug, Error)]
pub enum GeoJsonError {
    /// JSON 解析错误。
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// 不支持的几何类型。
    #[error("Unsupported geometry type: {0}")]
    UnsupportedGeometry(String),

    /// 无效的坐标。
    #[error("Invalid coordinate at index {0}")]
    InvalidCoordinate(usize),
}

/// 一个 GeoJSON 对象（顶层）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GeoJson {
    /// 单个 feature。
    Feature(Feature),
    /// feature 的集合。
    FeatureCollection(FeatureCollection),
    /// Point 几何对象。
    Point(PointGeometry),
    /// MultiPoint 几何。
    MultiPoint(MultiPointGeometry),
    /// LineString 几何。
    LineString(LineStringGeometry),
    /// MultiLineString 几何。
    MultiLineString(MultiLineStringGeometry),
    /// Polygon 几何。
    Polygon(PolygonGeometry),
    /// MultiPolygon 几何。
    MultiPolygon(MultiPolygonGeometry),
}

/// 一个 GeoJSON FeatureCollection。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureCollection {
    /// 此集合中的 features。
    pub features: Vec<Feature>,
}

/// 一个 GeoJSON Feature。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    /// 几何（可为 null）。
    pub geometry: Option<Geometry>,
    /// feature 属性。
    #[serde(default)]
    pub properties: serde_json::Value,
    /// feature ID。
    #[serde(default)]
    pub id: Option<serde_json::Value>,
}

/// 一个 GeoJSON Geometry。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Geometry {
    /// Point 几何。
    Point(PointGeometry),
    /// MultiPoint 几何。
    MultiPoint(MultiPointGeometry),
    /// LineString 几何。
    LineString(LineStringGeometry),
    /// MultiLineString 几何。
    MultiLineString(MultiLineStringGeometry),
    /// Polygon 几何。
    Polygon(PolygonGeometry),
    /// MultiPolygon 几何。
    MultiPolygon(MultiPolygonGeometry),
    /// GeometryCollection。
    GeometryCollection(GeometryCollection),
}

/// Point 几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointGeometry {
    /// [经度, 纬度, 可选高度]
    pub coordinates: Vec<f64>,
}

/// MultiPoint 几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPointGeometry {
    /// 位置数组。
    pub coordinates: Vec<Vec<f64>>,
}

/// LineString 几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineStringGeometry {
    /// 位置数组。
    pub coordinates: Vec<Vec<f64>>,
}

/// MultiLineString 几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLineStringGeometry {
    /// LineString 坐标数组的数组。
    pub coordinates: Vec<Vec<Vec<f64>>>,
}

/// Polygon 几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonGeometry {
    /// 环的数组（第一个为外环，其余为内孔）。
    pub coordinates: Vec<Vec<Vec<f64>>>,
}

/// MultiPolygon 几何。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPolygonGeometry {
    /// Polygon 坐标数组的数组。
    pub coordinates: Vec<Vec<Vec<Vec<f64>>>>,
}

/// GeometryCollection。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryCollection {
    /// 此集合中的几何。
    pub geometries: Vec<Geometry>,
}

/// GeoJSON 加载的选项。
#[derive(Debug, Clone)]
pub struct GeoJsonOptions {
    /// 点的默认标记颜色。
    pub marker_color: Color,
    /// 默认标记尺寸（像素）。
    pub marker_size: f64,
    /// 线/轮廓的默认描边颜色。
    pub stroke: Color,
    /// 默认描边宽度。
    pub stroke_width: f64,
    /// 多边形的默认填充颜色。
    pub fill: Color,
    /// 是否贴合地面。
    pub clamp_to_ground: bool,
}

impl Default for GeoJsonOptions {
    fn default() -> Self {
        Self {
            marker_color: Color::RED,
            marker_size: 8.0,
            stroke: Color::YELLOW,
            stroke_width: 2.0,
            fill: Color::new(1.0, 1.0, 0.0, 0.5), // 半透明黄色
            clamp_to_ground: false,
        }
    }
}

/// 将 GeoJSON 字符串解析为 DataSource。
pub fn parse_geojson(json: &str, options: &GeoJsonOptions) -> Result<DataSource, GeoJsonError> {
    let geojson: GeoJson = serde_json::from_str(json)?;
    let mut ds = DataSource::new("GeoJSON");

    let mut id_counter = 0u64;
    process_geojson(&geojson, options, &mut ds, &mut id_counter)?;

    ds.loaded = true;
    Ok(ds)
}

/// 递归地处理一个 GeoJSON 对象。
fn process_geojson(
    geojson: &GeoJson,
    options: &GeoJsonOptions,
    ds: &mut DataSource,
    id_counter: &mut u64,
) -> Result<(), GeoJsonError> {
    match geojson {
        GeoJson::FeatureCollection(fc) => {
            for feature in &fc.features {
                process_feature(feature, options, ds, id_counter)?;
            }
        }
        GeoJson::Feature(feature) => {
            process_feature(feature, options, ds, id_counter)?;
        }
        GeoJson::Point(pt) => {
            let entity = create_point_entity(*id_counter, &pt.coordinates, None, options);
            ds.entities.add(entity);
            *id_counter += 1;
        }
        GeoJson::MultiPoint(mpt) => {
            for coord in &mpt.coordinates {
                let entity = create_point_entity(*id_counter, coord, None, options);
                ds.entities.add(entity);
                *id_counter += 1;
            }
        }
        GeoJson::LineString(ls) => {
            let entity = create_polyline_entity(*id_counter, &ls.coordinates, None, options);
            ds.entities.add(entity);
            *id_counter += 1;
        }
        GeoJson::MultiLineString(mls) => {
            for line in &mls.coordinates {
                let entity = create_polyline_entity(*id_counter, line, None, options);
                ds.entities.add(entity);
                *id_counter += 1;
            }
        }
        GeoJson::Polygon(poly) => {
            let entity = create_polygon_entity(*id_counter, &poly.coordinates, None, options);
            ds.entities.add(entity);
            *id_counter += 1;
        }
        GeoJson::MultiPolygon(mpoly) => {
            for polygon in &mpoly.coordinates {
                let entity = create_polygon_entity(*id_counter, polygon, None, options);
                ds.entities.add(entity);
                *id_counter += 1;
            }
        }
    }
    Ok(())
}

/// 处理一个 GeoJSON feature。
fn process_feature(
    feature: &Feature,
    options: &GeoJsonOptions,
    ds: &mut DataSource,
    id_counter: &mut u64,
) -> Result<(), GeoJsonError> {
    let geometry = match &feature.geometry {
        Some(g) => g,
        None => return Ok(()), // 跳过没有几何的 feature
    };

    let name = extract_name(&feature.properties);
    let properties = extract_properties(&feature.properties);

    match geometry {
        Geometry::Point(pt) => {
            let entity = create_point_entity(*id_counter, &pt.coordinates, name.as_deref(), options)
                .with_properties(properties);
            ds.entities.add(entity);
            *id_counter += 1;
        }
        Geometry::MultiPoint(mpt) => {
            for coord in &mpt.coordinates {
                let entity =
                    create_point_entity(*id_counter, coord, name.as_deref(), options);
                ds.entities.add(entity);
                *id_counter += 1;
            }
        }
        Geometry::LineString(ls) => {
            let entity =
                create_polyline_entity(*id_counter, &ls.coordinates, name.as_deref(), options)
                    .with_properties(properties);
            ds.entities.add(entity);
            *id_counter += 1;
        }
        Geometry::MultiLineString(mls) => {
            for line in &mls.coordinates {
                let entity =
                    create_polyline_entity(*id_counter, line, name.as_deref(), options);
                ds.entities.add(entity);
                *id_counter += 1;
            }
        }
        Geometry::Polygon(poly) => {
            let entity =
                create_polygon_entity(*id_counter, &poly.coordinates, name.as_deref(), options)
                    .with_properties(properties);
            ds.entities.add(entity);
            *id_counter += 1;
        }
        Geometry::MultiPolygon(mpoly) => {
            for polygon in &mpoly.coordinates {
                let entity =
                    create_polygon_entity(*id_counter, polygon, name.as_deref(), options);
                ds.entities.add(entity);
                *id_counter += 1;
            }
        }
        Geometry::GeometryCollection(gc) => {
            for geom in &gc.geometries {
                let feature = Feature {
                    geometry: Some(geom.clone()),
                    properties: feature.properties.clone(),
                    id: feature.id.clone(),
                };
                process_feature(&feature, options, ds, id_counter)?;
            }
        }
    }
    Ok(())
}

/// 将一个 GeoJSON 位置 [lon_deg, lat_deg, alt?] 转换为弧度 [lon_rad, lat_rad, height]。
fn position_to_radians(coord: &[f64]) -> [f64; 3] {
    let lon = coord.first().copied().unwrap_or(0.0).to_radians();
    let lat = coord.get(1).copied().unwrap_or(0.0).to_radians();
    let height = coord.get(2).copied().unwrap_or(0.0);
    [lon, lat, height]
}

/// 由一个 GeoJSON 位置创建点实体。
fn create_point_entity(
    id: u64,
    coord: &[f64],
    name: Option<&str>,
    options: &GeoJsonOptions,
) -> Entity {
    let pos = position_to_radians(coord);
    let mut entity = Entity::new(format!("geojson-{}", id))
        .with_position(pos[0], pos[1], pos[2])
        .with_point(PointGraphics {
            color: Property::Constant(options.marker_color),
            pixel_size: Property::Constant(options.marker_size),
            ..Default::default()
        });

    if let Some(n) = name {
        entity = entity.with_name(n);
    }
    entity
}

/// 由 GeoJSON 坐标创建一个 polyline 实体。
fn create_polyline_entity(
    id: u64,
    coords: &[Vec<f64>],
    name: Option<&str>,
    options: &GeoJsonOptions,
) -> Entity {
    let positions: Vec<[f64; 3]> = coords.iter().map(|c| position_to_radians(c)).collect();

    let mut entity = Entity::new(format!("geojson-{}", id)).with_polyline(PolylineGraphics {
        positions: Property::Constant(positions),
        width: Property::Constant(options.stroke_width),
        color: Property::Constant(options.stroke),
        clamp_to_ground: Property::Constant(options.clamp_to_ground),
        ..Default::default()
    });

    if let Some(n) = name {
        entity = entity.with_name(n);
    }
    entity
}

/// 由 GeoJSON 坐标创建一个多边形实体。
fn create_polygon_entity(
    id: u64,
    rings: &[Vec<Vec<f64>>],
    name: Option<&str>,
    options: &GeoJsonOptions,
) -> Entity {
    let exterior: Vec<[f64; 3]> = rings
        .first()
        .map(|ring| ring.iter().map(|c| position_to_radians(c)).collect())
        .unwrap_or_default();

    let holes: Vec<Vec<[f64; 3]>> = rings
        .iter()
        .skip(1)
        .map(|ring| ring.iter().map(|c| position_to_radians(c)).collect())
        .collect();

    let mut entity = Entity::new(format!("geojson-{}", id)).with_polygon(PolygonGraphics {
        positions: Property::Constant(exterior),
        holes,
        material: Property::Constant(options.fill),
        outline: Property::Constant(true),
        outline_color: Property::Constant(options.stroke),
        outline_width: Property::Constant(options.stroke_width),
        ..Default::default()
    });

    if let Some(n) = name {
        entity = entity.with_name(n);
    }
    entity
}

/// 从 GeoJSON 属性中提取名称。
fn extract_name(properties: &serde_json::Value) -> Option<String> {
    if let Some(obj) = properties.as_object() {
        // 尝试常见的名称字段
        for key in &["name", "NAME", "Name", "title", "TITLE"] {
            if let Some(serde_json::Value::String(s)) = obj.get(*key) {
                return Some(s.clone());
            }
        }
    }
    None
}

/// 将所有属性提取为一个 HashMap。
fn extract_properties(
    properties: &serde_json::Value,
) -> std::collections::HashMap<String, serde_json::Value> {
    match properties.as_object() {
        Some(obj) => obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        None => std::collections::HashMap::new(),
    }
}

/// 供 Entity 批量添加属性的辅助 trait。
trait EntityExt {
    fn with_properties(
        self,
        properties: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self;
}

impl EntityExt for Entity {
    fn with_properties(
        mut self,
        properties: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        self.properties = properties;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_point() {
        let json = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [102.0, 0.5]
            },
            "properties": {
                "name": "Test Point"
            }
        }"#;

        let ds = parse_geojson(json, &GeoJsonOptions::default()).unwrap();
        assert_eq!(ds.entities.len(), 1);

        let entity = ds.entities.values().next().unwrap();
        assert_eq!(entity.name, Some("Test Point".to_string()));
        assert!(entity.point.is_some());
    }

    #[test]
    fn test_parse_linestring() {
        let json = r#"{
            "type": "Feature",
            "geometry": {
                "type": "LineString",
                "coordinates": [[102.0, 0.0], [103.0, 1.0], [104.0, 0.0]]
            },
            "properties": {}
        }"#;

        let ds = parse_geojson(json, &GeoJsonOptions::default()).unwrap();
        assert_eq!(ds.entities.len(), 1);

        let entity = ds.entities.values().next().unwrap();
        assert!(entity.polyline.is_some());
        let positions = entity.polyline.as_ref().unwrap().positions.get_value(0.0).unwrap();
        assert_eq!(positions.len(), 3);
    }

    #[test]
    fn test_parse_polygon() {
        let json = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0]]]
            },
            "properties": {"name": "Test Polygon"}
        }"#;

        let ds = parse_geojson(json, &GeoJsonOptions::default()).unwrap();
        assert_eq!(ds.entities.len(), 1);

        let entity = ds.entities.values().next().unwrap();
        assert!(entity.polygon.is_some());
        assert_eq!(entity.name, Some("Test Polygon".to_string()));
    }

    #[test]
    fn test_parse_feature_collection() {
        let json = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [0.0, 0.0]},
                    "properties": {}
                },
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [1.0, 1.0]},
                    "properties": {}
                }
            ]
        }"#;

        let ds = parse_geojson(json, &GeoJsonOptions::default()).unwrap();
        assert_eq!(ds.entities.len(), 2);
    }

    #[test]
    fn test_parse_polygon_with_hole() {
        let json = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [
                    [[100.0, 0.0], [101.0, 0.0], [101.0, 1.0], [100.0, 1.0], [100.0, 0.0]],
                    [[100.2, 0.2], [100.8, 0.2], [100.8, 0.8], [100.2, 0.8], [100.2, 0.2]]
                ]
            },
            "properties": {}
        }"#;

        let ds = parse_geojson(json, &GeoJsonOptions::default()).unwrap();
        let entity = ds.entities.values().next().unwrap();
        let polygon = entity.polygon.as_ref().unwrap();
        assert_eq!(polygon.holes.len(), 1);
        assert_eq!(polygon.holes[0].len(), 5);
    }

    #[test]
    fn test_position_to_radians() {
        let pos = position_to_radians(&[180.0, 90.0, 1000.0]);
        assert!((pos[0] - std::f64::consts::PI).abs() < 1e-10);
        assert!((pos[1] - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
        assert!((pos[2] - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_custom_options() {
        let json = r#"{
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [0.0, 0.0]},
            "properties": {}
        }"#;

        let options = GeoJsonOptions {
            marker_color: Color::BLUE,
            marker_size: 20.0,
            ..Default::default()
        };

        let ds = parse_geojson(json, &options).unwrap();
        let entity = ds.entities.values().next().unwrap();
        let point = entity.point.as_ref().unwrap();
        let color = point.color.get_value(0.0).unwrap();
        assert!((color.blue - 1.0).abs() < 1e-10);
        let size = point.pixel_size.get_value(0.0).unwrap();
        assert!((*size - 20.0).abs() < 1e-10);
    }
}
