//! Ported from `packages/engine/Source/DataSources/PolylineGraphics.js`.

use cesium_core::arc_type::ArcType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// Graphics properties for a polyline.
#[derive(Clone)]
pub struct PolylineGraphics {
    /// Whether this polyline is shown.
    pub show: bool,
    /// The positions of the polyline vertices.
    pub positions: Vec<Cartesian3>,
    /// The width in pixels.
    pub width: f64,
    /// The material color.
    pub material_color: Color,
    /// Whether to clamp the polyline to the ground.
    pub clamp_to_ground: bool,
    /// Whether the polyline forms a closed loop.
    pub loop_: bool,
    /// The type of arc used to connect the positions.
    pub arc_type: ArcType,
    /// The sampling granularity (mirrors `granularity`).
    pub granularity: Option<f64>,
    /// The draw order for ground polylines (mirrors `zIndex`).
    pub z_index: Option<f64>,
    /// The depth-fail material color (mirrors `depthFailMaterial` as a
    /// color material; `None` mirrors an unset depth-fail material).
    pub depth_fail_material_color: Option<Color>,
}

impl PolylineGraphics {
    /// Creates a new polyline graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            positions: Vec::new(),
            width: 1.0,
            material_color: Color::new(1.0, 1.0, 1.0, 1.0),
            clamp_to_ground: false,
            loop_: false,
            arc_type: ArcType::Geodesic,
            granularity: None,
            z_index: None,
            depth_fail_material_color: None,
        }
    }
}

impl Default for PolylineGraphics {
    fn default() -> Self { Self::new() }
}
