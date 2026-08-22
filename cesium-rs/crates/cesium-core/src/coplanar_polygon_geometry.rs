//! Ported from `packages/engine/Source/Core/CoplanarPolygonGeometry.js`.
//! NOTE: Requires `CoplanarPolygonGeometryLibrary` for `create_geometry`.

use crate::cartesian3::Cartesian3;
use crate::vertex_format::VertexFormat;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CoplanarPolygonGeometry {
    positions: Vec<Cartesian3>,
    vertex_format: VertexFormat,
}

impl CoplanarPolygonGeometry {
    pub fn new(positions: Vec<Cartesian3>, vertex_format: Option<VertexFormat>) -> Self {
        Self { positions, vertex_format: vertex_format.unwrap_or_default() }
    }
    // TODO: create_geometry
}
