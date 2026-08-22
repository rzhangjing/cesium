//! Ported from `packages/engine/Source/Core/PolylineGeometry.js`.
//! A description of a polyline.
//! NOTE: Requires `PolylinePipeline` + `Color` for `create_geometry`.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;
use crate::simple_polyline_geometry::ArcType;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PolylineGeometry {
    positions: Vec<Cartesian3>,
    width: f64,
    colors: Option<Vec<[f64; 4]>>,
    colors_per_vertex: bool,
    arc_type: ArcType,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl PolylineGeometry {
    pub fn new(
        positions: Vec<Cartesian3>,
        width: Option<f64>,
        colors: Option<Vec<[f64; 4]>>,
        colors_per_vertex: Option<bool>,
        arc_type: Option<ArcType>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        Self {
            positions,
            width: width.unwrap_or(1.0),
            colors,
            colors_per_vertex: colors_per_vertex.unwrap_or(false),
            arc_type: arc_type.unwrap_or(ArcType::Geodesic),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
        }
    }
    // TODO: create_geometry
}
