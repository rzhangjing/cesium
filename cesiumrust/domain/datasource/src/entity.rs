//! Entity definition and graphics properties.
//!
//! Maps to CesiumJS `DataSources/Entity.js` and graphics types
//! (PointGraphics, PolylineGraphics, PolygonGraphics, etc.)

use crate::property::{BoolProperty, Color, ColorProperty, NumberProperty, PositionProperty, Property, StringProperty};

/// Height reference for positioning relative to terrain.
///
/// Maps to CesiumJS `Scene/HeightReference.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeightReference {
    /// Position is absolute (no terrain adjustment).
    #[default]
    None,
    /// Position is clamped to the terrain surface.
    ClampToGround,
    /// Position height is relative to the terrain surface.
    RelativeToGround,
    /// Position is clamped to the most detailed 3D Tiles surface.
    ClampToTileset,
    /// Position height is relative to the most detailed 3D Tiles surface.
    RelativeToTileset,
}

/// Corner style for corridors and polyline volumes.
///
/// Maps to CesiumJS `Core/CornerType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CornerType {
    /// Rounded corners.
    #[default]
    Rounded,
    /// Mitered (sharp) corners.
    Mitered,
    /// Beveled (cut) corners.
    Beveled,
}

/// Classification type for ground primitives.
///
/// Maps to CesiumJS `Scene/ClassificationType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClassificationType {
    /// Classify both terrain and 3D Tiles.
    #[default]
    Both,
    /// Classify terrain only.
    Terrain,
    /// Classify 3D Tiles only.
    Cesium3DTile,
}

/// Shadow mode for an entity.
///
/// Maps to CesiumJS `Scene/ShadowMode.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowMode {
    /// Shadows are disabled.
    #[default]
    Disabled,
    /// Casts shadows only.
    CastOnly,
    /// Receives shadows only.
    ReceiveOnly,
    /// Casts and receives shadows.
    Enabled,
}

/// A plane defined by a normal and distance from origin.
///
/// Maps to CesiumJS `Core/Plane.js`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneDef {
    /// Plane normal [x, y, z].
    pub normal: [f64; 3],
    /// Signed distance from origin.
    pub distance: f64,
}

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
    /// Whether the ellipse is filled.
    pub fill: BoolProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
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
            fill: Property::Constant(true),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            show: Property::Constant(true),
        }
    }
}

/// Box graphics properties.
///
/// Maps to CesiumJS `DataSources/BoxGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct BoxGraphics {
    /// Box dimensions [width, depth, height] in meters.
    pub dimensions: Property<[f64; 3]>,
    /// Height reference.
    pub height_reference: HeightReference,
    /// Whether the box is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the box is shown.
    pub show: BoolProperty,
}

impl Default for BoxGraphics {
    fn default() -> Self {
        Self {
            dimensions: Property::Undefined,
            height_reference: HeightReference::None,
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
            show: Property::Constant(true),
        }
    }
}

/// Cylinder graphics properties.
///
/// Maps to CesiumJS `DataSources/CylinderGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct CylinderGraphics {
    /// Length (height) in meters.
    pub length: NumberProperty,
    /// Top radius in meters.
    pub top_radius: NumberProperty,
    /// Bottom radius in meters.
    pub bottom_radius: NumberProperty,
    /// Height reference.
    pub height_reference: HeightReference,
    /// Number of vertical lines for the outline.
    pub number_of_vertical_lines: NumberProperty,
    /// Number of slices (radial segments).
    pub slices: NumberProperty,
    /// Whether the cylinder is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the cylinder is shown.
    pub show: BoolProperty,
}

impl Default for CylinderGraphics {
    fn default() -> Self {
        Self {
            length: Property::Undefined,
            top_radius: Property::Undefined,
            bottom_radius: Property::Undefined,
            height_reference: HeightReference::None,
            number_of_vertical_lines: Property::Constant(16.0),
            slices: Property::Constant(128.0),
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
            show: Property::Constant(true),
        }
    }
}

/// Corridor graphics properties.
///
/// Maps to CesiumJS `DataSources/CorridorGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct CorridorGraphics {
    /// Corridor center-line positions (array of [lon, lat, height]).
    pub positions: Property<Vec<[f64; 3]>>,
    /// Corridor width in meters.
    pub width: NumberProperty,
    /// Height of the corridor.
    pub height: NumberProperty,
    /// Height reference.
    pub height_reference: HeightReference,
    /// Extruded height.
    pub extruded_height: NumberProperty,
    /// Corner type.
    pub corner_type: CornerType,
    /// Angular granularity in radians.
    pub granularity: NumberProperty,
    /// Whether the corridor is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Classification type.
    pub classification_type: ClassificationType,
    /// Z-index for ground corridor ordering.
    pub z_index: NumberProperty,
    /// Whether the corridor is shown.
    pub show: BoolProperty,
}

impl Default for CorridorGraphics {
    fn default() -> Self {
        Self {
            positions: Property::Undefined,
            width: Property::Undefined,
            height: Property::Constant(0.0),
            height_reference: HeightReference::None,
            extruded_height: Property::Undefined,
            corner_type: CornerType::Rounded,
            granularity: Property::Undefined,
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
            classification_type: ClassificationType::Both,
            z_index: Property::Constant(0.0),
            show: Property::Constant(true),
        }
    }
}

/// Rectangle graphics properties.
///
/// Maps to CesiumJS `DataSources/RectangleGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct RectangleGraphics {
    /// Rectangle coordinates [west, south, east, north] in radians.
    pub coordinates: Property<[f64; 4]>,
    /// Height in meters.
    pub height: NumberProperty,
    /// Height reference.
    pub height_reference: HeightReference,
    /// Extruded height.
    pub extruded_height: NumberProperty,
    /// Rotation of the rectangle in radians.
    pub rotation: NumberProperty,
    /// Texture coordinate rotation in radians.
    pub st_rotation: NumberProperty,
    /// Angular granularity in radians.
    pub granularity: NumberProperty,
    /// Whether the rectangle is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Classification type.
    pub classification_type: ClassificationType,
    /// Z-index for ground rectangle ordering.
    pub z_index: NumberProperty,
    /// Whether the rectangle is shown.
    pub show: BoolProperty,
}

impl Default for RectangleGraphics {
    fn default() -> Self {
        Self {
            coordinates: Property::Undefined,
            height: Property::Constant(0.0),
            height_reference: HeightReference::None,
            extruded_height: Property::Undefined,
            rotation: Property::Constant(0.0),
            st_rotation: Property::Constant(0.0),
            granularity: Property::Undefined,
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
            classification_type: ClassificationType::Both,
            z_index: Property::Constant(0.0),
            show: Property::Constant(true),
        }
    }
}

/// Wall graphics properties.
///
/// Maps to CesiumJS `DataSources/WallGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct WallGraphics {
    /// Wall positions (array of [lon, lat, height]).
    pub positions: Property<Vec<[f64; 3]>>,
    /// Minimum heights for each position.
    pub minimum_heights: Property<Vec<f64>>,
    /// Maximum heights for each position.
    pub maximum_heights: Property<Vec<f64>>,
    /// Angular granularity in radians.
    pub granularity: NumberProperty,
    /// Whether the wall is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the wall is shown.
    pub show: BoolProperty,
}

impl Default for WallGraphics {
    fn default() -> Self {
        Self {
            positions: Property::Undefined,
            minimum_heights: Property::Undefined,
            maximum_heights: Property::Undefined,
            granularity: Property::Undefined,
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
            show: Property::Constant(true),
        }
    }
}

/// Ellipsoid graphics properties.
///
/// Maps to CesiumJS `DataSources/EllipsoidGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct EllipsoidGraphics {
    /// Outer radii [x, y, z] in meters.
    pub radii: Property<[f64; 3]>,
    /// Inner radii for hollow ellipsoid.
    pub inner_radii: Property<[f64; 3]>,
    /// Minimum clock angle in radians.
    pub minimum_clock: NumberProperty,
    /// Maximum clock angle in radians.
    pub maximum_clock: NumberProperty,
    /// Minimum cone angle in radians.
    pub minimum_cone: NumberProperty,
    /// Maximum cone angle in radians.
    pub maximum_cone: NumberProperty,
    /// Height reference.
    pub height_reference: HeightReference,
    /// Number of radial slices.
    pub slices: NumberProperty,
    /// Number of stack partitions.
    pub stack_partitions: NumberProperty,
    /// Number of slice partitions.
    pub slice_partitions: NumberProperty,
    /// Number of subdivisions.
    pub subdivisions: NumberProperty,
    /// Whether the ellipsoid is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the ellipsoid is shown.
    pub show: BoolProperty,
}

impl Default for EllipsoidGraphics {
    fn default() -> Self {
        Self {
            radii: Property::Undefined,
            inner_radii: Property::Undefined,
            minimum_clock: Property::Constant(0.0),
            maximum_clock: Property::Constant(std::f64::consts::TAU),
            minimum_cone: Property::Constant(0.0),
            maximum_cone: Property::Constant(std::f64::consts::PI),
            height_reference: HeightReference::None,
            slices: Property::Constant(128.0),
            stack_partitions: Property::Constant(64.0),
            slice_partitions: Property::Constant(64.0),
            subdivisions: Property::Constant(128.0),
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
            show: Property::Constant(true),
        }
    }
}

/// Plane graphics properties.
///
/// Maps to CesiumJS `DataSources/PlaneGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PlaneGraphics {
    /// Plane definition (normal + distance).
    pub plane: Property<PlaneDef>,
    /// Dimensions [width, height] in meters.
    pub dimensions: Property<[f64; 2]>,
    /// Whether the plane is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the plane is shown.
    pub show: BoolProperty,
}

impl Default for PlaneGraphics {
    fn default() -> Self {
        Self {
            plane: Property::Undefined,
            dimensions: Property::Undefined,
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
            show: Property::Constant(true),
        }
    }
}

/// Path graphics properties (trail visualization).
///
/// Maps to CesiumJS `DataSources/PathGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PathGraphics {
    /// Lead time in seconds (how far ahead to show).
    pub lead_time: NumberProperty,
    /// Trail time in seconds (how far behind to show).
    pub trail_time: NumberProperty,
    /// Path width in pixels.
    pub width: NumberProperty,
    /// Sampling resolution in seconds.
    pub resolution: NumberProperty,
    /// Path material color.
    pub material: ColorProperty,
    /// Whether the path is shown.
    pub show: BoolProperty,
}

impl Default for PathGraphics {
    fn default() -> Self {
        Self {
            lead_time: Property::Undefined,
            trail_time: Property::Undefined,
            width: Property::Constant(1.0),
            resolution: Property::Constant(60.0),
            material: Property::Constant(Color::WHITE),
            show: Property::Constant(true),
        }
    }
}

/// Polyline volume graphics properties.
///
/// Maps to CesiumJS `DataSources/PolylineVolumeGraphics.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineVolumeGraphics {
    /// Volume center-line positions (array of [lon, lat, height]).
    pub positions: Property<Vec<[f64; 3]>>,
    /// 2D cross-section shape (array of [x, y] in meters).
    pub shape: Property<Vec<[f64; 2]>>,
    /// Corner type.
    pub corner_type: CornerType,
    /// Angular granularity in radians.
    pub granularity: NumberProperty,
    /// Whether the volume is filled.
    pub fill: BoolProperty,
    /// Fill material color.
    pub material: ColorProperty,
    /// Whether the outline is shown.
    pub outline: BoolProperty,
    /// Outline color.
    pub outline_color: ColorProperty,
    /// Outline width.
    pub outline_width: NumberProperty,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the volume is shown.
    pub show: BoolProperty,
}

impl Default for PolylineVolumeGraphics {
    fn default() -> Self {
        Self {
            positions: Property::Undefined,
            shape: Property::Undefined,
            corner_type: CornerType::Rounded,
            granularity: Property::Undefined,
            fill: Property::Constant(true),
            material: Property::Constant(Color::WHITE),
            outline: Property::Constant(false),
            outline_color: Property::Constant(Color::BLACK),
            outline_width: Property::Constant(1.0),
            shadows: ShadowMode::Disabled,
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

    /// Box graphics.
    pub box_graphics: Option<BoxGraphics>,

    /// Cylinder graphics.
    pub cylinder: Option<CylinderGraphics>,

    /// Corridor graphics.
    pub corridor: Option<CorridorGraphics>,

    /// Rectangle graphics.
    pub rectangle: Option<RectangleGraphics>,

    /// Wall graphics.
    pub wall: Option<WallGraphics>,

    /// Ellipsoid graphics.
    pub ellipsoid: Option<EllipsoidGraphics>,

    /// Plane graphics.
    pub plane: Option<PlaneGraphics>,

    /// Path graphics.
    pub path: Option<PathGraphics>,

    /// Polyline volume graphics.
    pub polyline_volume: Option<PolylineVolumeGraphics>,

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
            box_graphics: None,
            cylinder: None,
            corridor: None,
            rectangle: None,
            wall: None,
            ellipsoid: None,
            plane: None,
            path: None,
            polyline_volume: None,
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

    /// Sets box graphics.
    pub fn with_box(mut self, box_graphics: BoxGraphics) -> Self {
        self.box_graphics = Some(box_graphics);
        self
    }

    /// Sets cylinder graphics.
    pub fn with_cylinder(mut self, cylinder: CylinderGraphics) -> Self {
        self.cylinder = Some(cylinder);
        self
    }

    /// Sets corridor graphics.
    pub fn with_corridor(mut self, corridor: CorridorGraphics) -> Self {
        self.corridor = Some(corridor);
        self
    }

    /// Sets rectangle graphics.
    pub fn with_rectangle(mut self, rectangle: RectangleGraphics) -> Self {
        self.rectangle = Some(rectangle);
        self
    }

    /// Sets wall graphics.
    pub fn with_wall(mut self, wall: WallGraphics) -> Self {
        self.wall = Some(wall);
        self
    }

    /// Sets ellipsoid graphics.
    pub fn with_ellipsoid(mut self, ellipsoid: EllipsoidGraphics) -> Self {
        self.ellipsoid = Some(ellipsoid);
        self
    }

    /// Sets plane graphics.
    pub fn with_plane(mut self, plane: PlaneGraphics) -> Self {
        self.plane = Some(plane);
        self
    }

    /// Sets path graphics.
    pub fn with_path(mut self, path: PathGraphics) -> Self {
        self.path = Some(path);
        self
    }

    /// Sets polyline volume graphics.
    pub fn with_polyline_volume(mut self, polyline_volume: PolylineVolumeGraphics) -> Self {
        self.polyline_volume = Some(polyline_volume);
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
            || self.box_graphics.is_some()
            || self.cylinder.is_some()
            || self.corridor.is_some()
            || self.rectangle.is_some()
            || self.wall.is_some()
            || self.ellipsoid.is_some()
            || self.plane.is_some()
            || self.path.is_some()
            || self.polyline_volume.is_some()
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

    #[test]
    fn test_entity_with_box() {
        let entity = Entity::new("box-1").with_box(BoxGraphics {
            dimensions: Property::Constant([100.0, 200.0, 300.0]),
            material: Property::Constant(Color::RED),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let bx = entity.box_graphics.unwrap();
        let dims = bx.dimensions.get_value(0.0).unwrap();
        assert_eq!(*dims, [100.0, 200.0, 300.0]);
    }

    #[test]
    fn test_entity_with_cylinder() {
        let entity = Entity::new("cyl-1").with_cylinder(CylinderGraphics {
            length: Property::Constant(500.0),
            top_radius: Property::Constant(50.0),
            bottom_radius: Property::Constant(100.0),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let cyl = entity.cylinder.unwrap();
        assert_eq!(*cyl.length.get_value(0.0).unwrap(), 500.0);
        assert_eq!(*cyl.top_radius.get_value(0.0).unwrap(), 50.0);
    }

    #[test]
    fn test_entity_with_corridor() {
        let entity = Entity::new("cor-1").with_corridor(CorridorGraphics {
            positions: Property::Constant(vec![[0.0, 0.0, 0.0], [0.1, 0.1, 0.0]]),
            width: Property::Constant(200.0),
            corner_type: CornerType::Beveled,
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let cor = entity.corridor.unwrap();
        assert_eq!(cor.corner_type, CornerType::Beveled);
    }

    #[test]
    fn test_entity_with_rectangle() {
        let entity = Entity::new("rect-1").with_rectangle(RectangleGraphics {
            coordinates: Property::Constant([-0.1, -0.1, 0.1, 0.1]),
            height: Property::Constant(1000.0),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let rect = entity.rectangle.unwrap();
        assert_eq!(*rect.height.get_value(0.0).unwrap(), 1000.0);
    }

    #[test]
    fn test_entity_with_wall() {
        let entity = Entity::new("wall-1").with_wall(WallGraphics {
            positions: Property::Constant(vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0]]),
            maximum_heights: Property::Constant(vec![500.0, 500.0]),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let wall = entity.wall.unwrap();
        assert_eq!(wall.maximum_heights.get_value(0.0).unwrap().len(), 2);
    }

    #[test]
    fn test_entity_with_ellipsoid() {
        let entity = Entity::new("ell-1").with_ellipsoid(EllipsoidGraphics {
            radii: Property::Constant([100.0, 200.0, 300.0]),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let ell = entity.ellipsoid.unwrap();
        assert_eq!(*ell.radii.get_value(0.0).unwrap(), [100.0, 200.0, 300.0]);
    }

    #[test]
    fn test_entity_with_plane() {
        let entity = Entity::new("plane-1").with_plane(PlaneGraphics {
            plane: Property::Constant(PlaneDef { normal: [0.0, 0.0, 1.0], distance: 0.0 }),
            dimensions: Property::Constant([500.0, 500.0]),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let pl = entity.plane.unwrap();
        assert_eq!(pl.plane.get_value(0.0).unwrap().normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_entity_with_path() {
        let entity = Entity::new("path-1").with_path(PathGraphics {
            lead_time: Property::Constant(3600.0),
            trail_time: Property::Constant(7200.0),
            width: Property::Constant(3.0),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let path = entity.path.unwrap();
        assert_eq!(*path.lead_time.get_value(0.0).unwrap(), 3600.0);
    }

    #[test]
    fn test_entity_with_polyline_volume() {
        let entity = Entity::new("pv-1").with_polyline_volume(PolylineVolumeGraphics {
            positions: Property::Constant(vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0]]),
            shape: Property::Constant(vec![[-50.0, -50.0], [50.0, -50.0], [50.0, 50.0], [-50.0, 50.0]]),
            ..Default::default()
        });
        assert!(entity.has_graphics());
        let pv = entity.polyline_volume.unwrap();
        assert_eq!(pv.shape.get_value(0.0).unwrap().len(), 4);
    }

    #[test]
    fn test_height_reference_default() {
        assert_eq!(HeightReference::default(), HeightReference::None);
        assert_eq!(CornerType::default(), CornerType::Rounded);
        assert_eq!(ClassificationType::default(), ClassificationType::Both);
        assert_eq!(ShadowMode::default(), ShadowMode::Disabled);
    }
}
