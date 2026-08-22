//! Ported from `packages/engine/Source/Core/EllipseGeometry.js`.
//! NOTE: Requires `EllipseGeometryLibrary` for `create_geometry`.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::vertex_format::VertexFormat;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EllipseGeometry {
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    ellipsoid: Ellipsoid,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    vertex_format: VertexFormat,
    st_rotation: f64,
}

impl EllipseGeometry {
    pub fn new(center: Cartesian3, semi_major_axis: f64, semi_minor_axis: f64, ellipsoid: Option<Ellipsoid>, height: Option<f64>, extruded_height: Option<f64>, granularity: Option<f64>, vertex_format: Option<VertexFormat>, st_rotation: Option<f64>) -> Self {
        Self { center, semi_major_axis, semi_minor_axis, ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()), height: height.unwrap_or(0.0), extruded_height: extruded_height.unwrap_or(0.0), granularity: granularity.unwrap_or(0.017453292519943295), vertex_format: vertex_format.unwrap_or_default(), st_rotation: st_rotation.unwrap_or(0.0) }
    }
    // TODO: create_geometry
}
