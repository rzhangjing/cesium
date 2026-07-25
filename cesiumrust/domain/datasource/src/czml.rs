//! CZML data source parsing.
//!
//! Maps to CesiumJS `DataSources/CzmlDataSource.js`
//! CZML is a JSON format for describing time-dynamic 3D scenes.

use crate::entity::{
    Entity, PointGraphics, PolylineGraphics, PolygonGraphics,
    BillboardGraphics, LabelGraphics, ModelGraphics, EllipseGraphics,
    BoxGraphics, CylinderGraphics, CorridorGraphics, RectangleGraphics,
    WallGraphics, EllipsoidGraphics, PathGraphics,
};
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

    /// Billboard.
    #[serde(default)]
    pub billboard: Option<CzmlBillboard>,

    /// Model.
    #[serde(default)]
    pub model: Option<CzmlModel>,

    /// Ellipse.
    #[serde(default)]
    pub ellipse: Option<CzmlEllipse>,

    /// Box.
    #[serde(default, rename = "box")]
    pub box_graphics: Option<CzmlBox>,

    /// Cylinder.
    #[serde(default)]
    pub cylinder: Option<CzmlCylinder>,

    /// Corridor.
    #[serde(default)]
    pub corridor: Option<CzmlCorridor>,

    /// Rectangle.
    #[serde(default)]
    pub rectangle: Option<CzmlRectangle>,

    /// Wall.
    #[serde(default)]
    pub wall: Option<CzmlWall>,

    /// Ellipsoid.
    #[serde(default)]
    pub ellipsoid: Option<CzmlEllipsoid>,

    /// Path.
    #[serde(default)]
    pub path: Option<CzmlPath>,

    /// Availability (ISO 8601 time interval string).
    #[serde(default)]
    pub availability: Option<String>,

    /// Description.
    #[serde(default)]
    pub description: Option<String>,
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
    /// Font.
    #[serde(default)]
    pub font: Option<String>,
    /// Fill color.
    #[serde(default, rename = "fillColor")]
    pub fill_color: Option<CzmlColor>,
    /// Outline color.
    #[serde(default, rename = "outlineColor")]
    pub outline_color: Option<CzmlColor>,
}

/// CZML billboard.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlBillboard {
    /// Image URI.
    #[serde(default)]
    pub image: Option<String>,
    /// Scale.
    #[serde(default)]
    pub scale: Option<f64>,
    /// Color.
    #[serde(default)]
    pub color: Option<CzmlColor>,
    /// Rotation.
    #[serde(default)]
    pub rotation: Option<f64>,
    /// Width.
    #[serde(default)]
    pub width: Option<f64>,
    /// Height.
    #[serde(default)]
    pub height: Option<f64>,
}

/// CZML model.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlModel {
    /// Model URI (glTF/glb).
    #[serde(default)]
    pub gltf: Option<String>,
    /// Scale.
    #[serde(default)]
    pub scale: Option<f64>,
    /// Minimum pixel size.
    #[serde(default)]
    pub minimum_pixel_size: Option<f64>,
}

/// CZML ellipse.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlEllipse {
    /// Semi-major axis.
    #[serde(default)]
    pub semi_major_axis: Option<f64>,
    /// Semi-minor axis.
    #[serde(default)]
    pub semi_minor_axis: Option<f64>,
    /// Height.
    #[serde(default)]
    pub height: Option<f64>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML box.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlBox {
    /// Dimensions [x, y, z].
    #[serde(default)]
    pub dimensions: Option<CzmlCartesian3Value>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML cylinder.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlCylinder {
    /// Length.
    #[serde(default)]
    pub length: Option<f64>,
    /// Top radius.
    #[serde(default)]
    pub top_radius: Option<f64>,
    /// Bottom radius.
    #[serde(default)]
    pub bottom_radius: Option<f64>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML corridor.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlCorridor {
    /// Positions.
    #[serde(default)]
    pub positions: Option<CzmlPosition>,
    /// Width.
    #[serde(default)]
    pub width: Option<f64>,
    /// Height.
    #[serde(default)]
    pub height: Option<f64>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML rectangle.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlRectangle {
    /// Coordinates [west, south, east, north] in degrees.
    #[serde(default)]
    pub coordinates: Option<CzmlRectangleCoords>,
    /// Height.
    #[serde(default)]
    pub height: Option<f64>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML rectangle coordinates.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CzmlRectangleCoords {
    /// Flat array [west, south, east, north] in degrees.
    Array(Vec<f64>),
    /// Object with degrees field.
    Object { degrees: Vec<f64> },
}

/// CZML wall.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlWall {
    /// Positions.
    #[serde(default)]
    pub positions: Option<CzmlPosition>,
    /// Maximum heights.
    #[serde(default)]
    pub maximum_heights: Option<Vec<f64>>,
    /// Minimum heights.
    #[serde(default)]
    pub minimum_heights: Option<Vec<f64>>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML ellipsoid.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlEllipsoid {
    /// Radii [x, y, z].
    #[serde(default)]
    pub radii: Option<CzmlCartesian3Value>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML path.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPath {
    /// Lead time.
    #[serde(default)]
    pub lead_time: Option<f64>,
    /// Trail time.
    #[serde(default)]
    pub trail_time: Option<f64>,
    /// Width.
    #[serde(default)]
    pub width: Option<f64>,
    /// Material.
    #[serde(default)]
    pub material: Option<CzmlMaterial>,
}

/// CZML Cartesian3 value.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CzmlCartesian3Value {
    /// Flat array [x, y, z].
    Array(Vec<f64>),
    /// Object with cartesian3 field.
    Object { cartesian3: Vec<f64> },
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

    if let Some(ref desc) = packet.description {
        entity.description = Some(desc.clone());
    }

    // Process position
    if let Some(ref pos) = packet.position {
        let coords = extract_position_coords(pos);
        if coords.len() >= 3 {
            // Check if time-tagged (length > 3 and first value looks like time)
            if coords.len() > 3 && coords.len().is_multiple_of(4) {
                // Time-tagged: [time, lon, lat, height, time, lon, lat, height, ...]
                let samples: Vec<(f64, [f64; 3])> = coords
                    .chunks(4)
                    .filter(|c| c.len() == 4)
                    .map(|c| (c[0], [c[1].to_radians(), c[2].to_radians(), c[3]]))
                    .collect();
                entity.position = Property::Sampled(samples);
            } else {
                let lon = coords[0].to_radians();
                let lat = coords[1].to_radians();
                let height = coords[2];
                entity.position = Property::Constant([lon, lat, height]);
            }
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
            if let Some(color) = extract_material_color(mat) {
                polyline.color = Property::Constant(color);
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
            if let Some(color) = extract_material_color(mat) {
                polygon.material = Property::Constant(color);
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

    // Process label
    if let Some(ref lb) = packet.label {
        let mut label = LabelGraphics::default();
        if let Some(ref text) = lb.text {
            label.text = Property::Constant(text.clone());
        }
        if let Some(ref font) = lb.font {
            label.font = Property::Constant(font.clone());
        }
        if let Some(ref fc) = lb.fill_color {
            label.fill_color = Property::Constant(czml_color_to_color(fc));
        }
        if let Some(ref oc) = lb.outline_color {
            label.outline_color = Property::Constant(czml_color_to_color(oc));
        }
        entity.label = Some(label);
    }

    // Process billboard
    if let Some(ref bb) = packet.billboard {
        let mut billboard = BillboardGraphics::default();
        if let Some(ref image) = bb.image {
            billboard.image = Property::Constant(image.clone());
        }
        if let Some(scale) = bb.scale {
            billboard.scale = Property::Constant(scale);
        }
        if let Some(ref color) = bb.color {
            billboard.color = Property::Constant(czml_color_to_color(color));
        }
        if let Some(rotation) = bb.rotation {
            billboard.rotation = Property::Constant(rotation);
        }
        if let Some(w) = bb.width {
            billboard.width = Property::Constant(w);
        }
        if let Some(h) = bb.height {
            billboard.height = Property::Constant(h);
        }
        entity.billboard = Some(billboard);
    }

    // Process model
    if let Some(ref mdl) = packet.model {
        let mut model = ModelGraphics::default();
        if let Some(ref gltf) = mdl.gltf {
            model.uri = Property::Constant(gltf.clone());
        }
        if let Some(scale) = mdl.scale {
            model.scale = Property::Constant(scale);
        }
        if let Some(mps) = mdl.minimum_pixel_size {
            model.minimum_pixel_size = Property::Constant(mps);
        }
        entity.model = Some(model);
    }

    // Process ellipse
    if let Some(ref ell) = packet.ellipse {
        let mut ellipse = EllipseGraphics::default();
        if let Some(sma) = ell.semi_major_axis {
            ellipse.semi_major_axis = Property::Constant(sma);
        }
        if let Some(smi) = ell.semi_minor_axis {
            ellipse.semi_minor_axis = Property::Constant(smi);
        }
        if let Some(h) = ell.height {
            ellipse.height = Property::Constant(h);
        }
        if let Some(ref mat) = ell.material {
            if let Some(color) = extract_material_color(mat) {
                ellipse.material = Property::Constant(color);
            }
        }
        entity.ellipse = Some(ellipse);
    }

    // Process box
    if let Some(ref bx) = packet.box_graphics {
        let mut box_g = BoxGraphics::default();
        if let Some(ref dims) = bx.dimensions {
            let v = extract_cartesian3(dims);
            if v.len() >= 3 {
                box_g.dimensions = Property::Constant([v[0], v[1], v[2]]);
            }
        }
        if let Some(ref mat) = bx.material {
            if let Some(color) = extract_material_color(mat) {
                box_g.material = Property::Constant(color);
            }
        }
        entity.box_graphics = Some(box_g);
    }

    // Process cylinder
    if let Some(ref cyl) = packet.cylinder {
        let mut cylinder = CylinderGraphics::default();
        if let Some(l) = cyl.length {
            cylinder.length = Property::Constant(l);
        }
        if let Some(tr) = cyl.top_radius {
            cylinder.top_radius = Property::Constant(tr);
        }
        if let Some(br) = cyl.bottom_radius {
            cylinder.bottom_radius = Property::Constant(br);
        }
        if let Some(ref mat) = cyl.material {
            if let Some(color) = extract_material_color(mat) {
                cylinder.material = Property::Constant(color);
            }
        }
        entity.cylinder = Some(cylinder);
    }

    // Process corridor
    if let Some(ref cor) = packet.corridor {
        let mut corridor = CorridorGraphics::default();
        if let Some(ref pos) = cor.positions {
            let coords = extract_position_coords(pos);
            let positions = coords_to_positions(&coords);
            corridor.positions = Property::Constant(positions);
        }
        if let Some(w) = cor.width {
            corridor.width = Property::Constant(w);
        }
        if let Some(h) = cor.height {
            corridor.height = Property::Constant(h);
        }
        if let Some(ref mat) = cor.material {
            if let Some(color) = extract_material_color(mat) {
                corridor.material = Property::Constant(color);
            }
        }
        entity.corridor = Some(corridor);
    }

    // Process rectangle
    if let Some(ref rect) = packet.rectangle {
        let mut rectangle = RectangleGraphics::default();
        if let Some(ref coords) = rect.coordinates {
            let v = match coords {
                CzmlRectangleCoords::Array(a) => a.clone(),
                CzmlRectangleCoords::Object { degrees } => degrees.clone(),
            };
            if v.len() >= 4 {
                rectangle.coordinates = Property::Constant([
                    v[0].to_radians(), v[1].to_radians(),
                    v[2].to_radians(), v[3].to_radians(),
                ]);
            }
        }
        if let Some(h) = rect.height {
            rectangle.height = Property::Constant(h);
        }
        if let Some(ref mat) = rect.material {
            if let Some(color) = extract_material_color(mat) {
                rectangle.material = Property::Constant(color);
            }
        }
        entity.rectangle = Some(rectangle);
    }

    // Process wall
    if let Some(ref wl) = packet.wall {
        let mut wall = WallGraphics::default();
        if let Some(ref pos) = wl.positions {
            let coords = extract_position_coords(pos);
            let positions = coords_to_positions(&coords);
            wall.positions = Property::Constant(positions);
        }
        if let Some(ref mh) = wl.maximum_heights {
            wall.maximum_heights = Property::Constant(mh.clone());
        }
        if let Some(ref mh) = wl.minimum_heights {
            wall.minimum_heights = Property::Constant(mh.clone());
        }
        if let Some(ref mat) = wl.material {
            if let Some(color) = extract_material_color(mat) {
                wall.material = Property::Constant(color);
            }
        }
        entity.wall = Some(wall);
    }

    // Process ellipsoid
    if let Some(ref el) = packet.ellipsoid {
        let mut ellipsoid = EllipsoidGraphics::default();
        if let Some(ref radii) = el.radii {
            let v = extract_cartesian3(radii);
            if v.len() >= 3 {
                ellipsoid.radii = Property::Constant([v[0], v[1], v[2]]);
            }
        }
        if let Some(ref mat) = el.material {
            if let Some(color) = extract_material_color(mat) {
                ellipsoid.material = Property::Constant(color);
            }
        }
        entity.ellipsoid = Some(ellipsoid);
    }

    // Process path
    if let Some(ref pth) = packet.path {
        let mut path = PathGraphics::default();
        if let Some(lt) = pth.lead_time {
            path.lead_time = Property::Constant(lt);
        }
        if let Some(tt) = pth.trail_time {
            path.trail_time = Property::Constant(tt);
        }
        if let Some(w) = pth.width {
            path.width = Property::Constant(w);
        }
        if let Some(ref mat) = pth.material {
            if let Some(color) = extract_material_color(mat) {
                path.material = Property::Constant(color);
            }
        }
        entity.path = Some(path);
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

/// Extracts a color from a CZML material.
fn extract_material_color(mat: &CzmlMaterial) -> Option<Color> {
    mat.solid_color
        .as_ref()
        .and_then(|sc| sc.color.as_ref())
        .map(czml_color_to_color)
}

/// Extracts a Cartesian3 value from a CZML Cartesian3.
fn extract_cartesian3(val: &CzmlCartesian3Value) -> Vec<f64> {
    match val {
        CzmlCartesian3Value::Array(v) => v.clone(),
        CzmlCartesian3Value::Object { cartesian3 } => cartesian3.clone(),
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

    #[test]
    fn test_parse_czml_billboard() {
        let json = r#"[
            {"id": "document", "name": "Billboards"},
            {"id": "bb-1", "position": {"cartographicDegrees": [-75.0, 40.0, 0.0]},
             "billboard": {"image": "marker.png", "scale": 2.0, "color": {"rgba": [255, 0, 0, 255]}}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("bb-1").unwrap();
        assert!(entity.billboard.is_some());
        let bb = entity.billboard.as_ref().unwrap();
        assert_eq!(bb.image.get_value(0.0).unwrap(), "marker.png");
        assert!((*bb.scale.get_value(0.0).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_czml_model() {
        let json = r#"[
            {"id": "document", "name": "Models"},
            {"id": "model-1", "position": {"cartographicDegrees": [-75.0, 40.0, 0.0]},
             "model": {"gltf": "model.glb", "scale": 10.0}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("model-1").unwrap();
        assert!(entity.model.is_some());
        let model = entity.model.as_ref().unwrap();
        assert_eq!(model.uri.get_value(0.0).unwrap(), "model.glb");
    }

    #[test]
    fn test_parse_czml_box() {
        let json = r#"[
            {"id": "document", "name": "Boxes"},
            {"id": "box-1", "position": {"cartographicDegrees": [-75.0, 40.0, 0.0]},
             "box": {"dimensions": {"cartesian3": [100.0, 200.0, 300.0]},
                      "material": {"solidColor": {"color": {"rgba": [255, 0, 0, 255]}}}}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("box-1").unwrap();
        assert!(entity.box_graphics.is_some());
        let bx = entity.box_graphics.as_ref().unwrap();
        let dims = bx.dimensions.get_value(0.0).unwrap();
        assert_eq!(*dims, [100.0, 200.0, 300.0]);
    }

    #[test]
    fn test_parse_czml_time_dynamic_position() {
        let json = r#"[
            {"id": "document", "name": "Dynamic"},
            {"id": "sat-1", "position": {"cartographicDegrees": [0, -75.0, 40.0, 100.0, 60, -74.0, 41.0, 200.0]}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("sat-1").unwrap();
        // Should be sampled (time-tagged)
        match &entity.position {
            Property::Sampled(samples) => {
                assert_eq!(samples.len(), 2);
                assert!((samples[0].0 - 0.0).abs() < 1e-10);
                assert!((samples[1].0 - 60.0).abs() < 1e-10);
            }
            _ => panic!("Expected sampled position"),
        }
    }

    #[test]
    fn test_parse_czml_cylinder() {
        let json = r#"[
            {"id": "document", "name": "Cylinders"},
            {"id": "cyl-1", "position": {"cartographicDegrees": [-75.0, 40.0, 0.0]},
             "cylinder": {"length": 500.0, "topRadius": 50.0, "bottomRadius": 100.0}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("cyl-1").unwrap();
        assert!(entity.cylinder.is_some());
        let cyl = entity.cylinder.as_ref().unwrap();
        assert!((*cyl.length.get_value(0.0).unwrap() - 500.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_czml_path() {
        let json = r#"[
            {"id": "document", "name": "Paths"},
            {"id": "path-1", "path": {"leadTime": 3600, "trailTime": 7200, "width": 3.0}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("path-1").unwrap();
        assert!(entity.path.is_some());
        let path = entity.path.as_ref().unwrap();
        assert!((*path.lead_time.get_value(0.0).unwrap() - 3600.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_czml_label_enhanced() {
        let json = r#"[
            {"id": "document", "name": "Labels"},
            {"id": "label-1", "label": {"text": "Hello", "font": "16px monospace",
             "fillColor": {"rgba": [255, 255, 0, 255]}}}
        ]"#;

        let ds = parse_czml(json).unwrap();
        let entity = ds.entities.get("label-1").unwrap();
        assert!(entity.label.is_some());
        let label = entity.label.as_ref().unwrap();
        assert_eq!(label.text.get_value(0.0).unwrap(), "Hello");
        assert_eq!(label.font.get_value(0.0).unwrap(), "16px monospace");
    }
}
