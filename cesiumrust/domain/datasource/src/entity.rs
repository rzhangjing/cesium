//! Entity definition and graphics properties.
//!
//! Maps to CesiumJS `DataSources/Entity.js` and graphics types
//! (PointGraphics, PolylineGraphics, PolygonGraphics, etc.)

use crate::property::{BoolProperty, Color, ColorProperty, NumberProperty, PositionProperty, Property, StringProperty};

/// Point graphics properties.
///
/// Maps to CesiumJS `DataSources/PointGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PointGraphics {
    /// Point color.
    pub color: ColorProperty,
    /// Point pixel size.
    pub pixel_size: NumberProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width in pixels.
    pub outline_width: NumberProperty,
    /// Whether the point is shown.
    pub show: BoolProperty,
}

impl Default for PointGraphics {
    fn default() -> Self {
        Self {
            color: Property::Constant(Color::WHITE),
            pixel_size: Property::Constant(1.0),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(0.0),
            show: Property::Constant(true),
        }
    }
}

/// Polyline graphics properties.
///
/// Maps to CesiumJS `DataSources/PolylineGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineGraphics {
    /// Polyline positions (array of [lon, lat, height]).
    pub positions: Property<Vec<[f64; 3]>>,
    /// Line width in pixels.
    pub width: NumberProperty,
    /// Line color.
    pub color: ColorProperty,
    /// Whether the polyline is shown.
    pub show: BoolProperty,
    /// Whether to clamp to ground.
    pub clamp_to_ground: BoolProperty,
}

impl Default for PolylineGraphics {
    fn default() -> Self {
        Self {
            positions: Property::Undefined,
            width: Property::Constant(1.0),
            color: Property::Constant(Color::WHITE),
            show: Property::Constant(true),
            clamp_to_ground: Property::Constant(false),
        }
    }
}

/// Polygon graphics properties.
///
/// Maps to CesiumJS `DataSources/PolygonGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonGraphics {
    /// Polygon hierarchy positions (exterior ring).
    pub positions: Property<Vec<[f64; 3]>>,
    /// Holes (interior rings).
    pub holes: Vec<Vec<[f64; 3]>>,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the polygon is filled.
    pub fill: BoolProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Height of the polygon.
    pub height: NumberProperty,
    /// Extruded height.
    pub extruded_height: NumberProperty,
    /// Whether the polygon is shown.
    pub show: BoolProperty,
}

impl Default for PolygonGraphics {
    fn default() -> Self {
        Self {
            positions: Property::Undefined,
            holes: Vec::new(),
            material: Property::Constant(Color::WHITE),
            fill: Property::Constant(true),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            height: Property::Undefined,
            extruded_height: Property::Undefined,
            show: Property::Constant(true),
        }
    }
}

/// Billboard graphics properties.
///
/// Maps to CesiumJS `DataSources/BillboardGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct BillboardGraphics {
    /// Image URI.
    pub image: StringProperty,
    /// Width in pixels.
    pub width: NumberProperty,
    /// Height in pixels.
    pub height: NumberProperty,
    /// Color tint.
    pub color: ColorProperty,
    /// Rotation in radians.
    pub rotation: NumberProperty,
    /// Scale factor.
    pub scale: NumberProperty,
    /// Whether the billboard is shown.
    pub show: BoolProperty,
}

impl Default for BillboardGraphics {
    fn default() -> Self {
        Self {
            image: Property::Undefined,
            width: Property::Undefined,
            height: Property::Undefined,
            color: Property::Constant(Color::WHITE),
            rotation: Property::Constant(0.0),
            scale: Property::Constant(1.0),
            show: Property::Constant(true),
        }
    }
}

/// Label graphics properties.
///
/// Maps to CesiumJS `DataSources/LabelGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct LabelGraphics {
    /// Label text.
    pub text: StringProperty,
    /// Font (CSS format).
    pub font: StringProperty,
    /// Fill color.
    pub fill_color: ColorProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Whether the label is shown.
    pub show: BoolProperty,
}

impl Default for LabelGraphics {
    fn default() -> Self {
        Self {
            text: Property::Undefined,
            font: Property::Constant("30px sans-serif".to_string()),
            fill_color: Property::Constant(Color::WHITE),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(2.0),
            show: Property::Constant(true),
        }
    }
}

/// Model graphics properties.
///
/// Maps to CesiumJS `DataSources/ModelGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ModelGraphics {
    /// Model URI (glTF/glb).
    pub uri: StringProperty,
    /// Scale factor.
    pub scale: NumberProperty,
    /// Minimum pixel size.
    pub minimum_pixel_size: NumberProperty,
    /// Whether the model is shown.
    pub show: BoolProperty,
}

impl Default for ModelGraphics {
    fn default() -> Self {
        Self {
            uri: Property::Undefined,
            scale: Property::Constant(1.0),
            minimum_pixel_size: Property::Constant(0.0),
            show: Property::Constant(true),
        }
    }
}

/// Ellipse graphics properties.
///
/// Maps to CesiumJS `DataSources/EllipseGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct EllipseGraphics {
    /// Semi-major axis in meters.
    pub semi_major_axis: NumberProperty,
    /// Semi-minor axis in meters.
    pub semi_minor_axis: NumberProperty,
    /// Rotation in radians.
    pub rotation: NumberProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Height in meters.
    pub height: NumberProperty,
    /// Extruded height.
    pub extruded_height: NumberProperty,
    /// Whether the ellipse is shown.
    pub show: BoolProperty,
}

impl Default for EllipseGraphics {
    fn default() -> Self {
        Self {
            semi_major_axis: Property::Undefined,
            semi_minor_axis: Property::Undefined,
            rotation: Property::Constant(0.0),
            material: Property::Constant(Color::WHITE),
            height: Property::Constant(0.0),
            extruded_height: Property::Undefined,
            show: Property::Constant(true),
        }
    }
}

/// An entity in the data source.
///
/// Maps to CesiumJS `DataSources/Entity.js`
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Unique identifier.
    pub id: String,

    /// Human-readable name.
    pub name: Option<String>,

    /// Whether the entity is shown.
    pub show: bool,

    /// Entity description (HTML).
    pub description: Option<String>,

    /// Position property [longitude_rad, latitude_rad, height_m].
    pub position: PositionProperty,

    /// Orientation (quaternion [x, y, z, w]).
    pub orientation: Property<[f64; 4]>,

    /// Point graphics.
    pub point: Option<PointGraphics>,

    /// Polyline graphics.
    pub polyline: Option<PolylineGraphics>,

    /// Polygon graphics.
    pub polygon: Option<PolygonGraphics>,

    /// Billboard graphics.
    pub billboard: Option<BillboardGraphics>,

    /// Label graphics.
    pub label: Option<LabelGraphics>,

    /// Model graphics.
    pub model: Option<ModelGraphics>,

    /// Ellipse graphics.
    pub ellipse: Option<EllipseGraphics>,

    /// Parent entity ID.
    pub parent: Option<String>,

    /// Custom properties (key-value metadata).
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

impl Entity {
    /// Creates a new entity with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            show: true,
            description: None,
            position: Property::Undefined,
            orientation: Property::Undefined,
            point: None,
            polyline: None,
            polygon: None,
            billboard: None,
            label: None,
            model: None,
            ellipse: None,
            parent: None,
            properties: std::collections::HashMap::new(),
        }
    }

    /// Sets the name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the position as a constant [lon_rad, lat_rad, height_m].
    pub fn with_position(mut self, lon: f64, lat: f64, height: f64) -> Self {
        self.position = Property::Constant([lon, lat, height]);
        self
    }

    /// Sets point graphics.
    pub fn with_point(mut self, point: PointGraphics) -> Self {
        self.point = Some(point);
        self
    }

    /// Sets polyline graphics.
    pub fn with_polyline(mut self, polyline: PolylineGraphics) -> Self {
        self.polyline = Some(polyline);
        self
    }

    /// Sets polygon graphics.
    pub fn with_polygon(mut self, polygon: PolygonGraphics) -> Self {
        self.polygon = Some(polygon);
        self
    }

    /// Sets billboard graphics.
    pub fn with_billboard(mut self, billboard: BillboardGraphics) -> Self {
        self.billboard = Some(billboard);
        self
    }

    /// Sets label graphics.
    pub fn with_label(mut self, label: LabelGraphics) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets model graphics.
    pub fn with_model(mut self, model: ModelGraphics) -> Self {
        self.model = Some(model);
        self
    }

    /// Adds a custom property.
    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    /// Returns true if this entity has any renderable graphics.
    pub fn has_graphics(&self) -> bool {
        self.point.is_some()
            || self.polyline.is_some()
            || self.polygon.is_some()
            || self.billboard.is_some()
            || self.label.is_some()
            || self.model.is_some()
            || self.ellipse.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new("test-1").with_name("Test Entity");
        assert_eq!(entity.id, "test-1");
        assert_eq!(entity.name, Some("Test Entity".to_string()));
        assert!(entity.show);
        assert!(!entity.has_graphics());
    }

    #[test]
    fn test_entity_with_point() {
        let entity = Entity::new("point-1")
            .with_position(0.1, 0.2, 100.0)
            .with_point(PointGraphics {
                color: Property::Constant(Color::RED),
                pixel_size: Property::Constant(10.0),
                ..Default::default()
            });

        assert!(entity.has_graphics());
        let point = entity.point.unwrap();
        let color = point.color.get_value(0.0).unwrap();
        assert!((color.red - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_entity_with_polyline() {
        let entity = Entity::new("line-1").with_polyline(PolylineGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.1, 0.1, 0.0],
                [0.2, 0.0, 0.0],
            ]),
            width: Property::Constant(3.0),
            color: Property::Constant(Color::BLUE),
            ..Default::default()
        });

        assert!(entity.has_graphics());
        let polyline = entity.polyline.unwrap();
        let positions = polyline.positions.get_value(0.0).unwrap();
        assert_eq!(positions.len(), 3);
    }

    #[test]
    fn test_entity_with_polygon() {
        let entity = Entity::new("poly-1").with_polygon(PolygonGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [0.1, 0.1, 0.0],
                [0.0, 0.1, 0.0],
            ]),
            material: Property::Constant(Color::new(1.0, 0.0, 0.0, 0.5)),
            ..Default::default()
        });

        assert!(entity.has_graphics());
        let polygon = entity.polygon.unwrap();
        let material = polygon.material.get_value(0.0).unwrap();
        assert!((material.alpha - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_entity_custom_properties() {
        let entity = Entity::new("prop-1")
            .with_property("population", serde_json::json!(1000000))
            .with_property("name", serde_json::json!("City"));

        assert_eq!(entity.properties.len(), 2);
        assert_eq!(entity.properties["population"], serde_json::json!(1000000));
    }
}
