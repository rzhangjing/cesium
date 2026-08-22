//! Ported from `packages/engine/Source/Core/RectangleGeometry.js`.
//! NOTE: Requires `RectangleGeometryLibrary` for `create_geometry`.

use crate::rectangle::Rectangle;
use crate::vertex_format::VertexFormat;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RectangleGeometry {
    rectangle: Rectangle,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    vertex_format: VertexFormat,
}

impl RectangleGeometry {
    pub fn new(rectangle: Rectangle, height: Option<f64>, extruded_height: Option<f64>, granularity: Option<f64>, vertex_format: Option<VertexFormat>) -> Self {
        Self { rectangle, height: height.unwrap_or(0.0), extruded_height: extruded_height.unwrap_or(0.0), granularity: granularity.unwrap_or(0.017453292519943295), vertex_format: vertex_format.unwrap_or_default() }
    }
    // TODO: create_geometry
}
