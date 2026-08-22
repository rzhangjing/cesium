//! Ported from `packages/engine/Source/Core/PolygonGeometry.js`.
//! NOTE: Requires `PolygonGeometryLibrary` for `create_geometry`.

use crate::cartesian3::Cartesian3;
use crate::vertex_format::VertexFormat;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PolygonGeometry {
    positions: Vec<Cartesian3>,
    height: f64,
    extruded_height: f64,
    vertex_format: VertexFormat,
    granularity: f64,
}

impl PolygonGeometry {
    pub fn new(positions: Vec<Cartesian3>, height: Option<f64>, extruded_height: Option<f64>, vertex_format: Option<VertexFormat>, granularity: Option<f64>) -> Self {
        Self { positions, height: height.unwrap_or(0.0), extruded_height: extruded_height.unwrap_or(0.0), vertex_format: vertex_format.unwrap_or_default(), granularity: granularity.unwrap_or(0.017453292519943295) }
    }
    // TODO: create_geometry
}
