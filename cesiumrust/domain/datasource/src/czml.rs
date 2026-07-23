//! CZML data source parsing (basic).
//!
//! Maps to CesiumJS `DataSources/CzmlDataSource.js`
//! CZML is a JSON format for describing time-dynamic 3D scenes.

use crate::entity::{Entity, PointGraphics, PolylineGraphics, PolygonGraphics};
use crate::entity_collection::DataSource;
use crate::property::{Color, Property};
use serde::Deserialize;
use thiserror::Error;

/// CZML parsing errors.
#[derive(Debug, Error)]
pub enum CzmlError {
    /// JSON parsing error.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// Missing document packet.
    #[error("CZML must start with a document packet (id='document')")]
    MissingDocument,
}

/// A CZML packet.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPacket {
    /// Packet ID.
    pub id: String,

    /// Packet name.
    #[serde(default)]
    pub name: Option<String>,

    /// Position (cartographic degrees: [time, lon, lat, height, ...]).
    #[serde(default)]
    pub position: Option<CzmlPosition>,

    /// Point graphics.
    #[serde(default)]
    pub point: Option<CzmlPoint>,

    /// Polyline graphics.
    #[serde(default)]
    pub polyline: Option<CzmlPolyline>,

    /// Polygon graphics.
    #[serde(default)]
    pub polygon: Option<CzmlPolygon>,

    /// Label.
    #[serde(default)]
    pub label: Option<CzmlLabel>,
}

/// CZML position value.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CzmlPosition {
    /// Cartographic degrees as flat array [lon, lat, height] or time-tagged.
    CartographicDegrees(Vec<f64>),
    /// Object with cartographicDegrees field.
    Object {
        #[serde(rename = "cartographicDegrees")]
        cartographic_degrees: Vec<f64>,
    },
}

/// CZML point graphics.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPoint {
    /// Color as RGBA [r, g, b, a] (0-255).
    #[serde(default)]
    pub color: Option<CzmlColor>,
    /// Pixel size.
    #[serde(default)]
    pub pixel_size: Option<f64>,
    /// Outline color.
    #[serde(default)]
    pub outline_color: Option<CzmlColor>,
    /// Outline width.
    #[serde(default)]
    pub outline_width: Option<f64>,
}

/// CZML polyline graphics.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPolyline {
    /// Positions as cartographic degrees.
    #[serde(default)]
    pub positions: Option<CzmlPosition>,
    /// Width.
    #[serde(default)]
    pub width: Option<f64>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML polygon graphics.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPolygon {
    /// Positions as cartographic degrees.
    #[serde(default)]
    pub positions: Option<CzmlPosition>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
    /// Height.
    #[serde(default)]
    pub height: Option<f64>,
    /// Extruded height.
    #[serde(default)]
    pub extruded_height: Option<f64>,
}

/// CZML label.
#[derive(Debug, Clone, Deserialize)]
pub struct CzmlLabel {
    /// Label text.
    #[serde(default)]
    pub text: Option<String>,
}

/// CZML color value.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CzmlColor {
    /// RGBA array [r, g, b, a] (0-255).
    Rgba(Vec<f64>),
    /// Object with rgba field.
    Object { rgba: Vec<f64> },
}

/// CZML material.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlMaterial {
    /// Solid color.
    #[serde(default)]
    pub solid_color: Option<CzmlSolidColor>,
}

/// CZML solid color material.
#[derive(Debug, Clone, Deserialize)]
pub struct CzmlSolidColor {
    /// Color as RGBA.
    #[serde(default)]
    pub color: Option<CzmlColor>,
}

/// Parses a CZML string into a DataSource.
pub fn parse_czml(json: &str) -> Result<DataSource, CzmlError> {
    let packets: Vec<CzmlPacket> = serde_json::from_str(json)?;

    let mut ds = DataSource::new("CZML");

    for packet in &packets {
        // Skip document packet
        if packet.id == "document" {
            if let Some(ref name) = packet.name {
                ds.name = name.clone();
            }
            continue;
        }

        let entity = process_packet(packet);
        ds.entities.add(entity);
    }

    ds.loaded = true;
    Ok(ds)
}

/// Processes a CZML packet into an Entity.
fn process_packet(packet: &CzmlPacket) -> Entity {
    let mut entity = Entity::new(packet.id.clone());

    if let Some(ref name) = packet.name {
        entity = entity.with_name(name.clone());
    }

    // Process position
    if let Some(ref pos) = packet.position {
        let coords = extract_position_coords(pos);
        if coords.len() >= 3 {
            let lon = coords[0].to_radians();
            let lat = coords[1].to_radians();
            let height = coords[2];
            entity.position = Property::Constant([lon, lat, height]);
        }
    }

    // Process point
    if let Some(ref pt) = packet.point {
        let mut point = PointGraphics::default();
        if let Some(ref color) = pt.color {
            point.color = Property::Constant(czml_color_to_color(color));
        }
        if let Some(size) = pt.pixel_size {
            point.pixel_size = Property::Constant(size);
        }
        if let Some(ref oc) = pt.outline_color {
            point.outline_color = Property::Constant(czml_color_to_color(oc));
        }
        if let Some(ow) = pt.outline_width {
            point.outline_width = Property::Constant(ow);
        }
        entity.point = Some(point);
    }

    // Process polyline
    if let Some(ref pl) = packet.polyline {
        let mut polyline = PolylineGraphics::default();
        if let Some(ref pos) = pl.positions {
            let coords = extract_position_coords(pos);
            let positions = coords_to_positions(&coords);
            polyline.positions = Property::Constant(positions);
        }
        if let Some(width) = pl.width {
            polyline.width = Property::Constant(width);
        }
        if let Some(ref mat) = pl.material {
            if let Some(ref sc) = mat.solid_color {
                if let Some(ref color) = sc.color {
                    polyline.color = Property::Constant(czml_color_to_color(color));
                }
            }
        }
        entity.polyline = Some(polyline);
    }

    // Process polygon
    if let Some(ref pg) = packet.polygon {
        let mut polygon = PolygonGraphics::default();
        if let Some(ref pos) = pg.positions {
            let coords = extract_position_coords(pos);
            let positions = coords_to_positions(&coords);
            polygon.positions = Property::Constant(positions);
        }
        if let Some(ref mat) = pg.material {
            if let Some(ref sc) = mat.solid_color {
                if let Some(ref color) = sc.color {
                    polygon.material = Property::Constant(czml_color_to_color(color));
                }
            }
        }
        if let Some(h) = pg.height {
            polygon.height = Property::Constant(h);
        }
        if let Some(eh) = pg.extruded_height {
            polygon.extruded_height = Property::Constant(eh);
        }
        entity.polygon = Some(polygon);
    }

    entity
}

/// Extracts coordinate values from a CZML position.
fn extract_position_coords(pos: &CzmlPosition) -> Vec<f64> {
    match pos {
        CzmlPosition::CartographicDegrees(v) => v.clone(),
        CzmlPosition::Object { cartographic_degrees } => cartographic_degrees.clone(),
    }
}

/// Converts flat coordinate array [lon, lat, height, lon, lat, height, ...] to positions.
fn coords_to_positions(coords: &[f64]) -> Vec<[f64; 3]> {
    coords
        .chunks(3)
        .filter(|c| c.len() == 3)
        .map(|c| [c[0].to_radians(), c[1].to_radians(), c[2]])
        .collect()
}

/// Converts a CZML color to our Color type.
fn czml_color_to_color(czml_color: &CzmlColor) -> Color {
    let rgba = match czml_color {
        CzmlColor::Rgba(v) => v.clone(),
        CzmlColor::Object { rgba } => rgba.clone(),
    };

    if rgba.len() >= 4 {
        Color::new(
            rgba[0] / 255.0,
            rgba[1] / 255.0,
            rgba[2] / 255.0,
            rgba[3] / 255.0,
        )
    } else {
        Color::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_czml_document() {
        let json = r#"[
            {"id": "document", "name": "Test CZML", "version": "1.0"},
            {"id": "point-1", "name": "My Point", "position": {"cartographicDegrees": [-75.0, 40.0, 100.0]},
             "point": {"color": {"rgba": [255, 0, 0, 255]}, "pixelSize": 10}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        assert_eq!(ds.name, "Test CZML");
        assert_eq!(ds.entities.len(), 1);

        let entity = ds.entities.get("point-1").unwrap();
        assert_eq!(entity.name, Some("My Point".to_string()));
        assert!(entity.point.is_some());
    }

    #[test]
    fn test_parse_czml_polyline() {
        let json = r#"[
            {"id": "document", "name": "Lines"},
            {"id": "line-1", "polyline": {
                "positions": {"cartographicDegrees": [-75.0, 40.0, 0.0, -74.0, 41.0, 0.0]},
                "width": 3.0,
                "material": {"solidColor": {"color": {"rgba": [0, 255, 0, 255]}}}
            }}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("line-1").unwrap();
        assert!(entity.polyline.is_some());

        let polyline = entity.polyline.as_ref().unwrap();
        let positions = polyline.positions.get_value(0.0).unwrap();
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_parse_czml_polygon() {
        let json = r#"[
            {"id": "document", "name": "Polygons"},
            {"id": "poly-1", "polygon": {
                "positions": {"cartographicDegrees": [-75.0, 40.0, 0.0, -74.0, 40.0, 0.0, -74.0, 41.0, 0.0]},
                "material": {"solidColor": {"color": {"rgba": [255, 255, 0, 128]}}},
                "height": 0,
                "extrudedHeight": 10000
            }}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("poly-1").unwrap();
        assert!(entity.polygon.is_some());

        let polygon = entity.polygon.as_ref().unwrap();
        let positions = polygon.positions.get_value(0.0).unwrap();
        assert_eq!(positions.len(), 3);
        let eh = polygon.extruded_height.get_value(0.0).unwrap();
        assert!((*eh - 10000.0).abs() < 1e-10);
    }

    #[test]
    fn test_czml_color_conversion() {
        let color = CzmlColor::Rgba(vec![255.0, 128.0, 0.0, 255.0]);
        let result = czml_color_to_color(&color);
        assert!((result.red - 1.0).abs() < 1e-10);
        assert!((result.green - 128.0 / 255.0).abs() < 1e-10);
        assert!((result.blue - 0.0).abs() < 1e-10);
        assert!((result.alpha - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_coords_to_positions() {
        let coords = vec![-180.0, -90.0, 0.0, 180.0, 90.0, 1000.0];
        let positions = coords_to_positions(&coords);
        assert_eq!(positions.len(), 2);
        assert!((positions[0][0] - (-std::f64::consts::PI)).abs() < 1e-10);
        assert!((positions[1][2] - 1000.0).abs() < 1e-10);
    }
}
