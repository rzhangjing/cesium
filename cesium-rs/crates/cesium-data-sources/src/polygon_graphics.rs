//! Ported from `packages/engine/Source/DataSources/PolygonGraphics.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// Graphics properties for a polygon.
#[derive(Clone)]
pub struct PolygonGraphics {
    /// Whether this polygon is shown.
    pub show: bool,
    /// The polygon hierarchy (outer ring + holes).
    pub hierarchy: Vec<Cartesian3>,
    /// The height above the ellipsoid.
    pub height: Option<f64>,
    /// The extruded height.
    pub extruded_height: Option<f64>,
    /// The material color.
    pub material_color: Color,
    /// Whether to show the outline.
    pub outline: bool,
    /// The outline color.
    pub outline_color: Color,
    /// Whether to extrude the polygon to the ground.
    pub extrude: bool,
    /// Whether to fill the polygon.
    pub fill: bool,
}

impl PolygonGraphics {
    /// Creates a new polygon graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            hierarchy: Vec::new(),
            height: None,
            extruded_height: None,
            material_color: Color::new(1.0, 1.0, 1.0, 1.0),
            outline: false,
            outline_color: Color::new(0.0, 0.0, 0.0, 1.0),
            extrude: false,
            fill: true,
        }
    }
}

impl Default for PolygonGraphics {
    fn default() -> Self { Self::new() }
}
